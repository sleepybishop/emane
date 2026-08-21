use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

pub struct EelLoaderPluginFactory {
    handle: *mut c_void,
    create_func: extern "C" fn() -> *mut c_void,
    destroy_func: extern "C" fn(*mut c_void),
}

impl EelLoaderPluginFactory {
    pub fn construct(library_name: &str) -> Result<Self, String> {
        let c_lib_name = CString::new(library_name).unwrap();
        let handle = unsafe { libc::dlopen(c_lib_name.as_ptr(), libc::RTLD_NOW) };
        if handle.is_null() {
            let err = unsafe { CStr::from_ptr(libc::dlerror()) };
            return Err(err.to_string_lossy().into_owned());
        }

        let create_ptr = unsafe { libc::dlsym(handle, c"create".as_ptr()) };
        if create_ptr.is_null() {
            unsafe { libc::dlclose(handle) };
            return Err(format!(
                "{} missing create symbol. (Missing DECLARE_EEL_LOADER_PLUGIN()?)",
                library_name
            ));
        }

        let destroy_ptr = unsafe { libc::dlsym(handle, c"destroy".as_ptr()) };
        if destroy_ptr.is_null() {
            unsafe { libc::dlclose(handle) };
            return Err(format!(
                "{} missing destroy symbol. (Missing DECLARE_EEL_LOADER_PLUGIN()?)",
                library_name
            ));
        }

        Ok(EelLoaderPluginFactory {
            handle,
            create_func: unsafe {
                std::mem::transmute::<*mut c_void, extern "C" fn() -> *mut c_void>(create_ptr)
            },
            destroy_func: unsafe {
                std::mem::transmute::<*mut c_void, extern "C" fn(*mut c_void)>(destroy_ptr)
            },
        })
    }

    pub fn create_plugin(&self) -> *mut c_void {
        (self.create_func)()
    }

    pub fn destroy_plugin(&self, plugin: *mut c_void) {
        (self.destroy_func)(plugin)
    }
}

impl Drop for EelLoaderPluginFactory {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { libc::dlclose(self.handle) };
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_loader_plugin_factory_new() -> *mut EelLoaderPluginFactory {
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_loader_plugin_factory_construct(
    library_name: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut EelLoaderPluginFactory {
    let lib_str = unsafe { CStr::from_ptr(library_name) }.to_string_lossy();
    match EelLoaderPluginFactory::construct(&lib_str) {
        Ok(f) => Box::into_raw(Box::new(f)),
        Err(e) => {
            unsafe {
                *error_out = CString::new(e).unwrap().into_raw();
            }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_loader_plugin_factory_free(ptr: *mut EelLoaderPluginFactory) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_loader_plugin_factory_create_plugin(
    ptr: *const EelLoaderPluginFactory,
) -> *mut c_void {
    if ptr.is_null() {
        return ptr::null_mut();
    }
    let factory = unsafe { &*ptr };
    factory.create_plugin()
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_loader_plugin_factory_destroy_plugin(
    ptr: *const EelLoaderPluginFactory,
    plugin: *mut c_void,
) {
    if ptr.is_null() || plugin.is_null() {
        return;
    }
    let factory = unsafe { &*ptr };
    factory.destroy_plugin(plugin)
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_loader_plugin_factory_free_error(err: *mut c_char) {
    if !err.is_null() {
        unsafe {
            drop(CString::from_raw(err));
        }
    }
}
