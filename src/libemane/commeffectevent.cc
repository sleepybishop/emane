#include <cstdint>
#include "emane/events/commeffectevent.h"
#include <cstring>

extern "C" {
    struct EmaneRsCommEffect {
        uint32_t nem_id;
        float latency_seconds;
        float jitter_seconds;
        float probability_loss;
        float probability_duplicate;
        uint64_t unicast_bit_rate_bps;
        uint64_t broadcast_bit_rate_bps;
    };

    uint8_t* emane_rs_commeffect_event_serialize(
        const EmaneRsCommEffect* items,
        size_t num_items,
        size_t* out_len
    );

    void emane_rs_commeffect_event_free_serialize(uint8_t* ptr, size_t len);

    bool emane_rs_commeffect_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsCommEffect** out_items,
        size_t* out_num
    );

    void emane_rs_commeffect_event_free_deserialize(EmaneRsCommEffect* ptr, size_t len);
}

class EMANE::Events::CommEffectEvent::Implementation
{
public:
  Implementation(const CommEffects & items):
    items_{items}{}

  const CommEffects & getCommEffects() const
  {
    return items_;
  }

private:
  CommEffects items_;
};

EMANE::Events::CommEffectEvent::CommEffectEvent(const Serialization & serialization):
  Event(IDENTIFIER)
{
  EmaneRsCommEffect* out_items = nullptr;
  size_t out_num = 0;

  if(!emane_rs_commeffect_event_deserialize(
      reinterpret_cast<const uint8_t*>(serialization.c_str()),
      serialization.size(),
      &out_items,
      &out_num))
    {
      throw SerializationException("unable to deserialize CommEffectEvent");
    }

  CommEffects items;

  if (out_items && out_num > 0)
    {
      for(size_t i = 0; i < out_num; ++i)
        {
          std::chrono::duration<float> latency{out_items[i].latency_seconds};
          std::chrono::duration<float> jitter{out_items[i].jitter_seconds};
          items.emplace_back(
                static_cast<EMANE::NEMId>(out_items[i].nem_id),
                std::chrono::duration_cast<EMANE::Microseconds>(latency),
                std::chrono::duration_cast<EMANE::Microseconds>(jitter),
                out_items[i].probability_loss,
                out_items[i].probability_duplicate,
                out_items[i].unicast_bit_rate_bps,
                out_items[i].broadcast_bit_rate_bps
          );
        }
      emane_rs_commeffect_event_free_deserialize(out_items, out_num);
    }

  pImpl_.reset(new Implementation{items});
}

EMANE::Events::CommEffectEvent::CommEffectEvent(const CommEffects & items):
  Event{IDENTIFIER},
  pImpl_{new Implementation{items}}{}

EMANE::Events::CommEffectEvent::CommEffectEvent(const CommEffectEvent & rhs):
  Event{IDENTIFIER},
  pImpl_{new Implementation{rhs.getCommEffects()}}{}

EMANE::Events::CommEffectEvent & EMANE::Events::CommEffectEvent::operator=(const CommEffectEvent & rhs)
{
  pImpl_.reset(new Implementation{rhs.getCommEffects()});
  return *this;
}

EMANE::Events::CommEffectEvent::CommEffectEvent(CommEffectEvent && rval):
  Event{IDENTIFIER},
  pImpl_{new Implementation{{}}}
{
  rval.pImpl_.swap(pImpl_);
}

EMANE::Events::CommEffectEvent & EMANE::Events::CommEffectEvent::operator=(CommEffectEvent && rval)
{
  rval.pImpl_.swap(pImpl_);
  return *this;
}

EMANE::Events::CommEffectEvent::~CommEffectEvent(){}

const EMANE::Events::CommEffects & EMANE::Events::CommEffectEvent::getCommEffects() const
{
  return pImpl_->getCommEffects();
}

EMANE::Serialization EMANE::Events::CommEffectEvent::serialize() const
{
  const auto & items = pImpl_->getCommEffects();
  std::vector<EmaneRsCommEffect> rs_items;
  rs_items.reserve(items.size());
  
  for(auto & item : items)
    {
      EmaneRsCommEffect rs_item;
      rs_item.nem_id = item.getNEMId();
      rs_item.latency_seconds = std::chrono::duration_cast<std::chrono::duration<float>>(item.getLatency()).count();
      rs_item.jitter_seconds = std::chrono::duration_cast<std::chrono::duration<float>>(item.getJitter()).count();
      rs_item.probability_loss = item.getProbabilityLoss();
      rs_item.probability_duplicate = item.getProbabilityDuplicate();
      rs_item.unicast_bit_rate_bps = item.getUnicastBitRate();
      rs_item.broadcast_bit_rate_bps = item.getBroadcastBitRate();
      rs_items.push_back(rs_item);
    }

  size_t out_len = 0;
  uint8_t* ptr = emane_rs_commeffect_event_serialize(
      rs_items.empty() ? nullptr : rs_items.data(),
      rs_items.size(),
      &out_len
  );

  if(!ptr)
    {
      throw SerializationException("unable to serialize CommEffectEvent");
    }

  Serialization serialization(reinterpret_cast<const char*>(ptr), out_len);
  emane_rs_commeffect_event_free_serialize(ptr, out_len);

  return serialization;
}
