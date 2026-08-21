use std::collections::{HashMap, HashSet};
use std::os::raw::c_void;

pub type NEMId = u16;
pub type Microseconds = u64; // Or Duration

#[derive(Clone, Default)]
struct Utilization {
    total_utilization_microseconds: Microseconds,
    total_num_packets: usize,
    f_total_rx_power_milli_watts: f32,
}

impl Utilization {
    fn reset(&mut self) {
        self.total_utilization_microseconds = 0;
        self.total_num_packets = 0;
        self.f_total_rx_power_milli_watts = 0.0;
    }

    fn update(
        &mut self,
        num_packets: usize,
        utilization_microseconds: Microseconds,
        f_rx_power_milli_watts: f32,
    ) {
        self.total_num_packets += num_packets;
        self.total_utilization_microseconds += utilization_microseconds;
        self.f_total_rx_power_milli_watts += f_rx_power_milli_watts;
    }
}

pub struct NeighborEntry {
    last_activity_time: u64,
    prev_utilization_type_map: HashMap<u8, Utilization>,
    curr_utilization_type_map: HashMap<u8, Utilization>,
    one_hop_nbr_set: HashSet<NEMId>,
    common_nbr_set: HashSet<NEMId>,
    hidden_nbr_set: HashSet<NEMId>,
    f_estimated_num_common_neighbors: f32,
    f_hidden_channel_activity: f32,
    f_average_hidden_rx_power_milli_watts: f32,
    f_average_common_rx_power_milli_watts: f32,
}

impl NeighborEntry {
    pub fn new() -> Self {
        let mut prev = HashMap::new();
        let mut curr = HashMap::new();
        for &msg_type in &[1, 2, 4, 8] {
            // MSG_TYPE_BROADCAST_DATA etc
            prev.insert(msg_type, Utilization::default());
            curr.insert(msg_type, Utilization::default());
        }
        Self {
            last_activity_time: 0,
            prev_utilization_type_map: prev,
            curr_utilization_type_map: curr,
            one_hop_nbr_set: HashSet::new(),
            common_nbr_set: HashSet::new(),
            hidden_nbr_set: HashSet::new(),
            f_estimated_num_common_neighbors: 0.0,
            f_hidden_channel_activity: 0.0,
            f_average_hidden_rx_power_milli_watts: 0.0,
            f_average_common_rx_power_milli_watts: 0.0,
        }
    }

    pub fn update_channel_activity(
        &mut self,
        duration: Microseconds,
        msg_type: u8,
        activity_time: u64,
        f_rx_power: f32,
        num_packets: usize,
    ) {
        if let Some(util) = self.curr_utilization_type_map.get_mut(&msg_type) {
            util.update(num_packets, duration, f_rx_power);
        }
        self.last_activity_time = activity_time;
    }

    pub fn store_utilization(&mut self) {
        self.prev_utilization_type_map = self.curr_utilization_type_map.clone();
        for util in self.curr_utilization_type_map.values_mut() {
            util.reset();
        }
    }

    pub fn get_utilization_microseconds(&self, msg_type_mask: u8) -> Microseconds {
        let mut result = 0;
        for (&k, v) in &self.prev_utilization_type_map {
            if (k & msg_type_mask) != 0 {
                result += v.total_utilization_microseconds;
            }
        }
        result
    }
}

pub struct Neighbor2HopEntry {
    last_activity_time: u64,
    utilization: Utilization,
}

impl Neighbor2HopEntry {
    pub fn new() -> Self {
        Self {
            last_activity_time: 0,
            utilization: Utilization::default(),
        }
    }

    pub fn update_channel_activity(
        &mut self,
        duration: Microseconds,
        activity_time: u64,
        num_packets: usize,
    ) {
        self.utilization.update(num_packets, duration, 0.0);
        self.last_activity_time = activity_time;
    }

    pub fn reset_utilization(&mut self) {
        self.utilization.reset();
    }
}

