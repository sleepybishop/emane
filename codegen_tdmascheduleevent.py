rust_code = """
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
                    has_data_rate_bps: tx_msg.data_rate_bps.is_some(),
                    data_rate_bps: tx_msg.data_rate_bps.unwrap_or(0),
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
                has_data_rate_bps: f.data_rate_bps.is_some(),
                data_rate_bps: f.data_rate_bps.unwrap_or(0),
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
            has_data_rate_bps: msg.data_rate_bps.is_some(),
            data_rate_bps: msg.data_rate_bps.unwrap_or(0),
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
            let frames = unsafe { std::slice::from_raw_parts_mut(schedule.frames, schedule.num_frames) };
            for f in frames {
                if !f.slots.is_null() && f.num_slots > 0 {
                    unsafe {
                        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(f.slots, f.num_slots)));
                    }
                }
            }
            unsafe {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(schedule.frames, schedule.num_frames)));
            }
        }
    }
}
"""

with open("rust/emane-core/src/events.rs", "a") as f:
    f.write(rust_code)

