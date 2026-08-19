
use libc::{c_void, dlclose, dlerror, dlopen, dlsym, RTLD_NOW};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex, OnceLock};
use std::os::raw::c_char;

type FactoryMap<T> = Mutex<HashMap<String, Arc<T>>>;

fn mac_factory_map() -> &'static FactoryMap<LayerFactory> {
    static MAP: OnceLock<FactoryMap<LayerFactory>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn phy_factory_map() -> &'static FactoryMap<LayerFactory> {
    static MAP: OnceLock<FactoryMap<LayerFactory>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn shim_factory_map() -> &'static FactoryMap<LayerFactory> {
    static MAP: OnceLock<FactoryMap<LayerFactory>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn transport_factory_map() -> &'static FactoryMap<TransportFactory> {
    static MAP: OnceLock<FactoryMap<TransportFactory>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn event_agent_factory_map() -> &'static FactoryMap<TransportFactory> {
    static MAP: OnceLock<FactoryMap<TransportFactory>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn event_generator_factory_map() -> &'static FactoryMap<EventGeneratorFactory> {
    static MAP: OnceLock<FactoryMap<EventGeneratorFactory>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}


pub struct EventGeneratorFactory {
    handle: *mut c_void,
    create_func: extern "C" fn(*mut c_void) -> *mut c_void,
    destroy_func: extern "C" fn(*mut c_void),
}

unsafe impl Send for EventGeneratorFactory {}
unsafe impl Sync for EventGeneratorFactory {}

impl EventGeneratorFactory {
    pub fn new(library_name: &str) -> Result<Self, String> {
        let c_library_name = CString::new(library_name).map_err(|e| e.to_string())?;
        unsafe {
            let handle = dlopen(c_library_name.as_ptr(), RTLD_NOW);
            if handle.is_null() {
                return Err("dlopen error".to_string());
            }

            let create_sym = CString::new("create").unwrap();
            let create_ptr = dlsym(handle, create_sym.as_ptr());
            if create_ptr.is_null() {
                dlclose(handle);
                return Err(format!("{} missing create symbol.", library_name));
            }

            let destroy_sym = CString::new("destroy").unwrap();
            let destroy_ptr = dlsym(handle, destroy_sym.as_ptr());
            if destroy_ptr.is_null() {
                dlclose(handle);
                return Err(format!("{} missing destroy symbol.", library_name));
            }

            Ok(Self {
                handle,
                create_func: std::mem::transmute(create_ptr),
                destroy_func: std::mem::transmute(destroy_ptr),
            })
        }
    }

    pub fn create_instance(&self, platform: *mut c_void) -> *mut c_void {
        (self.create_func)(platform)
    }

    pub fn destroy_instance(&self, instance: *mut c_void) {
        (self.destroy_func)(instance)
    }
}

impl Drop for EventGeneratorFactory {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                dlclose(self.handle);
            }
        }
    }
}

pub struct LayerFactory {
    handle: *mut c_void,
    create_func: extern "C" fn(u16, *mut c_void, *mut c_void) -> *mut c_void,
    destroy_func: extern "C" fn(*mut c_void),
}

unsafe impl Send for LayerFactory {}
unsafe impl Sync for LayerFactory {}

impl LayerFactory {
    pub fn new(library_name: &str) -> Result<Self, String> {
        let c_library_name = CString::new(library_name).map_err(|e| e.to_string())?;
        unsafe {
            let handle = dlopen(c_library_name.as_ptr(), RTLD_NOW);
            if handle.is_null() {
                let err = dlerror();
                return Err(if !err.is_null() {
                    CStr::from_ptr(err).to_string_lossy().into_owned()
                } else {
                    "Unknown dlopen error".to_string()
                });
            }

            let create_sym = CString::new("create").unwrap();
            let create_ptr = dlsym(handle, create_sym.as_ptr());
            if create_ptr.is_null() {
                dlclose(handle);
                return Err(format!("{} missing create symbol.", library_name));
            }

            let destroy_sym = CString::new("destroy").unwrap();
            let destroy_ptr = dlsym(handle, destroy_sym.as_ptr());
            if destroy_ptr.is_null() {
                dlclose(handle);
                return Err(format!("{} missing destroy symbol.", library_name));
            }

            Ok(Self {
                handle,
                create_func: std::mem::transmute(create_ptr),
                destroy_func: std::mem::transmute(destroy_ptr),
            })
        }
    }

