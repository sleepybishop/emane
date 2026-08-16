use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::os::raw::c_char;
use crate::config::VoidPtr;

extern "C" {
    fn emane_c_event_service_user_process_event(p_user: *mut std::ffi::c_void, event_id: u16, data: *const c_char, len: usize);
    // Note: LogServiceSingleton::instance() logger might be accessed from C++ if we really need to log
    // but for now we just omit the debug log inside Rust, or use a C++ callback to log.
    // We will omit the DEBUG_LEVEL logs that were in the loops for simplicity.
    fn emane_c_log_debug(msg: *const c_char);
}

pub struct EventServiceUser {
    pub build_id: u16,
    pub nem_id: u16,
    pub p_user: VoidPtr,
}

pub struct EventServiceRegistry {
    pub users: HashMap<u16, EventServiceUser>,
    pub registrations: HashMap<u16, Vec<u16>>, // event_id -> build_ids
}

fn get_event_service_registry() -> &'static Mutex<EventServiceRegistry> {
    static REGISTRY: OnceLock<Mutex<EventServiceRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(EventServiceRegistry {
        users: HashMap::new(),
        registrations: HashMap::new(),
    }))
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_user(build_id: u16, nem_id: u16, p_user: *mut std::ffi::c_void) {
    let mut reg = get_event_service_registry().lock().unwrap();
    reg.users.insert(build_id, EventServiceUser {
        build_id,
        nem_id,
        p_user: VoidPtr(p_user),
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_event(build_id: u16, event_id: u16) -> bool {
    let mut reg = get_event_service_registry().lock().unwrap();
    if reg.users.contains_key(&build_id) {
        reg.registrations.entry(event_id).or_insert_with(Vec::new).push(build_id);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_route_local_event(
    build_id: u16, nem_id: u16, event_id: u16, data: *const c_char, len: usize
) {
    let reg = get_event_service_registry().lock().unwrap();
    if let Some(build_ids) = reg.registrations.get(&event_id) {
        for &registered_build_id in build_ids {
            if let Some(user) = reg.users.get(&registered_build_id) {
                if build_id == 0 || registered_build_id != build_id {
                    if nem_id == 0 || user.nem_id == nem_id {
                        unsafe {
                            emane_c_event_service_user_process_event(user.p_user.0, event_id, data, len);
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_process_event_message(
    nem_id: u16, event_id: u16, data: *const c_char, len: usize, ignore_nem: u16
) {
    let reg = get_event_service_registry().lock().unwrap();
    if let Some(build_ids) = reg.registrations.get(&event_id) {
        for &registered_build_id in build_ids {
            if let Some(user) = reg.users.get(&registered_build_id) {
                if ignore_nem == 0 || ignore_nem != user.nem_id {
                    if nem_id == 0 || user.nem_id == nem_id {
                        unsafe {
                            emane_c_event_service_user_process_event(user.p_user.0, event_id, data, len);
                        }
                    }
                }
            }
        }
    }
}

