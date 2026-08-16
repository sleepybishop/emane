#include "nemmanagerimpl.h"
#include "logservice.h"
#include "otamanager.h"
#include "timerservice.h"
#include "eventservice.h"
#include "eventserviceexception.h"
#include "emane/configureexception.h"
#include "emane/platformexception.h"
#include "emane/startexception.h"
#include "otaexception.h"
#include "antennaprofilemanifest.h"
#include "spectralmaskmanager.h"

extern "C" {
    void* emane_rs_nem_manager_create(const std::uint8_t* uuid_ptr);
    void emane_rs_nem_manager_destroy_manager(void* manager_ptr);
    void emane_rs_nem_manager_add(void* manager_ptr, std::uint16_t nem_id, void* nem_ptr);
    void emane_rs_nem_manager_set_config_str(void* manager_ptr, const char* key, const char* value);
    void emane_rs_nem_manager_set_config_u8(void* manager_ptr, const char* key, std::uint8_t value);
    void emane_rs_nem_manager_set_config_u16(void* manager_ptr, const char* key, std::uint16_t value);
    void emane_rs_nem_manager_set_config_u32(void* manager_ptr, const char* key, std::uint32_t value);
    void emane_rs_nem_manager_set_config_bool(void* manager_ptr, const char* key, bool value);
    void emane_rs_nem_manager_apply_config(void* manager_ptr);
    void emane_rs_nem_manager_start(void* manager_ptr);
    void emane_rs_nem_manager_post_start(void* manager_ptr);
    void emane_rs_nem_manager_stop(void* manager_ptr);
    void emane_rs_nem_manager_destroy(void* manager_ptr);
}

EMANE::Application::NEMManagerImpl* EMANE::Application::NEMManagerImpl::pInstance_ = nullptr;

EMANE::Application::NEMManagerImpl::NEMManagerImpl(const uuid_t & uuid):
  NEMManager{uuid}
{
  pRsNemManager_ = emane_rs_nem_manager_create(uuid);
  pInstance_ = this;
}

EMANE::Application::NEMManagerImpl::~NEMManagerImpl()
{
  if(pRsNemManager_) {
    emane_rs_nem_manager_destroy_manager(pRsNemManager_);
    pRsNemManager_ = nullptr;
  }
  pInstance_ = nullptr;
}

void EMANE::Application::NEMManagerImpl::add(std::unique_ptr<Application::NEM> & pNEM)
{
  emane_rs_nem_manager_add(pRsNemManager_, pNEM->getNEMId(), pNEM.release());
}