    pub fn create_layer(&self, id: u16, platform: *mut c_void, radio: *mut c_void) -> *mut c_void {
        (self.create_func)(id, platform, radio)
    }

    pub fn destroy_layer(&self, layer: *mut c_void) {
        (self.destroy_func)(layer)
    }
}

impl Drop for LayerFactory {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                dlclose(self.handle);
            }
        }
    }
}

pub struct TransportFactory {
    handle: *mut c_void,
    create_func: extern "C" fn(u16, *mut c_void) -> *mut c_void,
    destroy_func: extern "C" fn(*mut c_void),
}

unsafe impl Send for TransportFactory {}
unsafe impl Sync for TransportFactory {}

impl TransportFactory {
    pub fn new(library_name: &str) -> Result<Self, String> {
        let c_library_name = CString::new(library_name).map_err(|e| e.to_string())?;
        unsafe {
            let handle = dlopen(c_library_name.as_ptr(), RTLD_NOW);
            if handle.is_null() {
                let err = dlerror();
                return Err(if !err.is_null() {
                    CStr::from_ptr(err).to_string_lossy().into_owned()
                } else {
                    "Unknown dlopen error".to_string()
                });
            }

            let create_sym = CString::new("create").unwrap();
            let create_ptr = dlsym(handle, create_sym.as_ptr());
            if create_ptr.is_null() {
                dlclose(handle);
                return Err(format!("{} missing create symbol.", library_name));
            }

            let destroy_sym = CString::new("destroy").unwrap();
            let destroy_ptr = dlsym(handle, destroy_sym.as_ptr());
            if destroy_ptr.is_null() {
                dlclose(handle);
                return Err(format!("{} missing destroy symbol.", library_name));
            }

            Ok(Self {
                handle,
                create_func: std::mem::transmute(create_ptr),
                destroy_func: std::mem::transmute(destroy_ptr),
            })
        }
    }

    pub fn create_instance(&self, id: u16, platform: *mut c_void) -> *mut c_void {
        (self.create_func)(id, platform)
    }

    pub fn destroy_instance(&self, instance: *mut c_void) {
        (self.destroy_func)(instance)
    }
}

