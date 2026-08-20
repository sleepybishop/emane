#[derive(Default)]
pub struct TransponderNoProtocol {
    eot_us: u64,
    tx_opportunity_index: u64,
}

#[no_mangle] pub extern "C" fn rust_bentpipe_tnp_new() -> *mut TransponderNoProtocol { Box::into_raw(Box::new(TransponderNoProtocol::default())) }
#[no_mangle] pub extern "C" fn rust_bentpipe_tnp_free(ptr: *mut TransponderNoProtocol) { if !ptr.is_null() { unsafe { drop(Box::from_raw(ptr)); } } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tnp_start(_ptr: *mut TransponderNoProtocol) {}
#[no_mangle] pub extern "C" fn rust_bentpipe_tnp_stop(_ptr: *mut TransponderNoProtocol) {}
#[no_mangle] pub extern "C" fn rust_bentpipe_tnp_is_tx_opp(ptr: *mut TransponderNoProtocol, now_us: u64) -> bool { unsafe { now_us >= (*ptr).eot_us } }

#[no_mangle]
pub extern "C" fn rust_bentpipe_tnp_prepare_tx(
    ptr: *mut TransponderNoProtocol, 
    now_us: u64, 
    len: usize, 
    rate: u64, 
    out_duration: *mut u64, 
    out_idx: *mut u64
) {
    let tnp = unsafe { &mut *ptr };
    let duration = ((len as f64 * 8.0) / (rate as f64) * 1_000_000.0) as u64; // assuming microseconds
    tnp.tx_opportunity_index += 1;
    tnp.eot_us = now_us + duration;
    unsafe {
        *out_duration = duration;
        *out_idx = tnp.tx_opportunity_index;
    }
}
