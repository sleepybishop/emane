#include "emane/events/fadingselectionevent.h"
#include <cstring>

extern "C" {
    struct EmaneRsFadingSelection {
        uint32_t nem_id;
        uint32_t model;
    };

    uint8_t* emane_rs_fadingselection_event_serialize(
        const EmaneRsFadingSelection* items,
        size_t num_items,
        size_t* out_len
    );

    void emane_rs_fadingselection_event_free_serialize(uint8_t* ptr, size_t len);

    bool emane_rs_fadingselection_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsFadingSelection** out_items,
        size_t* out_num
    );

    void emane_rs_fadingselection_event_free_deserialize(EmaneRsFadingSelection* ptr, size_t len);
}

class EMANE::Events::FadingSelectionEvent::Implementation
{
public:
  Implementation(const FadingSelections & items):
    items_{items}{}

  const FadingSelections & getFadingSelections() const
  {
    return items_;
  }

private:
  FadingSelections items_;
};

EMANE::Events::FadingSelectionEvent::FadingSelectionEvent(const Serialization & serialization):
  Event(IDENTIFIER)
{
  EmaneRsFadingSelection* out_items = nullptr;
  size_t out_num = 0;

  if(!emane_rs_fadingselection_event_deserialize(
      reinterpret_cast<const uint8_t*>(serialization.c_str()),
      serialization.size(),
      &out_items,
      &out_num))
    {
      throw SerializationException("unable to deserialize FadingSelectionEvent");
    }

  FadingSelections items;

  if (out_items && out_num > 0)
    {
      for(size_t i = 0; i < out_num; ++i)
        {
          items.push_back({
                static_cast<EMANE::NEMId>(out_items[i].nem_id),
                static_cast<EMANE::Events::FadingModel>(out_items[i].model)
          });
        }
      emane_rs_fadingselection_event_free_deserialize(out_items, out_num);
    }

  pImpl_.reset(new Implementation{items});
}

EMANE::Events::FadingSelectionEvent::FadingSelectionEvent(const FadingSelections & items):
  Event{IDENTIFIER},
  pImpl_{new Implementation{items}}{}

EMANE::Events::FadingSelectionEvent::FadingSelectionEvent(const FadingSelectionEvent & rhs):
  Event{IDENTIFIER},
  pImpl_{new Implementation{rhs.getFadingSelections()}}{}

EMANE::Events::FadingSelectionEvent & EMANE::Events::FadingSelectionEvent::operator=(const FadingSelectionEvent & rhs)
{
  pImpl_.reset(new Implementation{rhs.getFadingSelections()});
  return *this;
}

EMANE::Events::FadingSelectionEvent::FadingSelectionEvent(FadingSelectionEvent && rval):
  Event{IDENTIFIER},
  pImpl_{new Implementation{{}}}
{
  rval.pImpl_.swap(pImpl_);
}

EMANE::Events::FadingSelectionEvent & EMANE::Events::FadingSelectionEvent::operator=(FadingSelectionEvent && rval)
{
  rval.pImpl_.swap(pImpl_);
  return *this;
}

EMANE::Events::FadingSelectionEvent::~FadingSelectionEvent(){}

const EMANE::Events::FadingSelections & EMANE::Events::FadingSelectionEvent::getFadingSelections() const
{
  return pImpl_->getFadingSelections();
}

EMANE::Serialization EMANE::Events::FadingSelectionEvent::serialize() const
{
  const auto & items = pImpl_->getFadingSelections();
  std::vector<EmaneRsFadingSelection> rs_items;
  rs_items.reserve(items.size());
  
  for(auto & item : items)
    {
      EmaneRsFadingSelection rs_item;
      rs_item.nem_id = item.getNEMId();
      rs_item.model = static_cast<uint32_t>(item.getFadingModel());
      rs_items.push_back(rs_item);
    }

  size_t out_len = 0;
  uint8_t* ptr = emane_rs_fadingselection_event_serialize(
      rs_items.empty() ? nullptr : rs_items.data(),
      rs_items.size(),
      &out_len
  );

  if(!ptr)
    {
      throw SerializationException("unable to serialize FadingSelectionEvent");
    }

  Serialization serialization(reinterpret_cast<const char*>(ptr), out_len);
  emane_rs_fadingselection_event_free_serialize(ptr, out_len);

  return serialization;
}
