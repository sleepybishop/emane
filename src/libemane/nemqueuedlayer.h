#ifndef EMANENEMQUEUEDLAYER_HEADER_
#define EMANENEMQUEUEDLAYER_HEADER_

#include "emane/nemlayer.h"
#include "emane/filedescriptorserviceprovider.h"
#include "emane/timerserviceprovider.h"

#include <functional>
#include <mutex>

namespace EMANE
{
  class NEMQueuedLayer :  public NEMLayer,
                          public FileDescriptorServiceProvider
  {
  public:
    ~NEMQueuedLayer();

    void initialize(Registrar & registrar) override;

    void start() override;

    void stop() override;

    void processConfiguration(const ConfigurationUpdate & update) override;

    void processDownstreamControl(const ControlMessages & msgs) override;

    void processDownstreamPacket(DownstreamPacket &pkt, const ControlMessages & msgs) override;

    void processUpstreamPacket(UpstreamPacket &pkt, const ControlMessages & msgs) override;

    void processUpstreamControl(const ControlMessages & msgs) override;

    void processEvent(const EventId &eventId, const Serialization &serialization) override;

    void processTimedEvent(TimerEventId eventId,
                           const TimePoint & expireTime,
                           const TimePoint & scheduleTime,
                           const TimePoint & fireTime,
                           const void * arg) override;

    template <typename Function>
    void processTimer(Function fn,
                      const TimePoint & expireTime,
                      const TimePoint & scheduleTime,
                      const TimePoint & fireTime);

  protected:
    NEMQueuedLayer(NEMId id, PlatformServiceProvider * pPlatformService);

    virtual void doProcessConfiguration(const ConfigurationUpdate &) = 0;

    virtual void doProcessDownstreamControl(const ControlMessages &) = 0;

    virtual void doProcessDownstreamPacket(DownstreamPacket &, const ControlMessages &) = 0;

    virtual void doProcessUpstreamPacket(UpstreamPacket &, const ControlMessages &) = 0;

    virtual void doProcessUpstreamControl(const ControlMessages &) = 0;

    virtual void doProcessEvent(const EventId &, const Serialization &) = 0;

    virtual void doProcessTimedEvent(TimerEventId eventId,
                                     const TimePoint & expireTime,
                                     const TimePoint & scheduleTime,
                                     const TimePoint & fireTime,
                                     const void * arg) = 0;

  private:
    using QCallback = std::function<void()>;
    void * rs_state_;

    StatisticNumeric<std::uint64_t> * pProcessedDownstreamPacket_;
    StatisticNumeric<std::uint64_t> * pProcessedUpstreamPacket_;
    StatisticNumeric<std::uint64_t> * pProcessedDownstreamControl_;
    StatisticNumeric<std::uint64_t> * pProcessedUpstreamControl_;
    StatisticNumeric<std::uint64_t> * pProcessedEvent_;
    StatisticNumeric<std::uint64_t> * pProcessedTimedEvent_;
    StatisticNumeric<std::uint64_t> * pProcessedConfiguration_;

    NEMQueuedLayer(const NEMQueuedLayer &) = delete;
    NEMQueuedLayer & operator=(const NEMQueuedLayer &) = delete;

    void handleProcessConfiguration(const ConfigurationUpdate update);
    void handleProcessDownstreamControl(const ControlMessages msgs);
    void handleProcessDownstreamPacket(DownstreamPacket pkt, const ControlMessages msgs);
    void handleProcessUpstreamPacket(UpstreamPacket pkt, const ControlMessages msgs);
    void handleProcessUpstreamControl(const ControlMessages msgs);
    void handleProcessEvent(const EventId eventId, const Serialization serialization);
    void handleProcessTimedEvent(TimerEventId eventId,
                                 const TimePoint expireTime,
                                 const TimePoint scheduleTime,
                                 const TimePoint fireTime,
                                 const void * arg);

    void removeFileDescriptor(int iFd) override;
    void addFileDescriptor_i(int iFd, DescriptorType type, Callback callback) override;
    void enqueue_i(QCallback && callback);
    void processTimer_i(TimerServiceProvider::TimerCallback callback,
                        const TimePoint & expireTime,
                        const TimePoint & scheduleTime,
                        const TimePoint & fireTime);
  };
}

#include "nemqueuedlayer.inl"

#endif //EMANENEMQUEUEDLAYER_HEADER_
