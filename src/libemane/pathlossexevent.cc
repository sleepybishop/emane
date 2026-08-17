#include <cstdint>
#include <vector>
#include "emane/events/pathlossexevent.h"
#include <cstring>

extern "C" {
    struct EmaneRsPathlossExEntry {
        uint64_t frequency_hz;
        float pathloss_db;
    };

    struct EmaneRsPathlossEx {
        uint32_t nem_id;
        const EmaneRsPathlossExEntry* entries;
        size_t num_entries;
    };

    uint8_t* emane_rs_pathlossex_event_serialize(
        const EmaneRsPathlossEx* items,
        size_t num_items,
        size_t* out_len
    );

    void emane_rs_pathlossex_event_free_serialize(uint8_t* ptr, size_t len);

    bool emane_rs_pathlossex_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsPathlossEx** out_items,
        size_t* out_num
    );

    void emane_rs_pathlossex_event_free_deserialize(EmaneRsPathlossEx* ptr, size_t len);
}

class EMANE::Events::PathlossExEvent::Implementation
{
public:
  Implementation(const PathlossExs & items):
    items_{items}{}

  const PathlossExs & getPathlossExs() const
  {
    return items_;
  }

private:
  PathlossExs items_;
};

EMANE::Events::PathlossExEvent::PathlossExEvent(const Serialization & serialization):
  Event(IDENTIFIER)
{
  EmaneRsPathlossEx* out_items = nullptr;
  size_t out_num = 0;

  if(!emane_rs_pathlossex_event_deserialize(
      reinterpret_cast<const uint8_t*>(serialization.c_str()),
      serialization.size(),
      &out_items,
      &out_num))
    {
      throw SerializationException("unable to deserialize PathlossExEvent");
    }

  PathlossExs items;

  if (out_items && out_num > 0)
    {
      for(size_t i = 0; i < out_num; ++i)
        {
          PathlossEx::FrequencyPathlossMap fpMap;
          if (out_items[i].entries && out_items[i].num_entries > 0)
            {
              for (size_t j = 0; j < out_items[i].num_entries; ++j)
                {
                  fpMap.insert({out_items[i].entries[j].frequency_hz, out_items[i].entries[j].pathloss_db});
                }
            }
          items.push_back({
                static_cast<EMANE::NEMId>(out_items[i].nem_id),
                std::move(fpMap)
          });
        }
      emane_rs_pathlossex_event_free_deserialize(out_items, out_num);
    }

  pImpl_.reset(new Implementation{items});
}

EMANE::Events::PathlossExEvent::PathlossExEvent(const PathlossExs & items):
  Event{IDENTIFIER},
  pImpl_{new Implementation{items}}{}

EMANE::Events::PathlossExEvent::PathlossExEvent(const PathlossExEvent & rhs):
  Event{IDENTIFIER},
  pImpl_{new Implementation{rhs.getPathlossExs()}}{}

EMANE::Events::PathlossExEvent & EMANE::Events::PathlossExEvent::operator=(const PathlossExEvent & rhs)
{
  pImpl_.reset(new Implementation{rhs.getPathlossExs()});
  return *this;
}

EMANE::Events::PathlossExEvent::PathlossExEvent(PathlossExEvent && rval):
  Event{IDENTIFIER},
  pImpl_{new Implementation{{}}}
{
  rval.pImpl_.swap(pImpl_);
}

EMANE::Events::PathlossExEvent & EMANE::Events::PathlossExEvent::operator=(PathlossExEvent && rval)
{
  rval.pImpl_.swap(pImpl_);
  return *this;
}

EMANE::Events::PathlossExEvent::~PathlossExEvent(){}

const EMANE::Events::PathlossExs & EMANE::Events::PathlossExEvent::getPathlossExs() const
{
  return pImpl_->getPathlossExs();
}

EMANE::Serialization EMANE::Events::PathlossExEvent::serialize() const
{
  const auto & items = pImpl_->getPathlossExs();
  std::vector<EmaneRsPathlossEx> rs_items;
  std::vector<std::vector<EmaneRsPathlossExEntry>> rs_entries(items.size());
  rs_items.reserve(items.size());
  
  size_t idx = 0;
  for(auto & item : items)
    {
      const auto & fpMap = item.getFrequencyPathlossMap();
      rs_entries[idx].reserve(fpMap.size());
      for (const auto & entry : fpMap)
        {
          rs_entries[idx].push_back({entry.first, entry.second});
        }

      EmaneRsPathlossEx rs_item;
      rs_item.nem_id = item.getNEMId();
      rs_item.entries = rs_entries[idx].empty() ? nullptr : rs_entries[idx].data();
      rs_item.num_entries = rs_entries[idx].size();
      rs_items.push_back(rs_item);
      ++idx;
    }

  size_t out_len = 0;
  uint8_t* ptr = emane_rs_pathlossex_event_serialize(
      rs_items.empty() ? nullptr : rs_items.data(),
      rs_items.size(),
      &out_len
  );

  if(!ptr)
    {
      throw SerializationException("unable to serialize PathlossExEvent");
    }

  Serialization serialization(reinterpret_cast<const char*>(ptr), out_len);
  emane_rs_pathlossex_event_free_serialize(ptr, out_len);

  return serialization;
}
