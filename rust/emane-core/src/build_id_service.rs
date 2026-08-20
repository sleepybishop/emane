use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};

#[repr(C)]
pub struct FfiNEMLayerComponent {
    pub build_id: u16,
    pub layer_type: i32,
    pub plugin_name: *const c_char,
}

#[repr(C)]
pub struct FfiNEMLayerComponentList {
    pub nem_id: u16,
    pub components: *mut FfiNEMLayerComponent,
    pub len: usize,
}

#[repr(C)]
pub struct FfiNEMLayerComponentMap {
    pub nems: *mut FfiNEMLayerComponentList,
    pub len: usize,
}

pub struct NEMLayerComponent {
    pub build_id: u16,
    pub layer_type: i32,
    pub plugin_name: String,
}

pub struct BuildIdService {
    next_build_id: u16,
    nem_manager_build_id: Option<u16>,
    transport_manager_build_id: Option<u16>,
    event_generator_manager_build_id: Option<u16>,
    event_agent_manager_build_id: Option<u16>,
    
    nem_layer_components: HashMap<u16, Vec<NEMLayerComponent>>,
    nem_transport_adapters: HashMap<u16, u16>,
    nem_transports: HashMap<u16, u16>,
    nems: HashMap<u16, u16>,
    event_generators: Vec<u16>,
    event_agents: Vec<u16>,
}

impl BuildIdService {
    pub fn new() -> Self {
        BuildIdService {
            next_build_id: 0,
            nem_manager_build_id: None,
            transport_manager_build_id: None,
            event_generator_manager_build_id: None,
            event_agent_manager_build_id: None,
            nem_layer_components: HashMap::new(),
            nem_transport_adapters: HashMap::new(),
            nem_transports: HashMap::new(),
            nems: HashMap::new(),
            event_generators: Vec::new(),
            event_agents: Vec::new(),
        }
    }

    pub fn assign_build_id(&mut self) -> u16 {
        self.next_build_id += 1;
        self.next_build_id
    }
}

fn get_build_id_service() -> &'static Mutex<BuildIdService> {
    static BUILD_ID_SERVICE: OnceLock<Mutex<BuildIdService>> = OnceLock::new();
    BUILD_ID_SERVICE.get_or_init(|| Mutex::new(BuildIdService::new()))
}

fn write_error(msg: &str, err_buf: *mut c_char, err_len: usize) {
    if err_buf.is_null() || err_len == 0 { return; }
    let c_msg = std::ffi::CString::new(msg).unwrap_or_default();
    let bytes = c_msg.as_bytes_with_nul();
    let copy_len = std::cmp::min(bytes.len(), err_len - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), err_buf as *mut u8, copy_len);
        *err_buf.add(copy_len) = 0;
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_assign() -> u16 {
    let mut s = get_build_id_service().lock().unwrap();
    s.assign_build_id()
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_nem_manager(build_id: u16, err_buf: *mut c_char, err_len: usize) {
    let mut s = get_build_id_service().lock().unwrap();
    if s.nem_manager_build_id.is_none() {
        s.nem_manager_build_id = Some(build_id);
    } else {
        write_error("NEM Manager already registered", err_buf, err_len);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_transport_manager(build_id: u16, err_buf: *mut c_char, err_len: usize) {
    let mut s = get_build_id_service().lock().unwrap();
    if s.transport_manager_build_id.is_none() {
        s.transport_manager_build_id = Some(build_id);
    } else {
        write_error("Transport Manager already registered", err_buf, err_len);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_event_generator_manager(build_id: u16, err_buf: *mut c_char, err_len: usize) {
    let mut s = get_build_id_service().lock().unwrap();
    if s.event_generator_manager_build_id.is_none() {
        s.event_generator_manager_build_id = Some(build_id);
    } else {
        write_error("Event Generator Manager already registered", err_buf, err_len);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_event_agent_manager(build_id: u16, err_buf: *mut c_char, err_len: usize) {
    let mut s = get_build_id_service().lock().unwrap();
    if s.event_agent_manager_build_id.is_none() {
        s.event_agent_manager_build_id = Some(build_id);
    } else {
        write_error("Event Agent Manager already registered", err_buf, err_len);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_layer(nem_id: u16, build_id: u16, layer_type: i32, plugin_name: *const c_char) {
    let mut s = get_build_id_service().lock().unwrap();
    let name = if plugin_name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(plugin_name).to_string_lossy().into_owned() }
    };
    
    let entry = s.nem_layer_components.entry(nem_id).or_insert_with(Vec::new);
    entry.push(NEMLayerComponent {
        build_id,
        layer_type,
        plugin_name: name,
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_transport(nem_id: u16, build_id: u16) {
    let mut s = get_build_id_service().lock().unwrap();
    s.nem_transports.insert(nem_id, build_id);
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_nem(nem_id: u16, build_id: u16) {
    let mut s = get_build_id_service().lock().unwrap();
    s.nems.insert(nem_id, build_id);
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_transport_adapter(nem_id: u16, build_id: u16) {
    let mut s = get_build_id_service().lock().unwrap();
    s.nem_transport_adapters.insert(nem_id, build_id);
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_event_generator(build_id: u16) {
    let mut s = get_build_id_service().lock().unwrap();
    s.event_generators.push(build_id);
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_register_event_agent(build_id: u16) {
    let mut s = get_build_id_service().lock().unwrap();
    s.event_agents.push(build_id);
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_get_nem_layer_component_map() -> FfiNEMLayerComponentMap {
    let s = get_build_id_service().lock().unwrap();
    let mut list_vec = Vec::new();
    
    for (&nem_id, components) in &s.nem_layer_components {
        let mut comp_vec = Vec::new();
        for comp in components {
            comp_vec.push(FfiNEMLayerComponent {
                build_id: comp.build_id,
                layer_type: comp.layer_type,
                plugin_name: CString::new(comp.plugin_name.clone()).unwrap().into_raw(),
            });
        }
        let mut comp_boxed = comp_vec.into_boxed_slice();
        list_vec.push(FfiNEMLayerComponentList {
            nem_id,
            components: comp_boxed.as_mut_ptr(),
            len: comp_boxed.len(),
        });
        std::mem::forget(comp_boxed);
    }
    
    let mut list_boxed = list_vec.into_boxed_slice();
    let map = FfiNEMLayerComponentMap {
        nems: list_boxed.as_mut_ptr(),
        len: list_boxed.len(),
    };
    std::mem::forget(list_boxed);
    map
}

#[no_mangle]
pub extern "C" fn emane_rs_buildid_free_nem_layer_component_map(map: FfiNEMLayerComponentMap) {
    if !map.nems.is_null() {
        let lists = unsafe { std::slice::from_raw_parts_mut(map.nems, map.len) };
        for list in lists {
            if !list.components.is_null() {
                let comps = unsafe { std::slice::from_raw_parts_mut(list.components, list.len) };
                for comp in comps {
                    if !comp.plugin_name.is_null() {
                        unsafe { let _ = CString::from_raw(comp.plugin_name as *mut c_char); }
                    }
                }
                unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(list.components, list.len))); }
            }
        }
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(map.nems, map.len))); }
    }
}
