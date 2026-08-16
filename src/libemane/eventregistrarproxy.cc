#include "eventregistrarproxy.h"
#include "emane/registrarexception.h"

extern "C" {
    bool emane_rs_event_service_register_event(uint16_t build_id, uint16_t event_id);
}

EMANE::EventRegistrarProxy::EventRegistrarProxy(BuildId buildId):
  buildId_{buildId}{}

void EMANE::EventRegistrarProxy::registerEvent(EventId eventId)
{
    if (!emane_rs_event_service_register_event(buildId_, eventId)) {
        throw RegistrarException{"Component not eligible to register for events"};
    }
}