pub struct FfiNeighborManager {
    id: u16,
    platform_service: *mut c_void,
    mac_layer: *mut c_void,
    one_hop_nbr_map: HashMap<NEMId, NeighborEntry>,
    two_hop_nbr_map: HashMap<NEMId, Neighbor2HopEntry>,
    nbr_time_out_microseconds: u64,
    f_estimated_num_one_hop_neighbors: f32,
    f_estimated_num_two_hop_neighbors: f32,
    f_local_node_tx: f32,
    total_one_hop_utilization_microseconds: i64,
    total_two_hop_utilization_microseconds: i64,
    average_message_duration_microseconds: i64,
    f_average_rx_power_per_message_milli_watts: f32,
}

impl FfiNeighborManager {
    pub fn new(id: u16, platform_service: *mut c_void, mac_layer: *mut c_void) -> Self {
        Self {
            id,
            platform_service,
            mac_layer,
            one_hop_nbr_map: HashMap::new(),
            two_hop_nbr_map: HashMap::new(),
            nbr_time_out_microseconds: 0,
            f_estimated_num_one_hop_neighbors: 0.0,
            f_estimated_num_two_hop_neighbors: 0.0,
            f_local_node_tx: 0.0,
            total_one_hop_utilization_microseconds: 0,
            total_two_hop_utilization_microseconds: 0,
            average_message_duration_microseconds: 0,
            f_average_rx_power_per_message_milli_watts: 0.0,
        }
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_create(
    id: u16,
    platform_service: *mut c_void,
    mac_layer: *mut c_void,
) -> *mut FfiNeighborManager {
    Box::into_raw(Box::new(FfiNeighborManager::new(
        id,
        platform_service,
        mac_layer,
    )))
}

#[no_mangle]
pub extern "C" fn NeighborManager_destroy(ptr: *mut FfiNeighborManager) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_updateDataChannelActivity(
    ptr: *mut FfiNeighborManager,
    src: u16,
    msg_type: u8,
    f_rx_power: f32,
    time_point: i64,
    duration: i64,
    _category: u8,
) {
    if let Some(mgr) = unsafe { ptr.as_mut() } {
        let entry = mgr
            .one_hop_nbr_map
            .entry(src)
            .or_insert_with(NeighborEntry::new);
        entry.update_channel_activity(duration as u64, msg_type, time_point as u64, f_rx_power, 1);
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_updateCtrlChannelActivity(
    ptr: *mut FfiNeighborManager,
    src: u16,
    origin: u16,
    msg_type: u8,
    f_rx_power: f32,
    time_point: i64,
    duration: i64,
    _category: u8,
) {
    if let Some(mgr) = unsafe { ptr.as_mut() } {
        if origin != mgr.id {
            if !mgr.one_hop_nbr_map.contains_key(&origin) {
                let entry2 = mgr
                    .two_hop_nbr_map
                    .entry(origin)
                    .or_insert_with(Neighbor2HopEntry::new);
                entry2.update_channel_activity(duration as u64, time_point as u64, 1);
            }
        }
        let entry = mgr
            .one_hop_nbr_map
            .entry(src)
            .or_insert_with(NeighborEntry::new);
        entry.update_channel_activity(0, msg_type, time_point as u64, f_rx_power, 1);
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_handleOneHopNeighborsEvent(
    _ptr: *mut FfiNeighborManager,
    _arg0: *const c_void,
) {
    // simplified
}

#[no_mangle]
pub extern "C" fn NeighborManager_start(_ptr: *mut FfiNeighborManager) {}

#[no_mangle]
pub extern "C" fn NeighborManager_resetStatistics(ptr: *mut FfiNeighborManager) {
    if let Some(mgr) = unsafe { ptr.as_mut() } {
        for entry in mgr.one_hop_nbr_map.values_mut() {
            entry.store_utilization();
        }
        for entry in mgr.two_hop_nbr_map.values_mut() {
            entry.reset_utilization();
        }
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_setNeighborTimeoutMicroseconds(
    ptr: *mut FfiNeighborManager,
    to: i64,
) {
    if let Some(mgr) = unsafe { ptr.as_mut() } {
        mgr.nbr_time_out_microseconds = to as u64;
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getNumberOfEstimatedOneHopNeighbors(
    ptr: *mut FfiNeighborManager,
) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.f_estimated_num_one_hop_neighbors
    } else {
        0.0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getNumberOfEstimatedTwoHopNeighbors(
    ptr: *mut FfiNeighborManager,
) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.f_estimated_num_two_hop_neighbors
    } else {
        0.0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getHiddenChannelActivity(
    ptr: *mut FfiNeighborManager,
    src: u16,
) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        if let Some(entry) = mgr.one_hop_nbr_map.get(&src) {
            return entry.f_hidden_channel_activity;
        }
    }
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getNumberOfEstimatedCommonNeighbors(
    ptr: *mut FfiNeighborManager,
    src: u16,
) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        if let Some(entry) = mgr.one_hop_nbr_map.get(&src) {
            return entry.f_estimated_num_common_neighbors;
        }
    }
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getNumberOfEstimatedHiddenNeighbors(
    ptr: *mut FfiNeighborManager,
    src: u16,
) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        if let Some(entry) = mgr.one_hop_nbr_map.get(&src) {
            let res =
                mgr.f_estimated_num_one_hop_neighbors - entry.f_estimated_num_common_neighbors;
            return if res < 0.0 { 0.0 } else { res };
        }
    }
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getLocalNodeTx(ptr: *mut FfiNeighborManager) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.f_local_node_tx
    } else {
        0.0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getTotalActiveOneHopNeighbors(
    ptr: *mut FfiNeighborManager,
) -> usize {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.one_hop_nbr_map.len()
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_setCategories(_ptr: *mut FfiNeighborManager, _arg0: u8) {}

#[no_mangle]
pub extern "C" fn NeighborManager_getTotalOneHopUtilizationMicroseconds(
    ptr: *mut FfiNeighborManager,
) -> i64 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.total_one_hop_utilization_microseconds
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getTotalTwoHopUtilizationMicroseconds(
    ptr: *mut FfiNeighborManager,
) -> i64 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.total_two_hop_utilization_microseconds
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getAverageMessageDurationMicroseconds(
    ptr: *mut FfiNeighborManager,
) -> i64 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.average_message_duration_microseconds
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getAllUtilizationMicroseconds(
    ptr: *mut FfiNeighborManager,
    src: u16,
) -> i64 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        if let Some(entry) = mgr.one_hop_nbr_map.get(&src) {
            return entry.get_utilization_microseconds(0xFF) as i64;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getAverageRxPowerPerMessageMilliWatts(
    ptr: *mut FfiNeighborManager,
) -> f32 {
    if let Some(mgr) = unsafe { ptr.as_ref() } {
        mgr.f_average_rx_power_per_message_milli_watts
    } else {
        0.0
    }
}

#[no_mangle]
pub extern "C" fn NeighborManager_getAverageRxPowerPerMessageHiddenNodesMilliWatts(
    _ptr: *mut FfiNeighborManager,
) -> f32 {
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getAverageRxPowerPerMessageCommonNodesMilliWatts(
    _ptr: *mut FfiNeighborManager,
) -> f32 {
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getRandomRxPowerCommonNodesMilliWatts(
    _ptr: *mut FfiNeighborManager,
    _arg0: u16,
) -> f32 {
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getRandomRxPowerHiddenNodesMilliWatts(
    _ptr: *mut FfiNeighborManager,
    _arg0: u16,
) -> f32 {
    0.0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getLastOneHopNbrListTxTime(_ptr: *mut FfiNeighborManager) -> i64 {
    0
}

#[no_mangle]
pub extern "C" fn NeighborManager_getUtilizationRatios(
    _ptr: *mut FfiNeighborManager,
) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn NeighborManager_registerStatistics(
    _ptr: *mut FfiNeighborManager,
    _arg0: *mut c_void,
) {
}
