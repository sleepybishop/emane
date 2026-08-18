use std::os::raw::c_void;

#[derive(Clone)]
pub struct R2riQueueMetric {
    pub queue_id: u32,
    pub max_size: u32,
    pub current_depth: u32,
    pub num_discards: u32,
    pub avg_delay_microsec: u64,
}

#[derive(Clone)]
pub struct R2riQueueMetricControlMessage {
    pub metrics: Vec<R2riQueueMetric>,
}

impl R2riQueueMetricControlMessage {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_r2ri_queue_metric_create() -> *mut c_void {
    let msg = Box::new(R2riQueueMetricControlMessage::new());
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_r2ri_queue_metric_add_metric(
    ptr: *mut c_void,
    queue_id: u32,
    max_size: u32,
    current_depth: u32,
    num_discards: u32,
    avg_delay_microsec: u64,
) {
    let msg = &mut *(ptr as *mut R2riQueueMetricControlMessage);
    msg.metrics.push(R2riQueueMetric {
        queue_id,
        max_size,
        current_depth,
        num_discards,
        avg_delay_microsec,
    });
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_r2ri_queue_metric_clone(
    ptr: *const c_void,
) -> *mut c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = &*(ptr as *const R2riQueueMetricControlMessage);
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_r2ri_queue_metric_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut R2riQueueMetricControlMessage));
    }
}
