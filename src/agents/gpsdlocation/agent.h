
#ifndef EMANEAGENTSGPSDLOCATIONAGENT_HEADER_
#define EMANEAGENTSGPSDLOCATIONAGENT_HEADER_

#include "emane/eventagent.h"
#include "emane/types.h"
#include <string>

namespace EMANE {
  namespace Agents {
    namespace GPSDLocation {
      class Agent : public EventAgent {
      public:
        Agent(NEMId nemId, PlatformServiceProvider *pPlatformService);
        ~Agent();

        void initialize(Registrar & registrar) override;
        void configure(const ConfigurationUpdate & update) override;
        void start() override;
        void stop() override;
        void destroy() throw() override;
        void processEvent(const EventId&, const Serialization &) override;
        void processTimedEvent(TimerEventId eventId,
                               const TimePoint & expireTime,
                               const TimePoint & scheduleTime,
                               const TimePoint & fireTime,
                               const void * arg) override;

      private:
        NEMId nemId_;
        std::string sPseudoTerminalFile_;
        std::string sPseudoTerminalNameFile_;
        TimerEventId timerId_;
        void* rs_agent_;
      };
    }
  }
}
#endif
