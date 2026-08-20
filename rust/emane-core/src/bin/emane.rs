use emane_core::nem_manager::NemManager;
use emane_core::xml_parser::{parse_platform, ParamMap};
use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::path::Path;

#[repr(C)]
pub struct FfiStringArray {
    pub data: *const *const c_char,
    pub len: usize,
}

#[repr(C)]
pub struct FfiConfigUpdateReqItem {
    pub name: *const c_char,
    pub values: FfiStringArray,
}

#[repr(C)]
pub struct FfiConfigUpdateReq {
    pub data: *const FfiConfigUpdateReqItem,
    pub len: usize,
}

extern "C" {
    fn emane_rs_ffi_build_phy_layer(
        id: u16,
        sLibraryFile: *const c_char,
        req: FfiConfigUpdateReq,
        bSkipConfigure: bool,
    ) -> *mut c_void;
    fn emane_rs_ffi_build_mac_layer(
        id: u16,
        sLibraryFile: *const c_char,
        req: FfiConfigUpdateReq,
        bSkipConfigure: bool,
    ) -> *mut c_void;
    fn emane_rs_ffi_build_transport_layer(
        id: u16,
        sLibraryFile: *const c_char,
        req: FfiConfigUpdateReq,
        bSkipConfigure: bool,
    ) -> *mut c_void;
    fn emane_rs_ffi_build_shim_layer(
        id: u16,
        sLibraryFile: *const c_char,
        req: FfiConfigUpdateReq,
        bSkipConfigure: bool,
    ) -> *mut c_void;
    fn emane_rs_ffi_build_nem(
        id: u16,
        layers: *mut *mut c_void,
        num_layers: usize,
        req: FfiConfigUpdateReq,
        bExternalTransport: bool,
    ) -> *mut c_void;

    fn emane_rs_nem_manager_apply_config(manager_ptr: *mut c_void);
    fn emane_rs_nem_manager_start(manager_ptr: *mut c_void);
    fn emane_rs_nem_manager_post_start(manager_ptr: *mut c_void);
    fn emane_rs_nem_manager_stop(manager_ptr: *mut c_void);
}

struct ConfigUpdateGuard {
    items: Vec<FfiConfigUpdateReqItem>,
    _name_cstrs: Vec<CString>,
    _values_cstrs: Vec<Vec<CString>>,
    _values_ptrs: Vec<Vec<*const c_char>>,
}

