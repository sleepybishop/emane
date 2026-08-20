#include "eeleventgenerator.h"
#include "emane/generators/eel/formatexception.h"
#include "emane/configureexception.h"

extern "C" {
    struct EelCallbacks {
        void (*send_event)(void*, uint16_t, uint16_t, const void*, size_t);
        void (*plugin_load)(void*, const char*, uint16_t, const char*, const char* const*, size_t);
        void (*plugin_get_events)(void*, void (*)(uint16_t, uint16_t, const void*, size_t, void*), void*);
    };

    void* emane_rs_eel_generator_new(void* c_generator, EelCallbacks callbacks);
    void emane_rs_eel_generator_free(void* ptr);
    void emane_rs_eel_generator_add_input_file(void* ptr, const char* filename);
    bool emane_rs_eel_generator_add_loader(void* ptr, const char* loader, char** error_out);
    void emane_rs_eel_generator_start(void* ptr);
    void emane_rs_eel_generator_stop(void* ptr);

    void emane_c_eel_generator_send_event(void* generator, uint16_t nem_id, uint16_t event_id, const void* data, size_t len) {
        auto* gen = static_cast<EMANE::Generators::EEL::Generator*>(generator);
        EMANE::Serialization serialization(static_cast<const char*>(data), len);
        gen->sendEvent(nem_id, event_id, serialization);
    }
    
    void emane_c_eel_plugin_load(void* plugin_ptr, const char* module_type, uint16_t module_id, const char* event_type, const char* const* args, size_t num_args) {
        auto* entry = static_cast<EMANE::Generators::EEL::LoaderPlugin*>(plugin_ptr);
        if(!entry) return;
        
        EMANE::Generators::EEL::InputArguments input_args;
        for(size_t i = 0; i < num_args; ++i) {
            input_args.push_back(args[i]);
        }
        
        entry->load(module_type, module_id, event_type, input_args);
    }
    
    void emane_c_eel_plugin_get_events(void* plugin_ptr, void (*callback)(uint16_t, uint16_t, const void*, size_t, void*), void* user_data) {
        auto* entry = static_cast<EMANE::Generators::EEL::LoaderPlugin*>(plugin_ptr);
        if(!entry) return;
        
        EMANE::Generators::EEL::EventInfoList eventList = entry->getEvents(EMANE::Generators::EEL::DELTA); // Note: Should probably pass publish mode from Rust, but let's default to DELTA here as it was in original code
        for(auto& event : eventList) {
            const auto& serialization = event.getSerialization();
            callback(event.getNEMId(), event.getEventId(), serialization.c_str(), serialization.length(), user_data);
        }
    }
    
    void emane_rs_eel_loader_plugin_factory_free_error(char* err);
}

EMANE::Generators::EEL::Generator::Generator(PlatformServiceProvider *pPlatformService):
  EventGenerator(pPlatformService)
{
    EelCallbacks cb;
    cb.send_event = emane_c_eel_generator_send_event;
    cb.plugin_load = emane_c_eel_plugin_load;
    cb.plugin_get_events = emane_c_eel_plugin_get_events;
    rs_state_ = emane_rs_eel_generator_new(this, cb);
}

EMANE::Generators::EEL::Generator::~Generator() {
    emane_rs_eel_generator_free(rs_state_);
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
            for(const auto & any : item.second) {
                std::string sLoaderPlugin = any.asString();
                char* error_out = nullptr;
                if(!emane_rs_eel_generator_add_loader(rs_state_, sLoaderPlugin.c_str(), &error_out)) {
                    std::string err(error_out);
                    emane_rs_eel_loader_plugin_factory_free_error(error_out);
                    throw makeException<ConfigureException>("EEL::Generator: %s", err.c_str());
                }
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
}

void EMANE::Generators::EEL::Generator::sendEvent(NEMId nemId, EventId eventId, const Serialization & serialization) {
    pPlatformService_->eventService().sendEvent(nemId, eventId, serialization);
}

DECLARE_EVENT_GENERATOR(EMANE::Generators::EEL::Generator);
