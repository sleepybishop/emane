use crate::protobufs::emane_message;
use prost::Message;
use std::slice;

#[repr(C)]
pub struct EmaneRsPathloss {
    pub nem_id: u32,
    pub forward_pathloss_db: f32,
    pub reverse_pathloss_db: f32,
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_serialize(
    pathlosses: *const EmaneRsPathloss,
    num_pathlosses: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::PathlossEvent::default();

    if num_pathlosses > 0 && !pathlosses.is_null() {
        let slice = unsafe { slice::from_raw_parts(pathlosses, num_pathlosses) };
        for p in slice {
            msg.pathlosses
                .push(emane_message::pathloss_event::Pathloss {
                    nem_id: p.nem_id,
                    forward_pathlossd_b: p.forward_pathloss_db,
                    reverse_pathlossd_b: p.reverse_pathloss_db,
                });
        }
    }

    let mut buf = Vec::with_capacity(msg.encoded_len());
    if msg.encode(&mut buf).is_ok() {
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
pub extern "C" fn emane_rs_pathloss_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            // Reconstruct the boxed slice to drop it
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_deserialize(
    buf: *const u8,
    len: usize,
    out_pathlosses: *mut *mut EmaneRsPathloss,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }

    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::PathlossEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.pathlosses.len());
        for p in msg.pathlosses {
            vec.push(EmaneRsPathloss {
                nem_id: p.nem_id,
                forward_pathloss_db: p.forward_pathlossd_b,
                reverse_pathloss_db: p.reverse_pathlossd_b,
            });
        }

        let mut boxed = vec.into_boxed_slice();
        unsafe {
            *out_num = boxed.len();
            *out_pathlosses = boxed.as_mut_ptr();
        }
        std::mem::forget(boxed);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_free_deserialize(ptr: *mut EmaneRsPathloss, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[repr(C)]
pub struct EmaneRsAntennaProfile {
    pub nem_id: u32,
    pub profile_id: u32,
    pub antenna_azimuth_degrees: f64,
    pub antenna_elevation_degrees: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_antennaprofile_event_serialize(
    items: *const EmaneRsAntennaProfile,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::AntennaProfileEvent::default();

    if num_items > 0 && !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, num_items) };
        for p in slice {
            msg.profiles
                .push(emane_message::antenna_profile_event::Profile {
                    nem_id: p.nem_id,
                    profile_id: p.profile_id,
                    antenna_azimuth_degrees: p.antenna_azimuth_degrees,
                    antenna_elevation_degrees: p.antenna_elevation_degrees,
                });
        }
    }

    let mut buf = Vec::with_capacity(msg.encoded_len());
    if msg.encode(&mut buf).is_ok() {
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
pub extern "C" fn emane_rs_antennaprofile_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antennaprofile_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsAntennaProfile,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }

    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::AntennaProfileEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.profiles.len());
        for p in msg.profiles {
            vec.push(EmaneRsAntennaProfile {
                nem_id: p.nem_id,
                profile_id: p.profile_id,
                antenna_azimuth_degrees: p.antenna_azimuth_degrees,
                antenna_elevation_degrees: p.antenna_elevation_degrees,
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
pub extern "C" fn emane_rs_antennaprofile_event_free_deserialize(
    ptr: *mut EmaneRsAntennaProfile,
    len: usize,
) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[repr(C)]
pub struct EmaneRsCommEffect {
    pub nem_id: u32,
    pub latency_seconds: f32,
    pub jitter_seconds: f32,
    pub probability_loss: f32,
    pub probability_duplicate: f32,
    pub unicast_bit_rate_bps: u64,
    pub broadcast_bit_rate_bps: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_commeffect_event_serialize(
    items: *const EmaneRsCommEffect,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::CommEffectEvent::default();

    if num_items > 0 && !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, num_items) };
        for p in slice {
            msg.comm_effects
                .push(emane_message::comm_effect_event::CommEffect {
                    nem_id: p.nem_id,
                    latency_seconds: p.latency_seconds,
                    jitter_seconds: p.jitter_seconds,
                    probability_loss: p.probability_loss,
                    probability_duplicate: p.probability_duplicate,
                    unicast_bit_ratebps: p.unicast_bit_rate_bps,
                    broadcast_bit_ratebps: p.broadcast_bit_rate_bps,
                });
        }
    }

    let mut buf = Vec::with_capacity(msg.encoded_len());
    if msg.encode(&mut buf).is_ok() {
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
pub extern "C" fn emane_rs_commeffect_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_commeffect_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsCommEffect,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }

    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::CommEffectEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.comm_effects.len());
        for p in msg.comm_effects {
            vec.push(EmaneRsCommEffect {
                nem_id: p.nem_id,
                latency_seconds: p.latency_seconds,
                jitter_seconds: p.jitter_seconds,
                probability_loss: p.probability_loss,
                probability_duplicate: p.probability_duplicate,
                unicast_bit_rate_bps: p.unicast_bit_ratebps,
                broadcast_bit_rate_bps: p.broadcast_bit_ratebps,
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
pub extern "C" fn emane_rs_commeffect_event_free_deserialize(
    ptr: *mut EmaneRsCommEffect,
    len: usize,
) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[repr(C)]
pub struct EmaneRsFadingSelection {
    pub nem_id: u32,
    pub model: i32,
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingselection_event_serialize(
    items: *const EmaneRsFadingSelection,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::FadingSelectionEvent::default();

    if num_items > 0 && !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, num_items) };
        for p in slice {
            msg.entries
                .push(emane_message::fading_selection_event::Entry {
                    nem_id: p.nem_id,
                    model: p.model,
                });
        }
    }

    let mut buf = Vec::with_capacity(msg.encoded_len());
    if msg.encode(&mut buf).is_ok() {
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
pub extern "C" fn emane_rs_fadingselection_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingselection_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsFadingSelection,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }

    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::FadingSelectionEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.entries.len());
        for p in msg.entries {
            vec.push(EmaneRsFadingSelection {
                nem_id: p.nem_id,
                model: p.model,
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
pub extern "C" fn emane_rs_fadingselection_event_free_deserialize(
    ptr: *mut EmaneRsFadingSelection,
    len: usize,
) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

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
                    pathloss
                        .entries
                        .push(emane_message::pathloss_ex_event::pathloss::Entry {
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
pub extern "C" fn emane_rs_pathlossex_event_free_deserialize(
    ptr: *mut EmaneRsPathlossEx,
    len: usize,
) {
    if !ptr.is_null() {
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
        for p in slice.iter() {
            if !p.entries.is_null() && p.num_entries > 0 {
                unsafe {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        p.entries as *mut EmaneRsPathlossExEntry,
                        p.num_entries,
                    )));
                }
            }
        }
        unsafe {
            drop(Box::from_raw(slice));
        }
    }
}

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
            let loc = emane_message::location_event::Location {
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
            // required field, so we can access directly if the proto had it required (prost generated struct has it as T not Option<T> for required in proto2)
            // Wait, in prost, proto2 required is generated just like proto3 fields without Option unless specified. Actually, proto2 required generates `pub position: ::prost::alloc::boxed::Box<Position>` or something, or just `pub position: Position` if not boxed. Let's use `p.position` assuming it is generated as `pub position: ...`
            // Wait, for required message fields, prost generates `pub position: MessageType` or `Option<MessageType>`. Actually for message fields it always generates `Option<MessageType>`. We need to handle it.
            // Let's assume it's `Option<Position>`. If it's `Option<Position>`, we need to check if it's `Some`. If `None`, we can't deserialize correctly but proto2 guarantees it if parsed.
            // Wait, looking at `Position`, if it's `Option`, `p.position.unwrap_or_default()` works. Let's use `unwrap_or_default()`.
            // Wait, I don't know if prost generates it as Option. Yes, prost ALWAYS generates `Option<T>` for nested messages, regardless of `required` or `optional` in proto2.

            // Wait, if it generates `T` then `p.position` works. Let's just do `p.position` first, and if compilation fails we fix it.
            // Actually I'll use a match or `if let`.

            vec.push(EmaneRsLocation {
                nem_id: p.nem_id,
                position: if let Some(ref pos) = Some(p.position) {
                    EmaneRsPosition {
                        latitude_degrees: pos.latitude_degrees,
                        longitude_degrees: pos.longitude_degrees,
                        altitude_meters: pos.altitude_meters,
                    }
                } else {
                    EmaneRsPosition {
                        latitude_degrees: 0.0,
                        longitude_degrees: 0.0,
                        altitude_meters: 0.0,
                    }
                },
                has_velocity: p.velocity.is_some(),
                velocity: if let Some(ref vel) = p.velocity {
                    EmaneRsVelocity {
                        azimuth_degrees: vel.azimuth_degrees,
                        elevation_degrees: vel.elevation_degrees,
                        magnitude_meters_per_second: vel.magnitude_meters_per_second,
                    }
                } else {
                    EmaneRsVelocity {
                        azimuth_degrees: 0.0,
                        elevation_degrees: 0.0,
                        magnitude_meters_per_second: 0.0,
                    }
                },
                has_orientation: p.orientation.is_some(),
                orientation: if let Some(ref ori) = p.orientation {
                    EmaneRsOrientation {
                        roll_degrees: ori.roll_degrees,
                        pitch_degrees: ori.pitch_degrees,
                        yaw_degrees: ori.yaw_degrees,
                    }
                } else {
                    EmaneRsOrientation {
                        roll_degrees: 0.0,
                        pitch_degrees: 0.0,
                        yaw_degrees: 0.0,
                    }
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

#[repr(C)]
pub struct EmaneRsTdmaSlotTx {
    pub has_frequency_hz: bool,
    pub frequency_hz: u64,
    pub has_data_rate_bps: bool,
    pub data_rate_bps: u64,
    pub has_service_class: bool,
    pub service_class: u32,
    pub has_power_dbm: bool,
    pub power_dbm: f64,
    pub has_destination: bool,
    pub destination: u32,
}

#[repr(C)]
pub struct EmaneRsTdmaSlotRx {
    pub has_frequency_hz: bool,
    pub frequency_hz: u64,
}

#[repr(C)]
pub struct EmaneRsTdmaSlot {
    pub index: u32,
    pub type_: i32, // SLOT_TX = 1, SLOT_RX = 2, SLOT_IDLE = 3
    pub has_tx: bool,
    pub tx: EmaneRsTdmaSlotTx,
    pub has_rx: bool,
    pub rx: EmaneRsTdmaSlotRx,
}

#[repr(C)]
pub struct EmaneRsTdmaFrame {
    pub index: u32,
    pub has_frequency_hz: bool,
    pub frequency_hz: u64,
    pub has_data_rate_bps: bool,
    pub data_rate_bps: u64,
    pub has_service_class: bool,
    pub service_class: u32,
    pub has_power_dbm: bool,
    pub power_dbm: f64,
    pub slots: *mut EmaneRsTdmaSlot,
    pub num_slots: usize,
}

#[repr(C)]
pub struct EmaneRsTdmaStructure {
    pub slots_per_frame: u32,
    pub frames_per_multi_frame: u32,
    pub slot_duration_microseconds: u64,
    pub slot_overhead_microseconds: u64,
    pub bandwidth_hz: u64,
}

#[repr(C)]
pub struct EmaneRsTdmaSchedule {
    pub frames: *mut EmaneRsTdmaFrame,
    pub num_frames: usize,
    pub has_structure: bool,
    pub structure: EmaneRsTdmaStructure,
    pub has_frequency_hz: bool,
    pub frequency_hz: u64,
    pub has_data_rate_bps: bool,
    pub data_rate_bps: u64,
    pub has_service_class: bool,
    pub service_class: u32,
    pub has_power_dbm: bool,
    pub power_dbm: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_tdmaschedule_event_deserialize(
    buf: *const u8,
    len: usize,
    out_msg: *mut *mut EmaneRsTdmaSchedule,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }

    let slice = unsafe { std::slice::from_raw_parts(buf, len) };
    if let Ok(msg) = <emane_message::TdmaScheduleEvent as prost::Message>::decode(slice) {
        let mut frames_vec = Vec::with_capacity(msg.frames.len());
        for f in msg.frames {
            let mut slots_vec = Vec::with_capacity(f.slots.len());
            for s in f.slots {
                let has_tx = s.tx.is_some();
                let tx_msg = s.tx.unwrap_or_default();
                let tx = EmaneRsTdmaSlotTx {
                    has_frequency_hz: tx_msg.frequency_hz.is_some(),
                    frequency_hz: tx_msg.frequency_hz.unwrap_or(0),
                    has_data_rate_bps: tx_msg.data_ratebps.is_some(),
                    data_rate_bps: tx_msg.data_ratebps.unwrap_or(0),
                    has_service_class: tx_msg.service_class.is_some(),
                    service_class: tx_msg.service_class.unwrap_or(0),
                    has_power_dbm: tx_msg.powerd_bm.is_some(),
                    power_dbm: tx_msg.powerd_bm.unwrap_or(0.0),
                    has_destination: tx_msg.destination.is_some(),
                    destination: tx_msg.destination.unwrap_or(0),
                };

                let has_rx = s.rx.is_some();
                let rx_msg = s.rx.unwrap_or_default();
                let rx = EmaneRsTdmaSlotRx {
                    has_frequency_hz: rx_msg.frequency_hz.is_some(),
                    frequency_hz: rx_msg.frequency_hz.unwrap_or(0),
                };

                slots_vec.push(EmaneRsTdmaSlot {
                    index: s.index,
                    type_: s.r#type,
                    has_tx,
                    tx,
                    has_rx,
                    rx,
                });
            }

            let mut slots_boxed = slots_vec.into_boxed_slice();
            let slots_ptr = slots_boxed.as_mut_ptr();
            let num_slots = slots_boxed.len();
            std::mem::forget(slots_boxed);

            frames_vec.push(EmaneRsTdmaFrame {
                index: f.index,
                has_frequency_hz: f.frequency_hz.is_some(),
                frequency_hz: f.frequency_hz.unwrap_or(0),
                has_data_rate_bps: f.data_ratebps.is_some(),
                data_rate_bps: f.data_ratebps.unwrap_or(0),
                has_service_class: f.service_class.is_some(),
                service_class: f.service_class.unwrap_or(0),
                has_power_dbm: f.powerd_bm.is_some(),
                power_dbm: f.powerd_bm.unwrap_or(0.0),
                slots: slots_ptr,
                num_slots,
            });
        }

        let mut frames_boxed = frames_vec.into_boxed_slice();
        let frames_ptr = frames_boxed.as_mut_ptr();
        let num_frames = frames_boxed.len();
        std::mem::forget(frames_boxed);

        let has_structure = msg.structure.is_some();
        let struct_msg = msg.structure.unwrap_or_default();
        let structure = EmaneRsTdmaStructure {
            slots_per_frame: struct_msg.slots_per_frame,
            frames_per_multi_frame: struct_msg.frames_per_multi_frame,
            slot_duration_microseconds: struct_msg.slot_duration_microseconds,
            slot_overhead_microseconds: struct_msg.slot_overhead_microseconds,
            bandwidth_hz: struct_msg.bandwidth_hz,
        };

        let schedule = Box::new(EmaneRsTdmaSchedule {
            frames: frames_ptr,
            num_frames,
            has_structure,
            structure,
            has_frequency_hz: msg.frequency_hz.is_some(),
            frequency_hz: msg.frequency_hz.unwrap_or(0),
            has_data_rate_bps: msg.data_ratebps.is_some(),
            data_rate_bps: msg.data_ratebps.unwrap_or(0),
            has_service_class: msg.service_class.is_some(),
            service_class: msg.service_class.unwrap_or(0),
            has_power_dbm: msg.powerd_bm.is_some(),
            power_dbm: msg.powerd_bm.unwrap_or(0.0),
        });

        unsafe { *out_msg = Box::into_raw(schedule) };
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdmaschedule_event_free_deserialize(ptr: *mut EmaneRsTdmaSchedule) {
    if !ptr.is_null() {
        let schedule = unsafe { Box::from_raw(ptr) };
        if !schedule.frames.is_null() && schedule.num_frames > 0 {
            let frames =
                unsafe { std::slice::from_raw_parts_mut(schedule.frames, schedule.num_frames) };
            for f in frames {
                if !f.slots.is_null() && f.num_slots > 0 {
                    unsafe {
                        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                            f.slots,
                            f.num_slots,
                        )));
                    }
                }
            }
            unsafe {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    schedule.frames,
                    schedule.num_frames,
                )));
            }
        }
    }
}