fn build_ffi_config_req(params: &ParamMap) -> ConfigUpdateGuard {
    let mut guard = ConfigUpdateGuard {
        items: Vec::new(),
        _name_cstrs: Vec::new(),
        _values_cstrs: Vec::new(),
        _values_ptrs: Vec::new(),
    };

    for (k, v) in params {
        let name_cstr = CString::new(k.as_str()).unwrap();
        let mut vals_cstr = Vec::new();
        let mut vals_ptr = Vec::new();
        for val in &v.values {
            let c_val = CString::new(val.as_str()).unwrap();
            vals_ptr.push(c_val.as_ptr());
            vals_cstr.push(c_val);
        }

        guard._name_cstrs.push(name_cstr);
        guard._values_cstrs.push(vals_cstr);
        guard._values_ptrs.push(vals_ptr);
    }

    // Assemble items after we have pinned the strings in vectors
    for i in 0..guard._name_cstrs.len() {
        let ffi_str_array = FfiStringArray {
            data: guard._values_ptrs[i].as_ptr(),
            len: guard._values_ptrs[i].len(),
        };
        let item = FfiConfigUpdateReqItem {
            name: guard._name_cstrs[i].as_ptr(),
            values: ffi_str_array,
        };
        guard.items.push(item);
    }

    guard
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut config_url = String::new();
    let mut daemonize = false;
    let mut log_file = None;
    let mut log_level = 2;
    let mut pid_file = None;
    let mut priority = 50;
    let mut realtime = false;
    let mut syslog = false;
    let mut uuid_file = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--daemonize" => daemonize = true,
            "-r" | "--realtime" => realtime = true,
            "--syslog" => syslog = true,
            "-h" | "--help" => {
                println!("Usage: {} [OPTIONS]... CONFIG_URL", args[0]);
                return;
            }
            "-v" | "--version" => {
                println!("1.0.0");
                return;
            }
            "-f" | "--logfile" => {
                i += 1;
                if i < args.len() {
                    log_file = Some(args[i].clone());
                }
            }
            "-l" | "--loglevel" => {
                i += 1;
                if i < args.len() {
                    log_level = args[i].parse().unwrap_or(2);
                }
            }
            "--pidfile" => {
                i += 1;
                if i < args.len() {
                    pid_file = Some(args[i].clone());
                }
            }
            "-p" | "--priority" => {
                i += 1;
                if i < args.len() {
                    priority = args[i].parse().unwrap_or(50);
                }
            }
            "--uuidfile" => {
                i += 1;
                if i < args.len() {
                    uuid_file = Some(args[i].clone());
                }
            }
            arg if !arg.starts_with("-") => {
                config_url = arg.to_string();
            }
            _ => {
                eprintln!("Unknown option {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if config_url.is_empty() {
        eprintln!("Missing CONFIG_URL");
        std::process::exit(1);
    }

    let platform_path = Path::new(&config_url);
    let platform_config = match parse_platform(platform_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to parse platform configuration: {}", e);
            std::process::exit(1);
        }
    };

    let uuid = [0u8; 16]; // Just dummy UUID for now
    let mut nem_manager = NemManager::new(uuid);

    println!(
        "Loaded platform config with {} NEMs",
        platform_config.nems.len()
    );

    for nem in platform_config.nems {
        println!("Loaded NEM id={}", nem.id);

        let mut built_layers = Vec::new();

        for layer in nem.layers {
            let plugin_name = layer.plugin.clone().unwrap_or_default();
            println!("  Layer: {} (plugin: {})", layer.layer_type, plugin_name);

            let lib_file = CString::new(plugin_name).unwrap();
            let guard = build_ffi_config_req(&layer.params);
            let req = FfiConfigUpdateReq {
                data: guard.items.as_ptr(),
                len: guard.items.len(),
            };

            let layer_ptr = unsafe {
                match layer.layer_type.as_str() {
                    "phy" => emane_rs_ffi_build_phy_layer(nem.id, lib_file.as_ptr(), req, false),
                    "mac" => emane_rs_ffi_build_mac_layer(nem.id, lib_file.as_ptr(), req, false),
                    "transport" => {
                        emane_rs_ffi_build_transport_layer(nem.id, lib_file.as_ptr(), req, false)
                    }
                    "shim" => emane_rs_ffi_build_shim_layer(nem.id, lib_file.as_ptr(), req, false),
                    _ => {
                        eprintln!("Unknown layer type: {}", layer.layer_type);
                        std::ptr::null_mut()
                    }
                }
            };

            if !layer_ptr.is_null() {
                built_layers.push(layer_ptr);
            }
        }

        let nem_guard = build_ffi_config_req(&nem.params);
        let nem_req = FfiConfigUpdateReq {
            data: nem_guard.items.as_ptr(),
            len: nem_guard.items.len(),
        };

        let nem_ptr = unsafe {
            emane_rs_ffi_build_nem(
                nem.id,
                built_layers.as_mut_ptr(),
                built_layers.len(),
                nem_req,
                nem.external_transport,
            )
        };

        if !nem_ptr.is_null() {
            nem_manager.add(nem.id, nem_ptr);
        } else {
            eprintln!("Failed to build NEM {}", nem.id);
        }
    }

    let manager_ptr = &mut nem_manager as *mut _ as *mut c_void;
    unsafe {
        emane_rs_nem_manager_apply_config(manager_ptr);
        emane_rs_nem_manager_start(manager_ptr);
        emane_rs_nem_manager_post_start(manager_ptr);
    }

    println!("Emulator started! Parking thread.");

    std::thread::park();

    println!("Stopping emulator...");
    unsafe {
        emane_rs_nem_manager_stop(manager_ptr);
    }
}