impl Drop for TransportFactory {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                dlclose(self.handle);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_create_mac_layer(
    s_library_file: *const c_char,
    id: u16,
    platform: *mut c_void,
    radio: *mut c_void,
    err_buf: *mut c_char,
    err_buf_len: usize,
) -> *mut c_void {
    let lib_name = CStr::from_ptr(s_library_file).to_string_lossy().into_owned();
    let mut map = mac_factory_map().lock().unwrap();
    let factory = map.entry(lib_name.clone()).or_insert_with(|| {
        Arc::new(LayerFactory::new(&lib_name).unwrap_or_else(|e| {
            let err_c = CString::new(e).unwrap();
            libc::strncpy(err_buf, err_c.as_ptr(), err_buf_len - 1);
            *err_buf.add(err_buf_len - 1) = 0;
            panic!("Failed to load factory");
        }))
    });
    factory.create_layer(id, platform, radio)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_create_phy_layer(
    s_library_file: *const c_char,
    id: u16,
    platform: *mut c_void,
    radio: *mut c_void,
    err_buf: *mut c_char,
    err_buf_len: usize,
) -> *mut c_void {
    let lib_name = CStr::from_ptr(s_library_file).to_string_lossy().into_owned();
    let mut map = phy_factory_map().lock().unwrap();
    let factory = map.entry(lib_name.clone()).or_insert_with(|| {
        Arc::new(LayerFactory::new(&lib_name).unwrap_or_else(|e| {
            let err_c = CString::new(e).unwrap();
            libc::strncpy(err_buf, err_c.as_ptr(), err_buf_len - 1);
            *err_buf.add(err_buf_len - 1) = 0;
            panic!("Failed to load factory");
        }))
    });
    factory.create_layer(id, platform, radio)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_create_shim_layer(
    s_library_file: *const c_char,
    id: u16,
    platform: *mut c_void,
    radio: *mut c_void,
    err_buf: *mut c_char,
    err_buf_len: usize,
) -> *mut c_void {
    let lib_name = CStr::from_ptr(s_library_file).to_string_lossy().into_owned();
    let mut map = shim_factory_map().lock().unwrap();
    let factory = map.entry(lib_name.clone()).or_insert_with(|| {
        Arc::new(LayerFactory::new(&lib_name).unwrap_or_else(|e| {
            let err_c = CString::new(e).unwrap();
            libc::strncpy(err_buf, err_c.as_ptr(), err_buf_len - 1);
            *err_buf.add(err_buf_len - 1) = 0;
            panic!("Failed to load factory");
        }))
    });
    factory.create_layer(id, platform, radio)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_create_transport(
    s_library_file: *const c_char,
    id: u16,
    platform: *mut c_void,
    err_buf: *mut c_char,
    err_buf_len: usize,
) -> *mut c_void {
    let lib_name = CStr::from_ptr(s_library_file).to_string_lossy().into_owned();
    let mut map = transport_factory_map().lock().unwrap();
    let factory = map.entry(lib_name.clone()).or_insert_with(|| {
        Arc::new(TransportFactory::new(&lib_name).unwrap_or_else(|e| {
            let err_c = CString::new(e).unwrap();
            libc::strncpy(err_buf, err_c.as_ptr(), err_buf_len - 1);
            *err_buf.add(err_buf_len - 1) = 0;
            panic!("Failed to load factory");
        }))
    });
    factory.create_instance(id, platform)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_create_event_agent(
    s_library_file: *const c_char,
    id: u16,
    platform: *mut c_void,
    err_buf: *mut c_char,
    err_buf_len: usize,
) -> *mut c_void {
    let lib_name = CStr::from_ptr(s_library_file).to_string_lossy().into_owned();
    let mut map = event_agent_factory_map().lock().unwrap();
    let factory = map.entry(lib_name.clone()).or_insert_with(|| {
        Arc::new(TransportFactory::new(&lib_name).unwrap_or_else(|e| {
            let err_c = CString::new(e).unwrap();
            libc::strncpy(err_buf, err_c.as_ptr(), err_buf_len - 1);
            *err_buf.add(err_buf_len - 1) = 0;
            panic!("Failed to load factory");
        }))
    });
    factory.create_instance(id, platform)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_create_event_generator(
    s_library_file: *const c_char,
    id: u16,
    platform: *mut c_void,
    err_buf: *mut c_char,
    err_buf_len: usize,
) -> *mut c_void {
    let lib_name = CStr::from_ptr(s_library_file).to_string_lossy().into_owned();
    let mut map = event_generator_factory_map().lock().unwrap();
    let factory = map.entry(lib_name.clone()).or_insert_with(|| {
        Arc::new(EventGeneratorFactory::new(&lib_name).unwrap_or_else(|e| {
            let err_c = CString::new(e).unwrap();
            libc::strncpy(err_buf, err_c.as_ptr(), err_buf_len - 1);
            *err_buf.add(err_buf_len - 1) = 0;
            panic!("Failed to load factory");
        }))
    });
    factory.create_instance(platform)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_factory_manager_destroy_all() {
    mac_factory_map().lock().unwrap().clear();
    phy_factory_map().lock().unwrap().clear();
    shim_factory_map().lock().unwrap().clear();
    transport_factory_map().lock().unwrap().clear();
    event_agent_factory_map().lock().unwrap().clear();
    event_generator_factory_map().lock().unwrap().clear();
}
