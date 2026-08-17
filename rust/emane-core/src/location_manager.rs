use std::collections::HashMap;
use std::os::raw::c_void;

extern "C" {
    fn emane_rs_ffi_free_pov(ptr: *mut c_void);
    fn emane_rs_ffi_free_loc_info(ptr: *mut c_void);
}

pub struct LocationManagerImpl {
    nem_id: u16,
    local_pov: *mut c_void,
    store: HashMap<u16, *mut c_void>,
    cache: HashMap<u16, *mut c_void>,
    seq: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_create(nem_id: u16) -> *mut c_void {
    let mgr = Box::new(LocationManagerImpl {
        nem_id,
        local_pov: std::ptr::null_mut(),
        store: HashMap::new(),
        cache: HashMap::new(),
        seq: 0,
    });
    Box::into_raw(mgr) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        let mut mgr = unsafe { Box::from_raw(ptr as *mut LocationManagerImpl) };
        if !mgr.local_pov.is_null() {
            unsafe { emane_rs_ffi_free_pov(mgr.local_pov); }
        }
        for (_, pov) in mgr.store.drain() {
            unsafe { emane_rs_ffi_free_pov(pov); }
        }
        for (_, loc_info) in mgr.cache.drain() {
            unsafe { emane_rs_ffi_free_loc_info(loc_info); }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_get_local_pov(ptr: *mut c_void) -> *mut c_void {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    mgr.local_pov
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_set_local_pov(ptr: *mut c_void, pov: *mut c_void) {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    if !mgr.local_pov.is_null() && mgr.local_pov != pov {
        unsafe { emane_rs_ffi_free_pov(mgr.local_pov); }
    }
    mgr.local_pov = pov;
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_get_pov(ptr: *mut c_void, nem_id: u16) -> *mut c_void {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    mgr.store.get(&nem_id).copied().unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_insert_pov(ptr: *mut c_void, nem_id: u16, pov: *mut c_void) {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    if let Some(old_pov) = mgr.store.insert(nem_id, pov) {
        if old_pov != pov {
            unsafe { emane_rs_ffi_free_pov(old_pov); }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_get_cache(ptr: *mut c_void, nem_id: u16) -> *mut c_void {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    mgr.cache.get(&nem_id).copied().unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_set_cache(ptr: *mut c_void, nem_id: u16, loc_info: *mut c_void) {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    if let Some(old) = mgr.cache.insert(nem_id, loc_info) {
        if old != loc_info {
            unsafe { emane_rs_ffi_free_loc_info(old); }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_clear_cache(ptr: *mut c_void) {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    for (_, loc_info) in mgr.cache.drain() {
        unsafe { emane_rs_ffi_free_loc_info(loc_info); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_erase_cache(ptr: *mut c_void, nem_id: u16) {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    if let Some(old) = mgr.cache.remove(&nem_id) {
        unsafe { emane_rs_ffi_free_loc_info(old); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_get_seq(ptr: *mut c_void) -> u64 {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    mgr.seq
}

#[no_mangle]
pub extern "C" fn emane_rs_location_manager_inc_seq(ptr: *mut c_void) -> u64 {
    let mgr = unsafe { &mut *(ptr as *mut LocationManagerImpl) };
    mgr.seq += 1;
    mgr.seq
}
