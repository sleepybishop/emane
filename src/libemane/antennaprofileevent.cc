#include "emane/events/antennaprofileevent.h"
#include <cstring>

extern "C" {
    struct EmaneRsAntennaProfile {
        uint32_t nem_id;
        uint32_t profile_id;
        double antenna_azimuth_degrees;
        double antenna_elevation_degrees;
    };

    uint8_t* emane_rs_antennaprofile_event_serialize(
        const EmaneRsAntennaProfile* items,
        size_t num_items,
        size_t* out_len
    );

    void emane_rs_antennaprofile_event_free_serialize(uint8_t* ptr, size_t len);

    bool emane_rs_antennaprofile_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsAntennaProfile** out_items,
        size_t* out_num
    );

    void emane_rs_antennaprofile_event_free_deserialize(EmaneRsAntennaProfile* ptr, size_t len);
}

class EMANE::Events::AntennaProfileEvent::Implementation
{
public:
  Implementation(const AntennaProfiles & items):
    items_{items}{}

  const AntennaProfiles & getAntennaProfiles() const
  {
    return items_;
  }

private:
  AntennaProfiles items_;
};

EMANE::Events::AntennaProfileEvent::AntennaProfileEvent(const Serialization & serialization):
  Event(IDENTIFIER)
{
  EmaneRsAntennaProfile* out_items = nullptr;
  size_t out_num = 0;

  if(!emane_rs_antennaprofile_event_deserialize(
      reinterpret_cast<const uint8_t*>(serialization.c_str()),
      serialization.size(),
      &out_items,
      &out_num))
    {
      throw SerializationException("unable to deserialize AntennaProfileEvent");
    }

  AntennaProfiles items;

  if (out_items && out_num > 0)
    {
      for(size_t i = 0; i < out_num; ++i)
        {
          items.emplace_back(
                static_cast<EMANE::NEMId>(out_items[i].nem_id),
                static_cast<EMANE::AntennaProfileId>(out_items[i].profile_id),
                out_items[i].antenna_azimuth_degrees,
                out_items[i].antenna_elevation_degrees
          );
        }
      emane_rs_antennaprofile_event_free_deserialize(out_items, out_num);
    }

  pImpl_.reset(new Implementation{items});
}

EMANE::Events::AntennaProfileEvent::AntennaProfileEvent(const AntennaProfiles & items):
  Event{IDENTIFIER},
  pImpl_{new Implementation{items}}{}

EMANE::Events::AntennaProfileEvent::AntennaProfileEvent(const AntennaProfileEvent & rhs):
  Event{IDENTIFIER},
  pImpl_{new Implementation{rhs.getAntennaProfiles()}}{}

EMANE::Events::AntennaProfileEvent & EMANE::Events::AntennaProfileEvent::operator=(const AntennaProfileEvent & rhs)
{
  pImpl_.reset(new Implementation{rhs.getAntennaProfiles()});
  return *this;
}

EMANE::Events::AntennaProfileEvent::AntennaProfileEvent(AntennaProfileEvent && rval):
  Event{IDENTIFIER},
  pImpl_{new Implementation{{}}}
{
  rval.pImpl_.swap(pImpl_);
}

EMANE::Events::AntennaProfileEvent & EMANE::Events::AntennaProfileEvent::operator=(AntennaProfileEvent && rval)
{
  rval.pImpl_.swap(pImpl_);
  return *this;
}

EMANE::Events::AntennaProfileEvent::~AntennaProfileEvent(){}

const EMANE::Events::AntennaProfiles & EMANE::Events::AntennaProfileEvent::getAntennaProfiles() const
{
  return pImpl_->getAntennaProfiles();
}

EMANE::Serialization EMANE::Events::AntennaProfileEvent::serialize() const
{
  const auto & items = pImpl_->getAntennaProfiles();
  std::vector<EmaneRsAntennaProfile> rs_items;
  rs_items.reserve(items.size());
  
  for(auto & item : items)
    {
      EmaneRsAntennaProfile rs_item;
      rs_item.nem_id = item.getNEMId();
      rs_item.profile_id = item.getAntennaProfileId();
      rs_item.antenna_azimuth_degrees = item.getAntennaAzimuthDegrees();
      rs_item.antenna_elevation_degrees = item.getAntennaElevationDegrees();
      rs_items.push_back(rs_item);
    }

  size_t out_len = 0;
  uint8_t* ptr = emane_rs_antennaprofile_event_serialize(
      rs_items.empty() ? nullptr : rs_items.data(),
      rs_items.size(),
      &out_len
  );

  if(!ptr)
    {
      throw SerializationException("unable to serialize AntennaProfileEvent");
    }

  Serialization serialization(reinterpret_cast<const char*>(ptr), out_len);
  emane_rs_antennaprofile_event_free_serialize(ptr, out_len);

  return serialization;
}
