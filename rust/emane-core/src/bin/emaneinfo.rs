extern crate emane_core;
use roxmltree::Document;
use std::env;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_void};
use std::process;

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

#[repr(C)]
pub struct FfiAny {
    pub any_type: i32,
    pub i64_value: i64,
    pub u64_value: u64,
    pub d_value: f64,
    pub s_value: *const c_char,
}

#[repr(C)]
pub struct FfiAnyArray {
    pub data: *const FfiAny,
    pub len: usize,
}

#[repr(C)]
pub struct FfiConfigInfo {
    pub name: *const c_char,
    pub any_type: i32,
    pub properties: u64,
    pub values: FfiAnyArray,
    pub usage: *const c_char,
    pub has_min_max: bool,
    pub min_value: FfiAny,
    pub max_value: FfiAny,
    pub min_occurs: usize,
    pub max_occurs: usize,
    pub regex_pattern: *const c_char,
}

#[repr(C)]
pub struct FfiConfigManifest {
    pub data: *mut FfiConfigInfo,
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
    fn emane_rs_ffi_component_get_build_id(component: *mut c_void) -> u16;

    fn emane_rs_config_get_manifest(build_id: u16) -> FfiConfigManifest;
    fn emane_rs_config_free_manifest(manifest: FfiConfigManifest);
}

fn type_as_string(any_type: i32) -> &'static str {
    match any_type {
        0 => "int64",
        1 => "uint64",
        2 => "int32",
        3 => "uint32",
        4 => "int16",
        5 => "uint16",
        6 => "int8",
        7 => "uint8",
        8 => "float",
        9 => "double",
        10 => "inetaddr",
        11 => "bool",
        12 => "string",
        _ => "unknown",
    }
}

