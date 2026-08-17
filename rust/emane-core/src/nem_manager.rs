use std::collections::HashMap;
use std::os::raw::{c_char, c_void};
use std::ffi::{CStr, CString};

// C callbacks
extern "C" {
    fn emane_c_nem_start(nem_ptr: *mut c_void);
    fn emane_c_nem_post_start(nem_ptr: *mut c_void);
    fn emane_c_nem_stop(nem_ptr: *mut c_void);
    fn emane_c_nem_destroy(nem_ptr: *mut c_void);
    fn emane_c_control_port_open(port_str: *const c_char);
    fn emane_c_control_port_close();
    fn emane_c_load_antenna_profile(uri: *const c_char);
    fn emane_c_load_spectral_mask(uri: *const c_char);
}

pub struct NemManager {
    uuid: [u8; 16],
    nems: HashMap<u16, *mut c_void>,
    
    ota_manager_group_addr: String,
    ota_manager_group_device: String,
    ota_manager_ttl: u8,
    ota_manager_mtu: u32,
    ota_manager_part_check_threshold: u16,
    ota_manager_part_timeout_threshold: u16,
    ota_manager_loopback: bool,
    ota_manager_channel_enable: bool,
    
    event_service_group_addr: String,
    event_service_device: String,
    event_service_ttl: u8,
    
    control_port_addr: String,
    
    antenna_profile_manifest_uri: String,
    spectral_mask_manifest_uri: String,
}

impl NemManager {
    pub fn new(uuid: [u8; 16]) -> Self {
        Self {
            uuid,
            nems: HashMap::new(),
            ota_manager_group_addr: String::new(),
            ota_manager_group_device: String::new(),
            ota_manager_ttl: 1,
            ota_manager_mtu: 0,
            ota_manager_part_check_threshold: 2,
            ota_manager_part_timeout_threshold: 5,
            ota_manager_loopback: false,
            ota_manager_channel_enable: true,
            
            event_service_group_addr: String::new(),
            event_service_device: String::new(),
            event_service_ttl: 1,
            
            control_port_addr: "0.0.0.0:47000".to_string(),
            
            antenna_profile_manifest_uri: String::new(),
            spectral_mask_manifest_uri: String::new(),
        }
    }

    pub fn add(&mut self, id: u16, nem_ptr: *mut c_void) {
        self.nems.insert(id, nem_ptr);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_create(uuid_ptr: *const u8) -> *mut c_void {
    let mut uuid = [0u8; 16];
    if !uuid_ptr.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(uuid_ptr, uuid.as_mut_ptr(), 16);
        }
    }
    let manager = Box::new(NemManager::new(uuid));
    Box::into_raw(manager) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_destroy_manager(manager_ptr: *mut c_void) {
    if !manager_ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(manager_ptr as *mut NemManager);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_add(manager_ptr: *mut c_void, nem_id: u16, nem_ptr: *mut c_void) {
    if manager_ptr.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    manager.add(nem_id, nem_ptr);
}

// Config callbacks
#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_set_config_str(
    manager_ptr: *mut c_void,
    key: *const c_char,
    value: *const c_char,
) {
    if manager_ptr.is_null() || key.is_null() || value.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    let k = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    let v = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() };
    