void EMANE::Application::NEMManagerImpl::initialize(Registrar & registrar)
{
  auto & configRegistrar = registrar.configurationRegistrar();

  configRegistrar.registerNonNumeric<INETAddr>("eventservicegroup",
                                               ConfigurationProperties::REQUIRED,
                                               {},
                                               "IPv4 or IPv6 Event Service channel multicast endpoint.");

  configRegistrar.registerNonNumeric<std::string>("eventservicedevice",
                                                  ConfigurationProperties::NONE,
                                                  {},
                                                  "Device to associate with the Event Service channel multicast endpoint.");

  configRegistrar.registerNumeric<std::uint8_t>("eventservicettl",
                                                ConfigurationProperties::DEFAULT,
                                                {1},
                                                "Device to associate with the Event Service channel multicast endpoint.");

  configRegistrar.registerNonNumeric<INETAddr>("otamanagergroup",
                                               ConfigurationProperties::NONE,
                                               {},
                                               "IPv4 or IPv6 Event Service OTA channel endpoint.");

  configRegistrar.registerNonNumeric<std::string>("otamanagerdevice",
                                                  ConfigurationProperties::NONE,
                                                  {},
                                                  "Device to associate with the OTA channel multicast endpoint.");

  configRegistrar.registerNumeric<std::uint32_t>("otamanagermtu",
                                                 ConfigurationProperties::DEFAULT,
                                                 {0},
                                                 "OTA channel MTU.");

  configRegistrar.registerNumeric<std::uint16_t>("otamanagerpartcheckthreshold",
                                                 ConfigurationProperties::DEFAULT,
                                                 {2},
                                                 "Defines the rate in seconds a check is performed to see if any OTA packet"
                                                 " part reassembly efforts should be abandoned.");

  configRegistrar.registerNumeric<std::uint16_t>("otamanagerparttimeoutthreshold",
                                                 ConfigurationProperties::DEFAULT,
                                                 {5},
                                                 "Defines the threshold in seconds to wait for another OTA packet part"
                                                 " for an existing reassembly effort before abandoning the effort.");

  configRegistrar.registerNumeric<std::uint8_t>("otamanagerttl",
                                                ConfigurationProperties::DEFAULT,
                                                {1},
                                                "OTA channel multicast message TTL.");

  configRegistrar.registerNumeric<bool>("otamanagerloopback",
                                        ConfigurationProperties::DEFAULT,
                                        {false},
                                        "Enable multicast loopback on the OTA channel multicast channel.");

  configRegistrar.registerNumeric<bool>("otamanagerchannelenable",
                                        ConfigurationProperties::DEFAULT,
                                        {true},
                                        "Enable OTA channel multicast communication.");


  configRegistrar.registerNonNumeric<INETAddr>("controlportendpoint",
                                               ConfigurationProperties::REQUIRED,
                                               {INETAddr{"0.0.0.0",47000 }},
                                               "IPv4 or IPv6 control port endpoint.");

  configRegistrar.registerNonNumeric<std::string>("antennaprofilemanifesturi",
                                                  EMANE::ConfigurationProperties::NONE,
                                                  {},
                                                  "URI of the antenna profile manifest to load."
                                                  " The antenna profile manifest contains a list of"
                                                  " antenna profile entries. Each entry contains a unique"
                                                  " profile identifier, an antenna pattern URI and an"
                                                  " antenna blockage URI. This parameter is required when"
                                                  " antennaprofileenable is on or if any other NEM"
                                                  " participating in the emulation has antennaprofileenable"
                                                  " set on, even in the case where antennaprofileenable is"
                                                  " off locally.");

  configRegistrar.registerNumeric<std::uint32_t>("stats.ota.maxpacketcountrows",
                                                 ConfigurationProperties::DEFAULT,
                                                 {0},
                                                 "OTA channel max packet count table rows.");

  configRegistrar.registerNumeric<std::uint32_t>("stats.ota.maxeventcountrows",
                                                 ConfigurationProperties::DEFAULT,
                                                 {0},
                                                 "OTA channel max event count table rows.");

  configRegistrar.registerNumeric<std::uint32_t>("stats.event.maxeventcountrows",
                                                 ConfigurationProperties::DEFAULT,
                                                 {0},
                                                 "Event channel max event count table rows.");

  configRegistrar.registerNonNumeric<std::string>("spectralmaskmanifesturi",
                                                  EMANE::ConfigurationProperties::NONE,
                                                  {},
                                                  "URI of the RF transmit spectral mask manifest to load."
                                                  " The spectral mask manifest contains a list of"
                                                  " spectral masks. Each spectral mask contains a unique"
                                                  " mask identifier, a primary signal definition and zero"
                                                  " or more spur definitions. This parameter is required when"
                                                  " any NEM participating in the emulation is using spectral"
                                                  " masks, even in the case where the local NEM is not.");

}

void EMANE::Application::NEMManagerImpl::configure(const ConfigurationUpdate & update)
{
  for(const auto & item : update)
    {
      if(item.first == "otamanagergroup")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asINETAddr().str().c_str());
        }
      else if(item.first == "otamanagerdevice")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asString().c_str());
        }
      else if(item.first == "otamanagerttl")
        {
          emane_rs_nem_manager_set_config_u8(pRsNemManager_, item.first.c_str(), item.second[0].asUINT8());
        }
      else if(item.first == "otamanagermtu")
        {
          emane_rs_nem_manager_set_config_u32(pRsNemManager_, item.first.c_str(), item.second[0].asUINT32());
        }
      else if(item.first == "otamanagerpartcheckthreshold")
        {
          emane_rs_nem_manager_set_config_u16(pRsNemManager_, item.first.c_str(), item.second[0].asUINT16());
        }
      else if(item.first == "otamanagerparttimeoutthreshold")
        {
          emane_rs_nem_manager_set_config_u16(pRsNemManager_, item.first.c_str(), item.second[0].asUINT16());
        }
      else if(item.first == "otamanagerloopback")
        {
          emane_rs_nem_manager_set_config_bool(pRsNemManager_, item.first.c_str(), item.second[0].asBool());
        }
      else if(item.first == "otamanagerchannelenable")
        {
          emane_rs_nem_manager_set_config_bool(pRsNemManager_, item.first.c_str(), item.second[0].asBool());
        }
      else if(item.first == "eventservicegroup")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asINETAddr().str().c_str());
        }
      else if(item.first == "eventservicedevice")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asString().c_str());
        }
      else if(item.first == "eventservicettl")
        {
          emane_rs_nem_manager_set_config_u8(pRsNemManager_, item.first.c_str(), item.second[0].asUINT8());
        }
      else if(item.first == "controlportendpoint")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asINETAddr().str().c_str());
        }
      else if(item.first == "antennaprofilemanifesturi")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asString().c_str());
        }
      else if(item.first == "stats.ota.maxpacketcountrows")
        {
          OTAManagerSingleton::instance()->setStatPacketCountRowLimit(item.second[0].asUINT32());
        }
      else if(item.first == "stats.ota.maxeventcountrows")
        {
          OTAManagerSingleton::instance()->setStatEventCountRowLimit(item.second[0].asUINT32());
        }
      else if(item.first == "stats.event.maxeventcountrows")
        {
          EventServiceSingleton::instance()->setStatEventCountRowLimit(item.second[0].asUINT32());
        }
      else if(item.first == "spectralmaskmanifesturi")
        {
          emane_rs_nem_manager_set_config_str(pRsNemManager_, item.first.c_str(), item.second[0].asString().c_str());
        }
      else
        {
          throw makeException<ConfigureException>("NEMManagerImpl: Unexpected configuration item %s", item.first.c_str());
        }
    }

  emane_rs_nem_manager_apply_config(pRsNemManager_);
}