fn get_string_value(any_type: i32, val: &FfiAny) -> String {
    if !val.s_value.is_null() {
        let s = unsafe { CStr::from_ptr(val.s_value).to_string_lossy().into_owned() };
        if !s.is_empty() {
            return s;
        }
    }
    match any_type {
        0 | 2 | 4 | 6 => val.i64_value.to_string(),
        1 | 3 | 5 | 7 => val.u64_value.to_string(),
        8 | 9 => val.d_value.to_string(),
        11 => (if val.u64_value > 0 { "true" } else { "false" }).to_string(),
        _ => String::new(),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut b_configuration = false;
    let mut b_show_manifest = false;
    let mut target_file = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("usage: emaneinfo [OPTIONS]... <plugin>");
                println!();
                println!(" CONFIG_URI                    URI of XML configuration file.");
                println!();
                println!("options:");
                println!("  -h, --help                     Print this message and exit.");
                println!("  -m, --manifest                 Print manifest.");
                println!("  -v, --version                  Print version and exit.");
                println!("  -c, --configuration            Parse configuration.");
                return;
            }
            "-v" | "--version" => {
                println!("1.0.0");
                return;
            }
            "-c" | "--configuration" => {
                b_configuration = true;
            }
            "-m" | "--manifest" => {
                b_show_manifest = true;
            }
            arg if !arg.starts_with("-") => {
                target_file = arg.to_string();
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    if target_file.is_empty() {
        eprintln!("Missing plugin");
        process::exit(1);
    }

    let mut plugin_type = "shim".to_string();
    let mut plugin_name = target_file.clone();

    if b_configuration {
        let content = fs::read_to_string(&target_file).unwrap_or_else(|e| {
            eprintln!("Failed to read file: {}", e);
            process::exit(1);
        });
        let doc = Document::parse(&content).unwrap_or_else(|e| {
            eprintln!("Failed to parse XML: {}", e);
            process::exit(1);
        });

        let root = doc.root_element();
        plugin_type = root.tag_name().name().to_string();
        if let Some(lib) = root.attribute("library") {
            plugin_name = lib.to_string();
        }
    } else {
        if plugin_name == "emanephy" || plugin_name.is_empty() {
            plugin_name.clear();
            plugin_type = "phy".to_string();
        }
    }

    let lib_file_cstr = CString::new(plugin_name.clone()).unwrap();
    let req = FfiConfigUpdateReq {
        data: std::ptr::null(),
        len: 0,
    };

    let layer_ptr = unsafe {
        match plugin_type.as_str() {
            "phy" => emane_rs_ffi_build_phy_layer(1, lib_file_cstr.as_ptr(), req, true),
            "mac" => emane_rs_ffi_build_mac_layer(1, lib_file_cstr.as_ptr(), req, true),
            "transport" => emane_rs_ffi_build_transport_layer(1, lib_file_cstr.as_ptr(), req, true),
            "shim" => emane_rs_ffi_build_shim_layer(1, lib_file_cstr.as_ptr(), req, true),
            _ => {
                // If it's something else, fallback to shim for now.
                emane_rs_ffi_build_shim_layer(1, lib_file_cstr.as_ptr(), req, true)
            }
        }
    };

    if layer_ptr.is_null() {
        eprintln!("Failed to load plugin: {}", plugin_name);
        process::exit(1);
    }

    let build_id = unsafe { emane_rs_ffi_component_get_build_id(layer_ptr) };

    if b_show_manifest {
        let manifest = unsafe { emane_rs_config_get_manifest(build_id) };
        println!("<?xml version=\"1.0\"?>");
        println!("<manifest>");
        println!("  <plugin name=\"{}\">", target_file);
        println!("    <configuration>");

        let slice = unsafe { std::slice::from_raw_parts(manifest.data, manifest.len) };
        for info in slice {
            let name = unsafe { CStr::from_ptr(info.name).to_string_lossy() };
            let usage = if info.usage.is_null() {
                "".to_string()
            } else {
                unsafe { CStr::from_ptr(info.usage).to_string_lossy().into_owned() }
            };

            let is_required = (info.properties & 1) != 0;
            let is_modifiable = (info.properties & 2) != 0;
            let is_default = (info.properties & 4) != 0;

            let req_str = if is_required { "yes" } else { "no" };
            let mod_str = if is_modifiable { "yes" } else { "no" };
            let def_str = if info.values.len > 0 { "yes" } else { "no" };

            println!(
                "      <parameter name=\"{}\" default=\"{}\" required=\"{}\" modifiable=\"{}\">",
                name, def_str, req_str, mod_str
            );

            let type_str = type_as_string(info.any_type);
            let b_numeric = type_str != "inetaddr" && type_str != "string";
            if b_numeric {
                let min_s = get_string_value(info.any_type, &info.min_value);
                let max_s = get_string_value(info.any_type, &info.max_value);
                println!(
                    "        <numeric type=\"{}\" minValue=\"{}\" maxValue=\"{}\">",
                    type_str, min_s, max_s
                );
                println!(
                    "          <values minOccurs=\"{}\" maxOccurs=\"{}\">",
                    info.min_occurs, info.max_occurs
                );
                let val_slice =
                    unsafe { std::slice::from_raw_parts(info.values.data, info.values.len) };
                for v in val_slice {
                    println!(
                        "            <value>{}</value>",
                        get_string_value(info.any_type, v)
                    );
                }
                println!("          </values>");
                if !usage.is_empty() {
                    println!("          <description>{}</description>", usage);
                }
                println!("        </numeric>");
            } else {
                println!("        <nonnumeric type=\"{}\">", type_str);
                println!(
                    "          <values minOccurs=\"{}\" maxOccurs=\"{}\">",
                    info.min_occurs, info.max_occurs
                );
                let val_slice =
                    unsafe { std::slice::from_raw_parts(info.values.data, info.values.len) };
                for v in val_slice {
                    println!(
                        "            <value>{}</value>",
                        get_string_value(info.any_type, v)
                    );
                }
                println!("          </values>");
                if !info.regex_pattern.is_null() {
                    let reg = unsafe { CStr::from_ptr(info.regex_pattern).to_string_lossy() };
                    if !reg.is_empty() {
                        println!("          <regex><![CDATA[{}]]></regex>", reg);
                    }
                }
                if !usage.is_empty() {
                    println!("          <description>{}</description>", usage);
                }
                println!("        </nonnumeric>");
            }
            println!("      </parameter>");
        }

        println!("    </configuration>");
        println!("    <statistics>");
        println!("    </statistics>");
        println!("    <statistictables>");
        println!("    </statistictables>");
        println!("  </plugin>");
        println!("</manifest>");

        unsafe { emane_rs_config_free_manifest(manifest) };
    }
}
