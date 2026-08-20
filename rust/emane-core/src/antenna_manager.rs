use std::collections::HashMap;
use std::os::raw::c_void;

extern "C" {
    fn emane_rs_ffi_free_antenna_info(ptr: *mut c_void);
    fn emane_rs_ffi_free_pointing(ptr: *mut c_void);
}

pub struct AntennaManagerImpl {
    store: HashMap<u16, HashMap<u16, *mut c_void>>,
    default_pointing: HashMap<u16, *mut c_void>,
    seq: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_create() -> *mut c_void {
    let mgr = Box::new(AntennaManagerImpl {
        store: HashMap::new(),
        default_pointing: HashMap::new(),
        seq: 0,
    });
    Box::into_raw(mgr) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        let mut mgr = unsafe { Box::from_raw(ptr as *mut AntennaManagerImpl) };
        for (_, mut inner_map) in mgr.store.drain() {
            for (_, info) in inner_map.drain() {
                unsafe {
                    emane_rs_ffi_free_antenna_info(info);
                }
            }
        }
        for (_, pt) in mgr.default_pointing.drain() {
            unsafe {
                emane_rs_ffi_free_pointing(pt);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_insert_antenna_info(
    ptr: *mut c_void,
    nem_id: u16,
    idx: u16,
    info: *mut c_void,
) {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    let inner = mgr.store.entry(nem_id).or_insert_with(HashMap::new);
    if let Some(old) = inner.insert(idx, info) {
        if old != info {
            unsafe {
                emane_rs_ffi_free_antenna_info(old);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_get_antenna_info(
    ptr: *mut c_void,
    nem_id: u16,
    idx: u16,
) -> *mut c_void {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    if let Some(inner) = mgr.store.get(&nem_id) {
        if let Some(info) = inner.get(&idx) {
            return *info;
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_remove_antenna_info(
    ptr: *mut c_void,
    nem_id: u16,
    idx: u16,
) {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    if let Some(inner) = mgr.store.get_mut(&nem_id) {
        if let Some(old) = inner.remove(&idx) {
            unsafe {
                emane_rs_ffi_free_antenna_info(old);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_insert_default_pointing(
    ptr: *mut c_void,
    nem_id: u16,
    pointing: *mut c_void,
) {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    if let Some(old) = mgr.default_pointing.insert(nem_id, pointing) {
        if old != pointing {
            unsafe {
                emane_rs_ffi_free_pointing(old);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_get_default_pointing(
    ptr: *mut c_void,
    nem_id: u16,
) -> *mut c_void {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    mgr.default_pointing
        .get(&nem_id)
        .copied()
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_get_seq(ptr: *mut c_void) -> u64 {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    mgr.seq
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_manager_inc_seq(ptr: *mut c_void) -> u64 {
    let mgr = unsafe { &mut *(ptr as *mut AntennaManagerImpl) };
    mgr.seq += 1;
    mgr.seq
}
