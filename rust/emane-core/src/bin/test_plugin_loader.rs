use libloading::{Library, Symbol};
use emane_core::plugin_interface::{PluginApi, PluginEntryFunc};
use std::ffi::CStr;

fn main() {
    let lib_path = "../../target/debug/libdummy_mac.so";
    println!("Loading plugin: {}", lib_path);
    
    unsafe {
        let lib = Library::new(lib_path).unwrap();
        let create_func: Symbol<PluginEntryFunc> = lib.get(b"emane_plugin_create").unwrap();
        
        let api_ptr = create_func();
        let api = &*api_ptr;
        
        let name = CStr::from_ptr(api.name).to_string_lossy();
        println!("Successfully loaded plugin! Name: {}, Type: {}", name, api.plugin_type);
        
        println!("Calling init(42)...");
        let _ptr = (api.init)(42, std::ptr::null());
        
        println!("Calling start()...");
        (api.start)(_ptr);
    }
}
