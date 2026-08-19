#ifndef EMANEGENERATORSEELGENERATOR_HEADER_
#define EMANEGENERATORSEELGENERATOR_HEADER_

#include "eelloaderpluginfactory.h"
#include "emane/eventgenerator.h"

#include <vector>
#include <map>
#include <string>
#include <list>
#include <cstdint>

extern "C" {
    void* emane_c_eel_generator_get_plugin(void* generator, const char* event_type);
    size_t emane_c_eel_generator_get_all_plugins(void* generator, void** plugins_out, size_t max_plugins);
    void emane_c_eel_generator_send_event(void* generator, uint16_t nem_id, uint16_t event_id, const void* data, size_t len);
}

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
        
        void* getPlugin(const std::string & eventType);
        size_t getAllPlugins(void** plugins_out, size_t max_plugins);

        void addPlugin(const std::string& sEventType, LoaderPlugin* pPlugin, EventPublishMode mode);
        void addPluginFactory(LoaderPluginFactory* pPluginFactory);

      private:
        using PluginFactoryList = std::list<LoaderPluginFactory *>;
        using EventPluginMap = std::map<std::string, std::pair<LoaderPlugin *,EventPublishMode>>;

        void* rs_state_;
        PluginFactoryList pluginFactoryList_;
        EventPluginMap eventPluginMap_;
      };
    }
  }
}

#endif // EMANEGENERATORSEELGENERATOR_HEADER_
