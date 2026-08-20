// C++ fading selection event models
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FadingModel {
    None = 0,
    Nakagami = 1,
    Lognormal = 2,
}

pub struct FadingManager {
    nem_id: u16,
    fading_model: String,
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingmanager_new(nem_id: u16) -> *mut FadingManager {
    Box::into_raw(Box::new(FadingManager {
        nem_id,
        fading_model: "none".to_string(),
    }))
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingmanager_free(ptr: *mut FadingManager) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn emane_rs_fadingmanager_update_config(
    ptr: *mut FadingManager,
    s_type: *const c_char,
) {
    if let Some(mgr) = unsafe { ptr.as_mut() } {
        if !s_type.is_null() {
            if let Ok(s) = unsafe { CStr::from_ptr(s_type) }.to_str() {
                mgr.fading_model = s.to_string();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingmanager_update_selection(
    _ptr: *mut FadingManager,
    _nem_id: u16,
    _model: u32,
) {
    // update logic
}

pub struct FadingAlgorithmStore {
    // store logic
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingalgorithmstore_new() -> *mut FadingAlgorithmStore {
    Box::into_raw(Box::new(FadingAlgorithmStore {}))
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingalgorithmstore_free(ptr: *mut FadingAlgorithmStore) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingmanager_create_store(
    _ptr: *mut FadingManager,
) -> *mut FadingAlgorithmStore {
    emane_rs_fadingalgorithmstore_new()
}
