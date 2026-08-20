#ifndef EMANEGENERATORSEELLOADERPATHLOSSEX_HEADER_
#define EMANEGENERATORSEELLOADERPATHLOSSEX_HEADER_

#include "emane/generators/eel/loaderplugin.h"

namespace EMANE
{
  namespace Generators
  {
    namespace EEL
    {
      class LoaderPathlossEx : public LoaderPlugin
      {
      public:
        LoaderPathlossEx();

        ~LoaderPathlossEx();

        void load(const ModuleType & modelType,
                  const ModuleId   & moduleId,
                  const EventType  & eventType,
                  const InputArguments & args) override;

        EventInfoList getEvents(EventPublishMode mode) override;

      private:
        void* rust_ptr_;
      };
    }
  }
}

#endif // EMANEGENERATORSEELLOADERPATHLOSSEX_HEADER_
