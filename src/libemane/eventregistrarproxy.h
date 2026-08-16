#ifndef EMANEEVENTREGISTRARPROXY_HEADER_
#define EMANEEVENTREGISTRARPROXY_HEADER_

#include "emane/eventregistrar.h"
#include "emane/types.h"

namespace EMANE
{
  class EventRegistrarProxy : public EventRegistrar
  {
  public:
    EventRegistrarProxy(BuildId buildId);

    void registerEvent(EventId eventId) override;

  private:
    BuildId buildId_;
  };
}

#endif //EMANEEVENTREGISTRARPROXY_HEADER_
