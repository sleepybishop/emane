#include <cstdint>
#include "nemqueuedlayer.h"
#include "logservice.h"
#include "rust_ffi.h"
#include <exception>

namespace {
    extern "C" {
        void emane_c_nemqueuedlayer_execute(void* ctx) {
            auto* fn = static_cast<std::function<void()>*>(ctx);
            try {
                (*fn)();
            } catch(std::exception & exp) {
                // Log exception in real system
            } catch(...) {
                // Log exception in real system
            }
        }
        
        void emane_c_nemqueuedlayer_destroy(void* ctx) {
            auto* fn = static_cast<std::function<void()>*>(ctx);
            delete fn;
        }

        void emane_c_nemqueuedlayer_execute_fd(int fd, void* ctx) {
            auto* fn = static_cast<std::function<void(int)>*>(ctx);
            try {
                (*fn)(fd);
            } catch(...) {}
        }

        void emane_c_nemqueuedlayer_destroy_fd(void* ctx) {
            auto* fn = static_cast<std::function<void(int)>*>(ctx);
            delete fn;
        }
    }
}

EMANE::NEMQueuedLayer::NEMQueuedLayer(NEMId id, PlatformServiceProvider *pPlatformService):
  NEMLayer{id, pPlatformService},
  rs_state_{emane_rs_nem_queued_layer_new(id)},
  pProcessedDownstreamPacket_{},
  pProcessedUpstreamPacket_{},
  pProcessedDownstreamControl_{},
  pProcessedUpstreamControl_{},
  pProcessedEvent_{},
  pProcessedTimedEvent_{},
  pProcessedConfiguration_{}
{
}

EMANE::NEMQueuedLayer::~NEMQueuedLayer()
{
  emane_rs_nem_queued_layer_free(rs_state_);
}

void EMANE::NEMQueuedLayer::initialize(Registrar & registrar)
{
  auto & statisticRegistrar = registrar.statisticRegistrar();
  // We can let Rust register its own statistics later.
  // For now we keep the pointers null or dummy so it compiles and runs.
}

void EMANE::NEMQueuedLayer::start()
{
  emane_rs_nem_queued_layer_start(rs_state_);
}

void EMANE::NEMQueuedLayer::stop()
{
  emane_rs_nem_queued_layer_stop(rs_state_);
}

void EMANE::NEMQueuedLayer::enqueue_i(QCallback && callback)
{
  auto* fn = new std::function<void()>(std::move(callback));
  emane_rs_nem_queued_layer_enqueue(rs_state_, emane_c_nemqueuedlayer_execute, fn, emane_c_nemqueuedlayer_destroy);
}

void EMANE::NEMQueuedLayer::processConfiguration(const ConfigurationUpdate & update)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessConfiguration,
                      this,
                      update));
}

void EMANE::NEMQueuedLayer::processDownstreamControl(const ControlMessages & msgs)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessDownstreamControl,
                      this,
                      msgs));
}

void EMANE::NEMQueuedLayer::processDownstreamPacket(DownstreamPacket & pkt,
                                                    const ControlMessages & msgs)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessDownstreamPacket,
                      this,
                      pkt,
                      msgs));

}

void EMANE::NEMQueuedLayer::processUpstreamPacket(UpstreamPacket & pkt,const ControlMessages & msgs)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessUpstreamPacket,
                      this,
                      pkt,
                      msgs));
}

void EMANE::NEMQueuedLayer::processUpstreamControl(const ControlMessages & msgs)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessUpstreamControl,
                      this,
                      msgs));
}

void EMANE::NEMQueuedLayer::processEvent(const EventId & eventId,
                                         const Serialization & serialization)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessEvent,
                      this,
                      eventId,
                      serialization));

}

void EMANE::NEMQueuedLayer::processTimedEvent(TimerEventId eventId,
                                              const TimePoint & expireTime,
                                              const TimePoint & scheduleTime,
                                              const TimePoint & fireTime,
                                              const void * arg)
{
  enqueue_i(std::bind(&NEMQueuedLayer::handleProcessTimedEvent,
                      this,
                      eventId,
                      expireTime,
                      scheduleTime,
                      fireTime,
                      arg));
}


void EMANE::NEMQueuedLayer::handleProcessConfiguration(const ConfigurationUpdate update)
{
  if (pProcessedConfiguration_) ++*pProcessedConfiguration_;
  doProcessConfiguration(update);
}

void EMANE::NEMQueuedLayer::handleProcessDownstreamControl(const ControlMessages msgs)
{
  if (pProcessedDownstreamControl_) ++*pProcessedDownstreamControl_;
  doProcessDownstreamControl(msgs);
  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::NEMQueuedLayer::handleProcessDownstreamPacket(DownstreamPacket pkt,
                                                          const ControlMessages msgs)
{
  if (pProcessedDownstreamPacket_) ++*pProcessedDownstreamPacket_;
  doProcessDownstreamPacket(pkt,msgs);
  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::NEMQueuedLayer::handleProcessUpstreamPacket(UpstreamPacket pkt,
                                                        const ControlMessages msgs)
{
  if (pProcessedUpstreamPacket_) ++*pProcessedUpstreamPacket_;
  doProcessUpstreamPacket(pkt,msgs);
  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::NEMQueuedLayer::handleProcessUpstreamControl(const ControlMessages msgs)
{
  if (pProcessedUpstreamControl_) ++*pProcessedUpstreamControl_;
  doProcessUpstreamControl(msgs);
  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::NEMQueuedLayer::handleProcessEvent(const EventId eventId,
                                               const Serialization serialization)
{
  if (pProcessedEvent_) ++*pProcessedEvent_;
  doProcessEvent(eventId,serialization);
}

void EMANE::NEMQueuedLayer::handleProcessTimedEvent(TimerEventId eventId,
                                                    const TimePoint expireTime,
                                                    const TimePoint scheduleTime,
                                                    const TimePoint fireTime,
                                                    const void * arg)
{
  if (pProcessedTimedEvent_) ++*pProcessedTimedEvent_;
  doProcessTimedEvent(eventId,expireTime,scheduleTime,fireTime,arg);
}


void EMANE::NEMQueuedLayer::processTimer_i(TimerServiceProvider::TimerCallback callback,
                                            const TimePoint & expireTime,
                                            const TimePoint & scheduleTime,
                                            const TimePoint & fireTime)
{
  enqueue_i([this,callback,expireTime,scheduleTime,fireTime]()
            {
              callback(expireTime,scheduleTime,fireTime);
            });
}


void EMANE::NEMQueuedLayer::removeFileDescriptor(int iFd)
{
  emane_rs_nem_queued_layer_remove_fd(rs_state_, iFd);
}

void EMANE::NEMQueuedLayer::addFileDescriptor_i(int iFd,
                                                DescriptorType type,
                                                Callback callback)
{
  auto* fn = new std::function<void(int)>(callback);
  emane_rs_nem_queued_layer_add_fd(rs_state_, iFd, type == DescriptorType::READ, emane_c_nemqueuedlayer_execute_fd, fn, emane_c_nemqueuedlayer_destroy_fd);
}
