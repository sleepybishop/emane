#include "eelloaderlocation.h"
#include "emane/events/locationevent.h"
#include "emane/utils/parameterconvert.h"
#include "emane/generators/eel/formatexception.h"

#include <sstream>
#include <cstring>
#include <vector>

extern "C" void* emane_location_loader_create();
extern "C" void emane_location_loader_destroy(void* ptr);
extern "C" bool emane_location_loader_load(void* ptr, const char* module_type, uint16_t module_id, const char* event_type, const char** args, size_t argc);
extern "C" void emane_location_loader_get_events(void* ptr, int mode, void* callback_data, void (*cb)(void*, uint16_t, uint16_t, const uint8_t*, size_t));

EMANE::Generators::EEL::LoaderLocation::LoaderLocation()
{
  rust_ptr_ = emane_location_loader_create();
}

EMANE::Generators::EEL::LoaderLocation::~LoaderLocation()
{
  emane_location_loader_destroy(rust_ptr_);
}

void EMANE::Generators::EEL::LoaderLocation::load(const ModuleType & moduleType, 
                                                  const ModuleId   & moduleId, 
                                                  const EventType  & eventType,
                                                  const InputArguments & args)
{
  std::vector<const char*> c_args;
  for(const auto& s : args) {
      c_args.push_back(s.c_str());
  }
  if (!emane_location_loader_load(rust_ptr_, moduleType.c_str(), moduleId, eventType.c_str(), c_args.data(), c_args.size())) {
      throw FormatException("LoaderLocation error inside rust");
  }
}

EMANE::Generators::EEL::EventInfoList EMANE::Generators::EEL::LoaderLocation::getEvents(EventPublishMode mode)
{
  EventInfoList eventInfoList;

  auto cb = [](void* ctx, uint16_t nem_id, uint16_t event_id, const uint8_t* data, size_t len) {
      auto* list = static_cast<EventInfoList*>(ctx);
      list->push_back({nem_id, event_id, {reinterpret_cast<const char*>(data), len}});
  };
  
  emane_location_loader_get_events(rust_ptr_, mode == DELTA ? 0 : 1, &eventInfoList, cb);
  
  return eventInfoList;
}

DECLARE_EEL_LOADER_PLUGIN(EMANE::Generators::EEL::LoaderLocation)
