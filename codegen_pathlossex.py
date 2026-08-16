rust_code = """
#[repr(C)]
pub struct EmaneRsPathlossExEntry {
    pub frequency_hz: u64,
    pub pathloss_db: f32,
}

#[repr(C)]
pub struct EmaneRsPathlossEx {
    pub nem_id: u32,
    pub entries: *const EmaneRsPathlossExEntry,
    pub num_entries: usize,
}

#[no_mangle]
pub extern "C" fn emane_rs_pathlossex_event_serialize(
    items: *const EmaneRsPathlossEx,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::PathlossExEvent::default();
    
    if num_items > 0 && !items.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(items, num_items) };
        for p in slice {
            let mut pathloss = emane_message::pathloss_ex_event::Pathloss {
                nem_id: p.nem_id,
                entries: Vec::new(),
            };
            if p.num_entries > 0 && !p.entries.is_null() {
                let entry_slice = unsafe { std::slice::from_raw_parts(p.entries, p.num_entries) };
                for e in entry_slice {
                    pathloss.entries.push(emane_message::pathloss_ex_event::pathloss::Entry {
                        frequency_hz: e.frequency_hz,
                        pathlossd_b: e.pathloss_db,
                    });
                }
            }
            msg.pathlosses.push(pathloss);
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
pub extern "C" fn emane_rs_pathlossex_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

// For deserialize, we need to return a dynamic array of EmaneRsPathlossEx,
// and each of those needs a dynamic array of EmaneRsPathlossExEntry.
// Because the C++ side will need to copy this and then call free, we'll
// allocate an array of entries for each pathloss.
// To make it easy to free, we'll just free each entries array and then the pathlosses array.

#[no_mangle]
pub extern "C" fn emane_rs_pathlossex_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsPathlossEx,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    
    let slice = unsafe { std::slice::from_raw_parts(buf, len) };
    if let Ok(msg) = <emane_message::PathlossExEvent as prost::Message>::decode(slice) {
        let mut vec = Vec::with_capacity(msg.pathlosses.len());
        for p in msg.pathlosses {
            let mut entries_vec = Vec::with_capacity(p.entries.len());
            for e in p.entries {
                entries_vec.push(EmaneRsPathlossExEntry {
                    frequency_hz: e.frequency_hz,
                    pathloss_db: e.pathlossd_b,
                });
            }
            let mut entries_boxed = entries_vec.into_boxed_slice();
            let entries_ptr = entries_boxed.as_mut_ptr();
            let entries_len = entries_boxed.len();
            std::mem::forget(entries_boxed);
            
            vec.push(EmaneRsPathlossEx {
                nem_id: p.nem_id,
                entries: entries_ptr,
                num_entries: entries_len,
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
pub extern "C" fn emane_rs_pathlossex_event_free_deserialize(ptr: *mut EmaneRsPathlossEx, len: usize) {
    if !ptr.is_null() {
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
        for p in slice.iter() {
            if !p.entries.is_null() && p.num_entries > 0 {
                unsafe {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(p.entries as *mut EmaneRsPathlossExEntry, p.num_entries)));
                }
            }
        }
        unsafe {
            drop(Box::from_raw(slice));
        }
    }
}
"""

cpp_code = """#include "emane/events/pathlossexevent.h"
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
"""

with open("rust/emane-core/src/events.rs", "a") as f:
    f.write(rust_code)
with open("src/libemane/pathlossexevent.cc", "w") as f:
    f.write(cpp_code)
