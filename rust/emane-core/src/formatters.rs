use std::os::raw::c_char;
use std::ffi::CString;

pub type AddStringCallback = extern "C" fn(*mut std::os::raw::c_void, *const c_char);

#[no_mangle]
pub extern "C" fn emane_rs_format_orientation(
    pitch: f64,
    roll: f64,
    yaw: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new("orientation:").unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("pitch: {}", pitch)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("roll: {}", roll)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("yaw: {}", yaw)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_position(
    lat: f64,
    lon: f64,
    alt: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new("position:").unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("lat: {}", lat)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("lon: {}", lon)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("alt: {}", alt)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_position_neu(
    north: f64,
    east: f64,
    up: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new("position NEU:").unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("north: {}", north)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("east: {}", east)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("up: {}", up)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_velocity(
    az: f64,
    el: f64,
    mag: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new("velocity:").unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("az: {}", az)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("el: {}", el)).unwrap();
    add_string(ctx, s.as_ptr());

    let s = CString::new(format!("mag: {}", mag)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_antenna_profile_element(
    nem_id: u16,
    profile_id: u16,
    azimuth: f64,
    elevation: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!(
        "nem: {} profile: {} antenna az: {} antenna el: {}",
        nem_id, profile_id, azimuth, elevation
    )).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_comm_effect_element(
    nem_id: u16,
    latency_sec: f64,
    jitter_sec: f64,
    prob_loss: f32,
    prob_dup: f32,
    unicast_bps: u64,
    broadcast_bps: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!(
        "nem: {} latency: {} jitter: {} loss: {} dup: {} unicast bps: {} broadcast bps: {}",
        nem_id, latency_sec, jitter_sec, prob_loss, prob_dup, unicast_bps, broadcast_bps
    )).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_fading_selection_element(
    nem_id: u16,
    fading_model: i32, // Events::FadingModel
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let model_str = match fading_model {
        0 => "none", // NONE
        1 => "nakagami", // NAKAGAMI
        2 => "lognormal", // LOGNORMAL
        _ => "unknown",
    };
    
    let s = CString::new(format!(
        "nem: {} model: {}",
        nem_id, model_str
    )).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_pathloss_element(
    nem_id: u16,
    fwd_pathloss: f32,
    rev_pathloss: f32,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!(
        "nem: {} fwd pathloss: {} rev pathloss: {}",
        nem_id, fwd_pathloss, rev_pathloss
    )).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_pathloss_ex_start(
    nem_id: u16,
) -> *mut String {
    Box::into_raw(Box::new(format!("nem: {}", nem_id)))
}

#[no_mangle]
pub extern "C" fn emane_rs_format_pathloss_ex_append(
    ptr: *mut String,
    freq: u64,
    pathloss: f32,
) {
    if !ptr.is_null() {
        let s = unsafe { &mut *ptr };
        use std::fmt::Write;
        write!(s, " {}:{}", freq, pathloss).unwrap();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_format_pathloss_ex_end(
    ptr: *mut String,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    if !ptr.is_null() {
        let s = unsafe { Box::from_raw(ptr) };
        if let Ok(c_str) = CString::new(*s) {
            add_string(ctx, c_str.as_ptr());
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_format_foi_bandwidth(
    bandwidth: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!("bandwidth: {}", bandwidth)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_foi_freq(
    freq: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!("freq: {}", freq)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_frequency_control_segment(
    freq: u64,
    duration: i64,
    offset: i64,
    power: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("freq: {}", freq)).unwrap();
    let s2 = CString::new(format!("duration: {}", duration)).unwrap();
    let s3 = CString::new(format!("offset: {}", offset)).unwrap();
    let s4 = CString::new(format!("power: {}", power)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_spectrum_filter_add(
    filter_index: u16,
    antenna_index: u16,
    freq: u64,
    bandwidth: u64,
    subband_bin_size: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("filter idex: {}", filter_index)).unwrap();
    let s2 = CString::new(format!("antenna idex: {}", antenna_index)).unwrap();
    let s3 = CString::new(format!("frequency: {}", freq)).unwrap();
    let s4 = CString::new(format!("bandwidth: {}", bandwidth)).unwrap();
    let s5 = CString::new(format!("subband bin size: {}", subband_bin_size)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
    add_string(ctx, s5.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_rx_antenna_remove(
    antenna_index: u16,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!("anntena id: {}", antenna_index)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_spectrum_filter_remove(
    filter_index: u16,
    antenna_index: u16,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("filter index: {}", filter_index)).unwrap();
    let s2 = CString::new(format!("antenna index: {}", antenna_index)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_rx_antenna_add_info(
    antenna_index: u16,
    gain: f64,
    gain_b: bool,
    azimuth: f64,
    elevation: f64,
    profile_id: u16,
    pointing_b: bool,
    bandwidth: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("anntena id: {}", antenna_index)).unwrap();
    let s2 = CString::new(format!("fixed gain: {}/{}", gain, gain_b as u8)).unwrap();
    let s3 = CString::new(format!("az: {}", azimuth)).unwrap();
    let s4 = CString::new(format!("el: {}/{}", elevation, pointing_b as u8)).unwrap();
    let s5 = CString::new(format!("profile: {}", profile_id)).unwrap();
    let s6 = CString::new(format!("bandwidth: {}", bandwidth)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
    add_string(ctx, s5.as_ptr());
    add_string(ctx, s6.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_rx_antenna_add_freq(
    freq: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s = CString::new(format!("freq: {}", freq)).unwrap();
    add_string(ctx, s.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_rx_antenna_update_info(
    antenna_index: u16,
    gain: f64,
    gain_b: bool,
    azimuth: f64,
    elevation: f64,
    pointing_b: bool,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("anntena id: {}", antenna_index)).unwrap();
    let s2 = CString::new(format!("fixed gain: {}/{}", gain, gain_b as u8)).unwrap();
    let s3 = CString::new(format!("az: {}", azimuth)).unwrap();
    let s4 = CString::new(format!("el: {}/{}", elevation, pointing_b as u8)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_receive_properties(
    sot: f64,
    span: i64,
    prop_delay: i64,
    rx_sens: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("sot: {}", sot)).unwrap();
    let s2 = CString::new(format!("span: {}", span)).unwrap();
    let s3 = CString::new(format!("prop delay: {}", prop_delay)).unwrap();
    let s4 = CString::new(format!("receiver sensitivity: {}", rx_sens)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_r2ri_self_metric(
    broadcast_bps: u64,
    max_bps: u64,
    interval: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("broadcast data rate: {}", broadcast_bps)).unwrap();
    let s2 = CString::new(format!("max data rate: {}", max_bps)).unwrap();
    let s3 = CString::new(format!("report interval: {}", interval)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_transmitter_control(
    nem_id: u16,
    power: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("src: {}", nem_id)).unwrap();
    let s2 = CString::new(format!("power: {}", power)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_r2ri_queue_metric(
    queue_id: u8,
    max_size: u32,
    current_depth: u32,
    num_discards: u32,
    avg_delay: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("queue: {}", queue_id)).unwrap();
    let s2 = CString::new(format!("max size: {}", max_size)).unwrap();
    let s3 = CString::new(format!("current size: {}", current_depth)).unwrap();
    let s4 = CString::new(format!("num discards size: {}", num_discards)).unwrap();
    let s5 = CString::new(format!("avg delay: {}", avg_delay)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
    add_string(ctx, s5.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_r2ri_neighbor_metric(
    nem_id: u16,
    num_rx_frames: u64,
    num_tx_frames: u64,
    num_missed_frames: u64,
    bandwidth_consumption: f64,
    sinr_avg: f32,
    sinr_stdv: f32,
    noise_floor_avg: f32,
    noise_floor_stdv: f32,
    rx_avg_data_rate: u64,
    tx_avg_data_rate: u64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("nem: {}", nem_id)).unwrap();
    let s2 = CString::new(format!("rx frames: {}", num_rx_frames)).unwrap();
    let s3 = CString::new(format!("tx frames: {}", num_tx_frames)).unwrap();
    let s4 = CString::new(format!("missed frames: {}", num_missed_frames)).unwrap();
    let s5 = CString::new(format!("bandwidth consumptions: {}", bandwidth_consumption)).unwrap();
    let s6 = CString::new(format!("sinr avg: {}", sinr_avg)).unwrap();
    let s7 = CString::new(format!("sinr stdv: {}", sinr_stdv)).unwrap();
    let s8 = CString::new(format!("noise floor avg: {}", noise_floor_avg)).unwrap();
    let s9 = CString::new(format!("noise floor stdv: {}", noise_floor_stdv)).unwrap();
    let s10 = CString::new(format!("rx data rate avg: {}", rx_avg_data_rate)).unwrap();
    let s11 = CString::new(format!("tx data rate avg: {}", tx_avg_data_rate)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
    add_string(ctx, s5.as_ptr());
    add_string(ctx, s6.as_ptr());
    add_string(ctx, s7.as_ptr());
    add_string(ctx, s8.as_ptr());
    add_string(ctx, s9.as_ptr());
    add_string(ctx, s10.as_ptr());
    add_string(ctx, s11.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_mimo_receive_properties(
    sot: f64,
    prop_delay: i64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("sot: {}", sot)).unwrap();
    let s2 = CString::new(format!("prop delay: {}", prop_delay)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_mimo_receive_antenna_info(
    rx_antenna_index: u16,
    tx_antenna_index: u16,
    span: i64,
    rx_sens: f64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("rx antenna index: {}", rx_antenna_index)).unwrap();
    let s2 = CString::new(format!("tx antenna index: {}", tx_antenna_index)).unwrap();
    let s3 = CString::new(format!("span: {}", span)).unwrap();
    let s4 = CString::new(format!("rx sensitivity: {}", rx_sens)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
    add_string(ctx, s3.as_ptr());
    add_string(ctx, s4.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_rs_format_mimo_doppler_shift(
    freq: u64,
    shift: i64,
    ctx: *mut std::os::raw::c_void,
    add_string: AddStringCallback,
) {
    let s1 = CString::new(format!("shift freq: {}", freq)).unwrap();
    let s2 = CString::new(format!("shift hz: {}", shift)).unwrap();
    add_string(ctx, s1.as_ptr());
    add_string(ctx, s2.as_ptr());
}
