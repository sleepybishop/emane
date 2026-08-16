rust_code = """
#[repr(C)]
pub struct EmaneRsPosition {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
    pub altitude_meters: f64,
}

#[repr(C)]
pub struct EmaneRsVelocity {
    pub azimuth_degrees: f64,
    pub elevation_degrees: f64,
    pub magnitude_meters_per_second: f64,
}

#[repr(C)]
pub struct EmaneRsOrientation {
    pub roll_degrees: f64,
    pub pitch_degrees: f64,
    pub yaw_degrees: f64,
}

#[repr(C)]
pub struct EmaneRsLocation {
    pub nem_id: u32,
    pub position: EmaneRsPosition,
    pub has_velocity: bool,
    pub velocity: EmaneRsVelocity,
    pub has_orientation: bool,
    pub orientation: EmaneRsOrientation,
}

#[no_mangle]
pub extern "C" fn emane_rs_location_event_serialize(
    items: *const EmaneRsLocation,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::LocationEvent::default();
    
    if num_items > 0 && !items.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(items, num_items) };
        for p in slice {
            let mut loc = emane_message::location_event::Location {
                nem_id: p.nem_id,
                position: emane_message::location_event::location::Position {
                    latitude_degrees: p.position.latitude_degrees,
                    longitude_degrees: p.position.longitude_degrees,
                    altitude_meters: p.position.altitude_meters,
                },
                velocity: if p.has_velocity {
                    Some(emane_message::location_event::location::Velocity {
                        azimuth_degrees: p.velocity.azimuth_degrees,
                        elevation_degrees: p.velocity.elevation_degrees,
                        magnitude_meters_per_second: p.velocity.magnitude_meters_per_second,
                    })
                } else {
                    None
                },
                orientation: if p.has_orientation {
                    Some(emane_message::location_event::location::Orientation {
                        roll_degrees: p.orientation.roll_degrees,
                        pitch_degrees: p.orientation.pitch_degrees,
                        yaw_degrees: p.orientation.yaw_degrees,
                    })
                } else {
                    None
                },
            };
            msg.locations.push(loc);
        }
    }
    
    let mut buf = Vec::with_capacity(prost::Message::encoded_len(&msg));
    if prost::Message::encode(&msg, &mut buf).is_ok() {
        let mut boxed = buf.into_boxed_slice();
        unsafe { *out_len = boxed.len() };
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        ptr
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsLocation,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    
    let slice = unsafe { std::slice::from_raw_parts(buf, len) };
    if let Ok(msg) = <emane_message::LocationEvent as prost::Message>::decode(slice) {
        let mut vec = Vec::with_capacity(msg.locations.len());
        for p in msg.locations {
            let pos = p.position; // required field, so we can access directly if the proto had it required (prost generated struct has it as T not Option<T> for required in proto2)
            // Wait, in prost, proto2 required is generated just like proto3 fields without Option unless specified. Actually, proto2 required generates `pub position: ::prost::alloc::boxed::Box<Position>` or something, or just `pub position: Position` if not boxed. Let's use `p.position` assuming it is generated as `pub position: ...`
            // Wait, for required message fields, prost generates `pub position: MessageType` or `Option<MessageType>`. Actually for message fields it always generates `Option<MessageType>`. We need to handle it.
            // Let's assume it's `Option<Position>`. If it's `Option<Position>`, we need to check if it's `Some`. If `None`, we can't deserialize correctly but proto2 guarantees it if parsed.
            // Wait, looking at `Position`, if it's `Option`, `p.position.unwrap_or_default()` works. Let's use `unwrap_or_default()`.
            // Wait, I don't know if prost generates it as Option. Yes, prost ALWAYS generates `Option<T>` for nested messages, regardless of `required` or `optional` in proto2.
            
            // Wait, if it generates `T` then `p.position` works. Let's just do `p.position` first, and if compilation fails we fix it.
            // Actually I'll use a match or `if let`.
            
            vec.push(EmaneRsLocation {
                nem_id: p.nem_id,
                position: if let Some(ref pos) = p.position {
                    EmaneRsPosition {
                        latitude_degrees: pos.latitude_degrees,
                        longitude_degrees: pos.longitude_degrees,
                        altitude_meters: pos.altitude_meters,
                    }
                } else {
                    EmaneRsPosition { latitude_degrees: 0.0, longitude_degrees: 0.0, altitude_meters: 0.0 }
                },
                has_velocity: p.velocity.is_some(),
                velocity: if let Some(ref vel) = p.velocity {
                    EmaneRsVelocity {
                        azimuth_degrees: vel.azimuth_degrees,
                        elevation_degrees: vel.elevation_degrees,
                        magnitude_meters_per_second: vel.magnitude_meters_per_second,
                    }
                } else {
                    EmaneRsVelocity { azimuth_degrees: 0.0, elevation_degrees: 0.0, magnitude_meters_per_second: 0.0 }
                },
                has_orientation: p.orientation.is_some(),
                orientation: if let Some(ref ori) = p.orientation {
                    EmaneRsOrientation {
                        roll_degrees: ori.roll_degrees,
                        pitch_degrees: ori.pitch_degrees,
                        yaw_degrees: ori.yaw_degrees,
                    }
                } else {
                    EmaneRsOrientation { roll_degrees: 0.0, pitch_degrees: 0.0, yaw_degrees: 0.0 }
                },
            });
        }
        
        let mut boxed = vec.into_boxed_slice();
        unsafe { 
            *out_num = boxed.len();
            *out_items = boxed.as_mut_ptr();
        }
        std::mem::forget(boxed);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_event_free_deserialize(ptr: *mut EmaneRsLocation, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}
"""

cpp_code = """#include "emane/events/locationevent.h"
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
"""

with open("rust/emane-core/src/events.rs", "a") as f:
    f.write(rust_code)
with open("src/libemane/locationevent.cc", "w") as f:
    f.write(cpp_code)
