
#include "agent.h"
#include "emane/configureexception.h"
#include "emane/events/locationevent.h"
#include <iostream>
#include <fstream>
#include <cstdint>
#include <algorithm>

extern "C" {
    void* emane_rs_gpsd_agent_new(uint16_t nem_id);
    void emane_rs_gpsd_agent_free(void* ptr);
    void emane_rs_gpsd_agent_start(void* ptr, const char* pty_file);
    void emane_rs_gpsd_agent_stop(void* ptr);
    void emane_rs_gpsd_agent_update_location(void* ptr, double lat, double lon, double alt, bool has_velocity, double azm, double mag);
    void emane_rs_gpsd_agent_process_timed_event(void* ptr);
}

EMANE::Agents::GPSDLocation::Agent::Agent(NEMId nemId, PlatformServiceProvider* pPlatformService):
  EventAgent{nemId, pPlatformService},
  nemId_{nemId},
  rs_agent_{emane_rs_gpsd_agent_new(nemId)}
{}

EMANE::Agents::GPSDLocation::Agent::~Agent() {
    emane_rs_gpsd_agent_free(rs_agent_);
}

void EMANE::Agents::GPSDLocation::Agent::initialize(Registrar & registrar) {
    auto & configRegistrar = registrar.configurationRegistrar();
    configRegistrar.registerNonNumeric<std::string>("pseudoterminalfile",
                                                    ConfigurationProperties::NONE,
                                                    {},
                                                    "Pseudo terminal filename");
    configRegistrar.registerNonNumeric<std::string>("pseudoterminalnamefile",
                                                    ConfigurationProperties::NONE,
                                                    {},
                                                    "Pseudo terminal device filename");
}

void EMANE::Agents::GPSDLocation::Agent::configure(const ConfigurationUpdate & update) {
    for(const auto & item : update) {
        if(item.first == "pseudoterminalfile") {
            sPseudoTerminalFile_ = item.second[0].asString();
        } else if(item.first == "pseudoterminalnamefile") {
            sPseudoTerminalNameFile_ = item.second[0].asString();
        } else {
            throw ConfigureException("GPSDLocationAgent: Unexpected configuration item.");
        }
    }
}

void EMANE::Agents::GPSDLocation::Agent::start() {
    emane_rs_gpsd_agent_start(rs_agent_, sPseudoTerminalFile_.c_str());
    
    timerId_ = pPlatformService_->timerService().scheduleTimedEvent(
        Clock::now() + std::chrono::seconds(1),
        nullptr,
        std::chrono::seconds(1)
    );
}

void EMANE::Agents::GPSDLocation::Agent::stop() {
    emane_rs_gpsd_agent_stop(rs_agent_);
    pPlatformService_->timerService().cancelTimedEvent(timerId_);
}

void EMANE::Agents::GPSDLocation::Agent::destroy() throw() {}

void EMANE::Agents::GPSDLocation::Agent::processEvent(const EventId & eventId, const Serialization & serialization) {
    if(eventId == Events::LocationEvent::IDENTIFIER) {
        Events::LocationEvent event{serialization};
        const auto & locations = event.getLocations();
        auto iter = std::find_if(locations.begin(), locations.end(), [this](const Events::Location & p) {
            return p.getNEMId() == nemId_;
        });
        
        if(iter != locations.end()) {
            const auto & position = iter->getPosition();
            auto velocity = iter->getVelocity();
            bool has_velocity = velocity.second;
            double azm = 0.0;
            double mag = 0.0;
            if(has_velocity) {
                azm = velocity.first.getAzimuthDegrees();
                mag = velocity.first.getMagnitudeMetersPerSecond();
            }
            emane_rs_gpsd_agent_update_location(rs_agent_, 
                position.getLatitudeDegrees(), 
                position.getLongitudeDegrees(), 
                position.getAltitudeMeters(), 
                has_velocity, azm, mag);
        }
    }
}

void EMANE::Agents::GPSDLocation::Agent::processTimedEvent(TimerEventId, const TimePoint &, const TimePoint &, const TimePoint &, const void *) {
    emane_rs_gpsd_agent_process_timed_event(rs_agent_);
}

DECLARE_EVENT_AGENT(EMANE::Agents::GPSDLocation::Agent);