void EMANE::Application::NEMManagerImpl::start()
{
  emane_rs_nem_manager_start(pRsNemManager_);
}

void EMANE::Application::NEMManagerImpl::postStart()
{
  emane_rs_nem_manager_post_start(pRsNemManager_);
}

void EMANE::Application::NEMManagerImpl::stop()
{
  emane_rs_nem_manager_stop(pRsNemManager_);
}

void EMANE::Application::NEMManagerImpl::destroy() throw()
{
  emane_rs_nem_manager_destroy(pRsNemManager_);
}

// C callbacks implementation
extern "C" {
    void emane_c_nem_start(void* nem_ptr) {
        if(nem_ptr) static_cast<EMANE::Application::NEM*>(nem_ptr)->start();
    }
    
    void emane_c_nem_post_start(void* nem_ptr) {
        if(nem_ptr) static_cast<EMANE::Application::NEM*>(nem_ptr)->postStart();
    }
    
    void emane_c_nem_stop(void* nem_ptr) {
        if(nem_ptr) static_cast<EMANE::Application::NEM*>(nem_ptr)->stop();
    }
    
    void emane_c_nem_destroy(void* nem_ptr) {
        if(nem_ptr) {
            auto nem = static_cast<EMANE::Application::NEM*>(nem_ptr);
            nem->destroy();
            delete nem;
        }
    }
    
    void emane_c_control_port_open(const char* port_str) {
        if(EMANE::Application::NEMManagerImpl::instance() && port_str) {
            EMANE::INETAddr addr{port_str};
            EMANE::Application::NEMManagerImpl::instance()->getControlPortService().open(addr);
        }
    }
    
    void emane_c_control_port_close() {
        if(EMANE::Application::NEMManagerImpl::instance()) {
            EMANE::Application::NEMManagerImpl::instance()->getControlPortService().close();
        }
    }
    
    void emane_c_load_antenna_profile(const char* uri) {
        if(uri) EMANE::AntennaProfileManifest::instance()->load(uri);
    }
    
    void emane_c_load_spectral_mask(const char* uri) {
        if(uri) EMANE::SpectralMaskManager::instance()->load(uri);
    }
    
    void emane_c_ota_manager_open(
        const char* addr,
        const char* device,
        bool loopback,
        std::uint8_t ttl,
        const std::uint8_t* uuid,
        std::uint32_t mtu,
        std::uint16_t part_check_thresh,
        std::uint16_t part_timeout_thresh
    ) {
        try {
            EMANE::INETAddr inetAddr{addr};
            uuid_t u;
            std::copy(uuid, uuid + 16, std::begin(u));
            EMANE::OTAManagerSingleton::instance()->open(
                inetAddr,
                device ? device : "",
                loopback,
                ttl,
                u,
                mtu,
                EMANE::Seconds{part_check_thresh},
                EMANE::Seconds{part_timeout_thresh}
            );
        } catch(EMANE::OTAException & exp) {
            throw EMANE::StartException(exp.what());
        }
    }
    
    void emane_c_event_service_open(
        const char* addr,
        const char* device,
        std::uint8_t ttl,
        const std::uint8_t* uuid
    ) {
        try {
            EMANE::INETAddr inetAddr{addr};
            uuid_t u;
            std::copy(uuid, uuid + 16, std::begin(u));
            EMANE::EventServiceSingleton::instance()->open(
                inetAddr,
                device ? device : "",
                ttl,
                true,
                u
            );
        } catch(EMANE::EventServiceException & e) {
            throw EMANE::StartException(e.what());
        }
    }
}
