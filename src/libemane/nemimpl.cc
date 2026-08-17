#include <cstdint>
#include "nemimpl.h"
#include "logservice.h"
#include "emane/configureexception.h"
#include "emane/startexception.h"

extern "C" {
    void* emane_rs_nem_impl_create(std::uint16_t id, void* stack, bool b_ext, void* ota, void* net);
    void emane_rs_nem_impl_destroy(void* ptr);
    void emane_rs_nem_impl_start(void* ptr);
    void emane_rs_nem_impl_post_start(void* ptr);
    void emane_rs_nem_impl_stop(void* ptr);
    void emane_rs_nem_impl_destroy_layers(void* ptr);

    void emane_c_nem_adapter_open_ota(void* adapter) {
        if(adapter) static_cast<EMANE::NEMOTAAdapter*>(adapter)->open();
    }
    void emane_c_nem_adapter_close_ota(void* adapter) {
        if(adapter) static_cast<EMANE::NEMOTAAdapter*>(adapter)->close();
    }
    // We let C++ NEMImpl handle the network adapter opening because it requires C++ config parameters
    void emane_c_nem_adapter_open_net(void*) {}
    void emane_c_nem_adapter_close_net(void*) {}
}

EMANE::Application::NEMImpl::NEMImpl(NEMId id,
                                     std::unique_ptr<NEMLayerStack> & pNEMLayerStack,
                                     bool bExternalTransport):
  pNEMLayerStack_(std::move(pNEMLayerStack)),
  id_{id},
  bExternalTransport_{bExternalTransport},
  pNEMOTAAdapter_{new NEMOTAAdapter{id}},
  pNEMNetworkAdapter_{new NEMNetworkAdapter{id}}
{
  pNEMLayerStack_->connectLayers(pNEMNetworkAdapter_.get(), pNEMOTAAdapter_.get());
  
  pRsNemImpl_ = emane_rs_nem_impl_create(id_, pNEMLayerStack_->getRsNemLayerStack(), bExternalTransport_, pNEMOTAAdapter_.get(), pNEMNetworkAdapter_.get());
}

EMANE::Application::NEMImpl::~NEMImpl()
{
  if(pRsNemImpl_) {
      emane_rs_nem_impl_destroy(pRsNemImpl_);
      pRsNemImpl_ = nullptr;
  }
}

void EMANE::Application::NEMImpl::initialize(Registrar & registrar)
{
  if(bExternalTransport_)
    {
      auto & configRegistrar = registrar.configurationRegistrar();

      configRegistrar.registerNonNumeric<INETAddr>("platformendpoint",
                                                   ConfigurationProperties::REQUIRED,
                                                   {},
                                                   "IPv4 or IPv6 NEM Platform Service endpoint.");

      configRegistrar.registerNonNumeric<INETAddr>("transportendpoint",
                                                   ConfigurationProperties::REQUIRED,
                                                   {},
                                                   "IPv4 or IPv6 Transport endpoint.");

      configRegistrar.registerNonNumeric<std::string>("protocol",
                                                      EMANE::ConfigurationProperties::DEFAULT,
                                                      {"udp"},
                                                      "Defines the protocl used for communictation:"
                                                      " udp or tcp.",
                                                      1,
                                                      1,
                                                      "^(udp|tcp)$");
    }

  pNEMLayerStack_->initialize(registrar);
}

void EMANE::Application::NEMImpl::configure(const ConfigurationUpdate & update)
{
  for(const auto & item : update)
    {
      if(item.first == "platformendpoint")
        {
          platformEndpointAddr_ = item.second[0].asINETAddr();

          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  INFO_LEVEL,
                                  "NEM  %03hu NEMImpl::configure platformendpoint: %s",
                                  id_,
                                  platformEndpointAddr_.str().c_str());

        }
      else if(item.first == "transportendpoint")
        {
          transportEndpointAddr_ = item.second[0].asINETAddr();

          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  INFO_LEVEL,
                                  "NEM  %03hu NEMImpl::configure transportendpoint: %s",
                                  id_,
                                  transportEndpointAddr_.str().c_str());
        }
      else if(item.first == "protocol")
        {
          std::string sProtocol{item.second[0].asString()};

          protocol_ = sProtocol == "udp" ?
            NEMNetworkAdapter::Protocol::PROTOCOL_UDP :
            NEMNetworkAdapter::Protocol::PROTOCOL_TCP_SERVER;

          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  INFO_LEVEL,
                                  "NEM  %03hu NEMImpl::configure %s: %s",
                                  id_,
                                  item.first.c_str(),
                                  sProtocol.c_str());
        }
      else
        {
          throw makeException<ConfigureException>("NEMImpl::configure: "
                                                  "Unexpected configuration item %s",
                                                  item.first.c_str());
        }
    }
}

void EMANE::Application::NEMImpl::start()
{
  if(bExternalTransport_)
    {
      try
        {
          pNEMNetworkAdapter_->open(platformEndpointAddr_,
                                    transportEndpointAddr_,
                                    protocol_);
        }
      catch(NetworkAdapterException & exp)
        {
          throw StartException(exp.what());
        }
    }

  emane_rs_nem_impl_start(pRsNemImpl_);
}

void EMANE::Application::NEMImpl::postStart()
{
  emane_rs_nem_impl_post_start(pRsNemImpl_);
}

void EMANE::Application::NEMImpl::stop()
{
  if(bExternalTransport_)
    {
      pNEMNetworkAdapter_->close();
    }
    
  emane_rs_nem_impl_stop(pRsNemImpl_);
}

void EMANE::Application::NEMImpl::destroy()
  throw()
{
  emane_rs_nem_impl_destroy_layers(pRsNemImpl_);
}

EMANE::NEMId EMANE::Application::NEMImpl::getNEMId() const
{
  return id_;
}
