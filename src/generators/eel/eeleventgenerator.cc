#include "eeleventgenerator.h"
#include "emane/generators/eel/formatexception.h"
#include "emane/configureexception.h"
#include "emane/utils/parameterconvert.h"

extern "C" {
    void* emane_rs_eel_generator_new(void* c_generator);
    void emane_rs_eel_generator_free(void* ptr);
    void emane_rs_eel_generator_add_input_file(void* ptr, const char* filename);
    void emane_rs_eel_generator_start(void* ptr);
    void emane_rs_eel_generator_stop(void* ptr);
    void emane_rs_eel_generator_destroy(void* ptr);
    
    void* emane_c_eel_generator_get_plugin(void* generator, const char* event_type) {
        auto* gen = static_cast<EMANE::Generators::EEL::Generator*>(generator);
        return gen->getPlugin(event_type);
    }
    
    size_t emane_c_eel_generator_get_all_plugins(void* generator, void** plugins_out, size_t max_plugins) {
        auto* gen = static_cast<EMANE::Generators::EEL::Generator*>(generator);
        return gen->getAllPlugins(plugins_out, max_plugins);
    }
    
    void emane_c_eel_generator_send_event(void* generator, uint16_t nem_id, uint16_t event_id, const void* data, size_t len) {
        auto* gen = static_cast<EMANE::Generators::EEL::Generator*>(generator);
        EMANE::Serialization serialization(static_cast<const char*>(data), len);
        gen->sendEvent(nem_id, event_id, serialization);
    }
    
    void emane_c_eel_plugin_load(void* plugin_ptr, const char* module_type, uint16_t module_id, const char* event_type, const char* const* args, size_t num_args) {
        auto* entry = static_cast<std::pair<EMANE::Generators::EEL::LoaderPlugin*, EMANE::Generators::EEL::EventPublishMode>*>(plugin_ptr);
        if(!entry || !entry->first) return;
        
        EMANE::Generators::EEL::InputArguments input_args;
        for(size_t i = 0; i < num_args; ++i) {
            input_args.push_back(args[i]);
        }
        
        entry->first->load(module_type, module_id, event_type, input_args);
    }
    
    void emane_c_eel_plugin_get_events(void* plugin_ptr, void (*callback)(uint16_t, uint16_t, const void*, size_t, void*), void* user_data) {
        auto* entry = static_cast<std::pair<EMANE::Generators::EEL::LoaderPlugin*, EMANE::Generators::EEL::EventPublishMode>*>(plugin_ptr);
        if(!entry || !entry->first) return;
        
        EMANE::Generators::EEL::EventInfoList eventList = entry->first->getEvents(entry->second);
        for(auto& event : eventList) {
            const auto& serialization = event.getSerialization();
            callback(event.getNEMId(), event.getEventId(), serialization.c_str(), serialization.length(), user_data);
        }
    }
}

EMANE::Generators::EEL::Generator::Generator(PlatformServiceProvider *pPlatformService):
  EventGenerator(pPlatformService),
  rs_state_{emane_rs_eel_generator_new(this)}
{}

EMANE::Generators::EEL::Generator::~Generator() {
    emane_rs_eel_generator_free(rs_state_);
    for(auto p : pluginFactoryList_) {
        delete p;
    }
}

void EMANE::Generators::EEL::Generator::initialize(Registrar & registrar) {
    auto & configRegistrar = registrar.configurationRegistrar();

    configRegistrar.registerNonNumeric<std::string>("inputfile",
                                                    ConfigurationProperties::REQUIRED,
                                                    {},
                                                    "EEL Text input file.",
                                                    1,
                                                    1024);

    configRegistrar.registerNonNumeric<std::string>("loader",
                                                    ConfigurationProperties::REQUIRED,
                                                    {},
                                                    "EEL Loader plugin.",
                                                    1,
                                                    1024);
}