    match k.as_str() {
        "otamanagergroup" => manager.ota_manager_group_addr = v,
        "otamanagerdevice" => manager.ota_manager_group_device = v,
        "eventservicegroup" => manager.event_service_group_addr = v,
        "eventservicedevice" => manager.event_service_device = v,
        "controlportendpoint" => manager.control_port_addr = v,
        "antennaprofilemanifesturi" => manager.antenna_profile_manifest_uri = v,
        "spectralmaskmanifesturi" => manager.spectral_mask_manifest_uri = v,
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_set_config_u8(
    manager_ptr: *mut c_void,
    key: *const c_char,
    value: u8,
) {
    if manager_ptr.is_null() || key.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    let k = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    
    match k.as_str() {
        "otamanagerttl" => manager.ota_manager_ttl = value,
        "eventservicettl" => manager.event_service_ttl = value,
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_set_config_u16(
    manager_ptr: *mut c_void,
    key: *const c_char,
    value: u16,
) {
    if manager_ptr.is_null() || key.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    let k = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    
    match k.as_str() {
        "otamanagerpartcheckthreshold" => manager.ota_manager_part_check_threshold = value,
        "otamanagerparttimeoutthreshold" => manager.ota_manager_part_timeout_threshold = value,
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_set_config_u32(
    manager_ptr: *mut c_void,
    key: *const c_char,
    value: u32,
) {
    if manager_ptr.is_null() || key.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    let k = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    
    match k.as_str() {
        "otamanagermtu" => manager.ota_manager_mtu = value,
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_set_config_bool(
    manager_ptr: *mut c_void,
    key: *const c_char,
    value: bool,
) {
    if manager_ptr.is_null() || key.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    let k = unsafe { CStr::from_ptr(key).to_string_lossy().into_owned() };
    
    match k.as_str() {
        "otamanagerloopback" => manager.ota_manager_loopback = value,
        "otamanagerchannelenable" => manager.ota_manager_channel_enable = value,
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_apply_config(manager_ptr: *mut c_void) {
    if manager_ptr.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    
    if !manager.antenna_profile_manifest_uri.is_empty() {
        let uri = CString::new(manager.antenna_profile_manifest_uri.clone()).unwrap();
        unsafe { emane_c_load_antenna_profile(uri.as_ptr()); }
    }
    
    if !manager.spectral_mask_manifest_uri.is_empty() {
        let uri = CString::new(manager.spectral_mask_manifest_uri.clone()).unwrap();
        unsafe { emane_c_load_spectral_mask(uri.as_ptr()); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_start(manager_ptr: *mut c_void) {
    if manager_ptr.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    
    if manager.ota_manager_channel_enable {
        let addr = CString::new(manager.ota_manager_group_addr.clone()).unwrap();
        let dev = CString::new(manager.ota_manager_group_device.clone()).unwrap();
        unsafe {
            crate::ota_manager::emane_rs_ota_manager_open(
                addr.as_ptr(),
                dev.as_ptr(),
                manager.ota_manager_ttl,
                manager.ota_manager_loopback,
                manager.uuid.as_ptr(),
                manager.ota_manager_mtu as usize,
                manager.ota_manager_part_check_threshold,
                manager.ota_manager_part_timeout_threshold,
            );
        }
    }
    
    let evt_addr = CString::new(manager.event_service_group_addr.clone()).unwrap();
    let evt_dev = CString::new(manager.event_service_device.clone()).unwrap();
    unsafe {
        crate::event_service::emane_rs_event_service_mcast_open(
            evt_addr.as_ptr(),
            evt_dev.as_ptr(),
            manager.event_service_ttl as std::os::raw::c_int,
            true, // loopback
            manager.uuid.as_ptr(),
        );
    }
    
    let cp_addr = CString::new(manager.control_port_addr.clone()).unwrap();
    unsafe {
        emane_c_control_port_open(cp_addr.as_ptr());
    }
    
    for (_, nem_ptr) in &manager.nems {
        unsafe { emane_c_nem_start(*nem_ptr); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_post_start(manager_ptr: *mut c_void) {
    if manager_ptr.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    for (_, nem_ptr) in &manager.nems {
        unsafe { emane_c_nem_post_start(*nem_ptr); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_stop(manager_ptr: *mut c_void) {
    if manager_ptr.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    
    unsafe { emane_c_control_port_close(); }
    
    for (_, nem_ptr) in &manager.nems {
        unsafe { emane_c_nem_stop(*nem_ptr); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_manager_destroy(manager_ptr: *mut c_void) {
    if manager_ptr.is_null() { return; }
    let manager = unsafe { &mut *(manager_ptr as *mut NemManager) };
    
    for (_, nem_ptr) in &manager.nems {
        unsafe { emane_c_nem_destroy(*nem_ptr); }
    }
}
