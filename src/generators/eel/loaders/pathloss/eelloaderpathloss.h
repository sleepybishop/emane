#ifndef EMANEGENERATORSEELLOADERPATHLOSS_HEADER_
#define EMANEGENERATORSEELLOADERPATHLOSS_HEADER_

#include "emane/generators/eel/loaderplugin.h"

namespace EMANE
{
  namespace Generators
  {
    namespace EEL
    {
      class LoaderPathloss : public LoaderPlugin
      {
      public:
        LoaderPathloss();
        
        ~LoaderPathloss();
        
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

#endif // EMANEGENERATORSEELLOADERPATHLOSS_HEADER_