void EMANE::Generators::EEL::Generator::configure(const ConfigurationUpdate & update) {
    for(const auto & item : update) {
        if(item.first == "inputfile") {
            for(const auto & any : item.second) {
                std::string sInputFile = any.asString();
                emane_rs_eel_generator_add_input_file(rs_state_, sInputFile.c_str());
            }
        }
        else if(item.first == "loader") {
            try {
                for(const auto & any : item.second) {
                    std::string sLoaderPlugin = any.asString();
                    size_t pos = sLoaderPlugin.find(':');
                    if(pos != std::string::npos) {
                        std::string sEventTypes = sLoaderPlugin.substr(0,pos);
                        size_t pos2 = sLoaderPlugin.find(':',pos + 1);
                        std::string sLibraryName = sLoaderPlugin.substr(pos + 1,pos2- pos -1);
                        EventPublishMode publishMode = DELTA;

                        if(pos2 != std::string::npos) {
                            std::string sPublishMode = sLoaderPlugin.substr(pos2 + 1);
                            if(sPublishMode == "delta") { publishMode = DELTA; }
                            else if(sPublishMode == "full") { publishMode = FULL; }
                            else { throw makeException<ConfigureException>("EEL::Generator: Unkown 'loader' publish mode %s", sPublishMode.c_str()); }
                        }

                        LoaderPluginFactory * pPluginFactory = new LoaderPluginFactory();
                        pPluginFactory->construct("lib" + sLibraryName + ".so");
                        pluginFactoryList_.push_back(pPluginFactory);

                        std::pair<LoaderPlugin *,EventPublishMode> loaderEntry = std::make_pair(pPluginFactory->createPlugin(),publishMode);
                        size_t pos1 = 0;
                        pos2 = sEventTypes.find(',');

                        while(pos2 != std::string::npos) {
                            std::string sEventType = sEventTypes.substr(pos1,pos2 - pos1);
                            eventPluginMap_.insert(std::make_pair(sEventType,loaderEntry));
                            pos1 = pos2 + 1;
                            pos2 = sEventTypes.find(',',pos1);
                        }
                        std::string sEventType = sEventTypes.substr(pos1);
                        eventPluginMap_.insert(std::make_pair(sEventType,loaderEntry));
                    }
                    else {
                        throw makeException<ConfigureException>("EEL::Generator: Bad configuration 'loader' format %s", sLoaderPlugin.c_str());
                    }
                }
            }
            catch(Utils::FactoryException & exp) {
                throw makeException<ConfigureException>("EEL::Generator: Factory exception %s", exp.what());
            }
        }
        else {
            throw makeException<ConfigureException>("EEL::Generator: Unexpected configuration item %s", item.first.c_str());
        }
    }
}

void EMANE::Generators::EEL::Generator::start() {
    emane_rs_eel_generator_start(rs_state_);
}

void EMANE::Generators::EEL::Generator::stop() {
    emane_rs_eel_generator_stop(rs_state_);
}

void EMANE::Generators::EEL::Generator::destroy() throw() {
    emane_rs_eel_generator_destroy(rs_state_);
}

void EMANE::Generators::EEL::Generator::sendEvent(NEMId nemId, EventId eventId, const Serialization & serialization) {
    pPlatformService_->eventService().sendEvent(nemId, eventId, serialization);
}

void* EMANE::Generators::EEL::Generator::getPlugin(const std::string & eventType) {
    auto iter = eventPluginMap_.find(eventType);
    if(iter != eventPluginMap_.end()) {
        return &iter->second;
    }
    return nullptr;
}

size_t EMANE::Generators::EEL::Generator::getAllPlugins(void** plugins_out, size_t max_plugins) {
    size_t count = 0;
    std::list<std::pair<LoaderPlugin*, EventPublishMode>*> unique_plugins;
    for(auto& entry : eventPluginMap_) {
        bool found = false;
        for(auto* p : unique_plugins) {
            if(p == &entry.second) { found = true; break; }
        }
        if(!found) {
            unique_plugins.push_back(&entry.second);
        }
    }
    
    for(auto* p : unique_plugins) {
        if(count < max_plugins) {
            plugins_out[count] = p;
        }
        count++;
    }
    return count;
}

void EMANE::Generators::EEL::Generator::addPlugin(const std::string& sEventType, LoaderPlugin* pPlugin, EventPublishMode mode) {
    eventPluginMap_.insert(std::make_pair(sEventType, std::make_pair(pPlugin, mode)));
}

void EMANE::Generators::EEL::Generator::addPluginFactory(LoaderPluginFactory* pPluginFactory) {
    pluginFactoryList_.push_back(pPluginFactory);
}

DECLARE_EVENT_GENERATOR(EMANE::Generators::EEL::Generator);
