use std::os::raw::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct R2riNeighborMetricC {
    pub id: u16,
    pub num_rx_frames: u64,
    pub num_tx_frames: u64,
    pub num_missed_frames: u64,
    pub bandwidth_consumption_microsec: i64,
    pub sinr_avg_dbm: f32,
    pub sinr_stddev: f32,
    pub noise_floor_avg_dbm: f32,
    pub noise_floor_stddev: f32,
    pub rx_avg_data_rate_bps: u64,
    pub tx_avg_data_rate_bps: u64,
}

#[derive(Clone)]
pub struct R2riNeighborMetricControlMessage {
    pub metrics: Vec<R2riNeighborMetricC>,
}

#[no_mangle]
pub unsafe extern "C" fn emane_r2ri_neighbor_metric_control_message_create() -> *mut c_void {
    let msg = Box::new(R2riNeighborMetricControlMessage {
        metrics: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_r2ri_neighbor_metric_control_message_add_metric(
    msg_ptr: *mut c_void,
    metric: *const R2riNeighborMetricC,
) {
    if msg_ptr.is_null() || metric.is_null() {
        return;
    }
    let msg = &mut *(msg_ptr as *mut R2riNeighborMetricControlMessage);
    msg.metrics.push(*metric);
}

#[no_mangle]
pub unsafe extern "C" fn emane_r2ri_neighbor_metric_control_message_clone(
    msg_ptr: *mut c_void,
) -> *mut c_void {
    if msg_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = &*(msg_ptr as *const R2riNeighborMetricControlMessage);
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_r2ri_neighbor_metric_control_message_destroy(msg_ptr: *mut c_void) {
    if !msg_ptr.is_null() {
        drop(Box::from_raw(
            msg_ptr as *mut R2riNeighborMetricControlMessage,
        ));
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_r2ri_neighbor_metric_control_message_get_metric_count(
    msg_ptr: *mut c_void,
) -> usize {
    if msg_ptr.is_null() {
        return 0;
    }
    let msg = &*(msg_ptr as *const R2riNeighborMetricControlMessage);
    msg.metrics.len()
}

#[no_mangle]
pub unsafe extern "C" fn emane_r2ri_neighbor_metric_control_message_get_metric(
    msg_ptr: *mut c_void,
    index: usize,
    metric_out: *mut R2riNeighborMetricC,
) {
    if msg_ptr.is_null() || metric_out.is_null() {
        return;
    }
    let msg = &*(msg_ptr as *const R2riNeighborMetricControlMessage);
    if index < msg.metrics.len() {
        *metric_out = msg.metrics[index];
    }
}
