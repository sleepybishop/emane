#include "eelloaderfadingselection.h"
#include "emane/events/fadingselectionevent.h"
#include "emane/utils/parameterconvert.h"
#include "emane/generators/eel/formatexception.h"

#include <vector>
#include <sstream>
#include <cstring>

extern "C" void* emane_fadingselection_loader_create();
extern "C" void emane_fadingselection_loader_destroy(void* ptr);
extern "C" bool emane_fadingselection_loader_load(void* ptr, const char* module_type, uint16_t module_id, const char* event_type, const char** args, size_t argc);
extern "C" void emane_fadingselection_loader_get_events(void* ptr, int mode, void* callback_data, void (*cb)(void*, uint16_t, uint16_t, const uint8_t*, size_t));

EMANE::Generators::EEL::LoaderFadingSelection::LoaderFadingSelection()
{
  rust_ptr_ = emane_fadingselection_loader_create();
}

EMANE::Generators::EEL::LoaderFadingSelection::~LoaderFadingSelection()
{
  emane_fadingselection_loader_destroy(rust_ptr_);
}

void EMANE::Generators::EEL::LoaderFadingSelection::load(const ModuleType & moduleType,
                                                         const ModuleId   & moduleId,
                                                         const EventType  & eventType,
                                                         const InputArguments & args)
{
  std::vector<const char*> c_args;
  for(const auto& s : args) {
      c_args.push_back(s.c_str());
  }
  if (!emane_fadingselection_loader_load(rust_ptr_, moduleType.c_str(), moduleId, eventType.c_str(), c_args.data(), c_args.size())) {
      throw FormatException("LoaderFadingSelection error inside rust");
  }
}

EMANE::Generators::EEL::EventInfoList EMANE::Generators::EEL::LoaderFadingSelection::getEvents(EventPublishMode mode)
{
  EventInfoList eventInfoList;
  
  auto cb = [](void* ctx, uint16_t nem_id, uint16_t event_id, const uint8_t* data, size_t len) {
      auto* list = static_cast<EventInfoList*>(ctx);
      list->push_back({nem_id, event_id, {reinterpret_cast<const char*>(data), len}});
  };
  
  emane_fadingselection_loader_get_events(rust_ptr_, mode == DELTA ? 0 : 1, &eventInfoList, cb);
  return eventInfoList;
}

DECLARE_EEL_LOADER_PLUGIN(EMANE::Generators::EEL::LoaderFadingSelection)
