#include <cstdint>
#include <vector>
#include "emane/events/locationevent.h"
#include <cstring>

extern "C" {
    struct EmaneRsPosition {
        double latitude_degrees;
        double longitude_degrees;
        double altitude_meters;
    };

    struct EmaneRsVelocity {
        double azimuth_degrees;
        double elevation_degrees;
        double magnitude_meters_per_second;
    };

    struct EmaneRsOrientation {
        double roll_degrees;
        double pitch_degrees;
        double yaw_degrees;
    };

    struct EmaneRsLocation {
        uint32_t nem_id;
        EmaneRsPosition position;
        bool has_velocity;
        EmaneRsVelocity velocity;
        bool has_orientation;
        EmaneRsOrientation orientation;
    };

    uint8_t* emane_rs_location_event_serialize(
        const EmaneRsLocation* items,
        size_t num_items,
        size_t* out_len
    );

    void emane_rs_location_event_free_serialize(uint8_t* ptr, size_t len);

    bool emane_rs_location_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsLocation** out_items,
        size_t* out_num
    );

    void emane_rs_location_event_free_deserialize(EmaneRsLocation* ptr, size_t len);
}

class EMANE::Events::LocationEvent::Implementation
{
public:
  Implementation(const Locations & items):
    items_{items}{}

  const Locations & getLocations() const
  {
    return items_;
  }

private:
  Locations items_;
};

EMANE::Events::LocationEvent::LocationEvent(const Serialization & serialization):
  Event(IDENTIFIER)
{
  EmaneRsLocation* out_items = nullptr;
  size_t out_num = 0;

  if(!emane_rs_location_event_deserialize(
      reinterpret_cast<const uint8_t*>(serialization.c_str()),
      serialization.size(),
      &out_items,
      &out_num))
    {
      throw SerializationException("unable to deserialize LocationEvent");
    }

  Locations items;

  if (out_items && out_num > 0)
    {
      for(size_t i = 0; i < out_num; ++i)
        {
          Position pos(out_items[i].position.latitude_degrees,
                       out_items[i].position.longitude_degrees,
                       out_items[i].position.altitude_meters);
                       
          Orientation ori(0,0,0);
          bool has_ori = out_items[i].has_orientation;
          if (has_ori) {
              ori = Orientation(out_items[i].orientation.roll_degrees,
                                out_items[i].orientation.pitch_degrees,
                                out_items[i].orientation.yaw_degrees);
          }
          
          Velocity vel(0,0,0);
          bool has_vel = out_items[i].has_velocity;
          if (has_vel) {
              vel = Velocity(out_items[i].velocity.azimuth_degrees,
                             out_items[i].velocity.elevation_degrees,
                             out_items[i].velocity.magnitude_meters_per_second);
          }

          items.push_back(Location(
                static_cast<EMANE::NEMId>(out_items[i].nem_id),
                pos,
                {ori, has_ori},
                {vel, has_vel}
          ));
        }
      emane_rs_location_event_free_deserialize(out_items, out_num);
    }

  pImpl_.reset(new Implementation{items});
}

EMANE::Events::LocationEvent::LocationEvent(const Locations & items):
  Event{IDENTIFIER},
  pImpl_{new Implementation{items}}{}

EMANE::Events::LocationEvent::LocationEvent(const LocationEvent & rhs):
  Event{IDENTIFIER},
  pImpl_{new Implementation{rhs.getLocations()}}{}

EMANE::Events::LocationEvent & EMANE::Events::LocationEvent::operator=(const LocationEvent & rhs)
{
  pImpl_.reset(new Implementation{rhs.getLocations()});
  return *this;
}

EMANE::Events::LocationEvent::LocationEvent(LocationEvent && rval):
  Event{IDENTIFIER},
  pImpl_{new Implementation{{}}}
{
  rval.pImpl_.swap(pImpl_);
}

EMANE::Events::LocationEvent & EMANE::Events::LocationEvent::operator=(LocationEvent && rval)
{
  rval.pImpl_.swap(pImpl_);
  return *this;
}

EMANE::Events::LocationEvent::~LocationEvent(){}

const EMANE::Events::Locations & EMANE::Events::LocationEvent::getLocations() const
{
  return pImpl_->getLocations();
}

EMANE::Serialization EMANE::Events::LocationEvent::serialize() const
{
  const auto & items = pImpl_->getLocations();
  std::vector<EmaneRsLocation> rs_items;
  rs_items.reserve(items.size());
  
  for(auto & item : items)
    {
      EmaneRsLocation rs_item;
      rs_item.nem_id = item.getNEMId();
      
      const auto & pos = item.getPosition();
      rs_item.position.latitude_degrees = pos.getLatitudeDegrees();
      rs_item.position.longitude_degrees = pos.getLongitudeDegrees();
      rs_item.position.altitude_meters = pos.getAltitudeMeters();
      
      const auto & ori_pair = item.getOrientation();
      rs_item.has_orientation = ori_pair.second;
      if (rs_item.has_orientation) {
          rs_item.orientation.roll_degrees = ori_pair.first.getRollDegrees();
          rs_item.orientation.pitch_degrees = ori_pair.first.getPitchDegrees();
          rs_item.orientation.yaw_degrees = ori_pair.first.getYawDegrees();
      } else {
          rs_item.orientation.roll_degrees = 0.0;
          rs_item.orientation.pitch_degrees = 0.0;
          rs_item.orientation.yaw_degrees = 0.0;
      }
      
      const auto & vel_pair = item.getVelocity();
      rs_item.has_velocity = vel_pair.second;
      if (rs_item.has_velocity) {
          rs_item.velocity.azimuth_degrees = vel_pair.first.getAzimuthDegrees();
          rs_item.velocity.elevation_degrees = vel_pair.first.getElevationDegrees();
          rs_item.velocity.magnitude_meters_per_second = vel_pair.first.getMagnitudeMetersPerSecond();
      } else {
          rs_item.velocity.azimuth_degrees = 0.0;
          rs_item.velocity.elevation_degrees = 0.0;
          rs_item.velocity.magnitude_meters_per_second = 0.0;
      }

      rs_items.push_back(rs_item);
    }

  size_t out_len = 0;
  uint8_t* ptr = emane_rs_location_event_serialize(
      rs_items.empty() ? nullptr : rs_items.data(),
      rs_items.size(),
      &out_len
  );

  if(!ptr)
    {
      throw SerializationException("unable to serialize LocationEvent");
    }

  Serialization serialization(reinterpret_cast<const char*>(ptr), out_len);
  emane_rs_location_event_free_serialize(ptr, out_len);

  return serialization;
}
