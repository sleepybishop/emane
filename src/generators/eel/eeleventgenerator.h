#ifndef EMANEGENERATORSEELGENERATOR_HEADER_
#define EMANEGENERATORSEELGENERATOR_HEADER_

#include "eelloaderpluginfactory.h"
#include "emane/eventgenerator.h"

#include <string>

namespace EMANE
{
  namespace Generators
  {
    namespace EEL
    {
      class Generator : public EventGenerator
      {
      public:
        Generator(PlatformServiceProvider *pPlatformService);
        ~Generator();

        void initialize(Registrar & registrar) override;
        void configure(const ConfigurationUpdate & update) override;
        void start() override;
        void stop() override;
        void destroy() throw() override;
        void sendEvent(NEMId nemId, EventId eventId, const Serialization & serialization);

      private:
        void* rs_state_;
      };
    }
  }
}

#endif // EMANEGENERATORSEELGENERATOR_HEADER_
