#include "eelloadercommeffect.h"
#include "emane/events/commeffectevent.h"
#include "emane/generators/eel/formatexception.h"

#include <vector>
#include <string>

extern "C" {
    void* emane_commeffect_loader_new();
    void emane_commeffect_loader_drop(void* ptr);
    bool emane_commeffect_loader_load(void* ptr, const char* module_type, uint16_t module_id, const char** args, size_t num_args, char** error_out);
    void emane_commeffect_loader_free_error(char* err);
    typedef void (*CommEffectEventCallback)(uint16_t nem_id, const uint8_t* payload, size_t payload_len, void* ctx);
    void emane_commeffect_loader_get_events(void* ptr, uint8_t mode, CommEffectEventCallback cb, void* ctx);
}

EMANE::Generators::EEL::LoaderCommEffect::LoaderCommEffect()
{
    rust_loader_ = emane_commeffect_loader_new();
}
    
EMANE::Generators::EEL::LoaderCommEffect::~LoaderCommEffect()
{
    if (rust_loader_) {
        emane_commeffect_loader_drop(rust_loader_);
        rust_loader_ = nullptr;
    }
}

void EMANE::Generators::EEL::LoaderCommEffect::load(const ModuleType & moduleType, 
                                                    const ModuleId   & moduleId, 
                                                    const EventType  & ,
                                                    const InputArguments & args)
{
    std::vector<const char*> c_args;
    for (const auto& arg : args) {
        c_args.push_back(arg.c_str());
    }

    char* error_out = nullptr;
    bool success = emane_commeffect_loader_load(rust_loader_, moduleType.c_str(), moduleId, c_args.data(), c_args.size(), &error_out);
    
    if (!success) {
        std::string err_str = error_out ? error_out : "Unknown error in Rust loader";
        if (error_out) {
            emane_commeffect_loader_free_error(error_out);
        }
        throw FormatException(err_str);
    }
}
    
static void commEffectEventCallback(uint16_t nem_id, const uint8_t* payload, size_t payload_len, void* ctx) {
    auto list = static_cast<EMANE::Generators::EEL::EventInfoList*>(ctx);
    std::string serialization(reinterpret_cast<const char*>(payload), payload_len);
    list->push_back({nem_id, EMANE::Events::CommEffectEvent::IDENTIFIER, serialization});
}

EMANE::Generators::EEL::EventInfoList EMANE::Generators::EEL::LoaderCommEffect::getEvents(EventPublishMode mode)
{
    EventInfoList eventInfoList;
    uint8_t rust_mode = (mode == DELTA) ? 0 : 1;
    emane_commeffect_loader_get_events(rust_loader_, rust_mode, commEffectEventCallback, &eventInfoList);
    return eventInfoList;
}

DECLARE_EEL_LOADER_PLUGIN(EMANE::Generators::EEL::LoaderCommEffect)
