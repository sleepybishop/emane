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
