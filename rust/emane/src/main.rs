use std::env;
use std::collections::HashMap;
use std::ffi::CString;
use clap::Parser;
use roxmltree::Document;

use emane_core::config::{FfiConfigUpdateReq, FfiConfigUpdateReqItem, FfiStringArray};
use emane_core::nem_manager::{
    emane_rs_nem_manager_create,
    emane_rs_nem_manager_add,
    emane_rs_nem_manager_start,
    emane_rs_nem_manager_post_start,
    emane_rs_nem_manager_stop,
    emane_rs_nem_manager_destroy,
    emane_rs_nem_manager_set_config_str,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(help = "URL of XML configuration file.")]
    config_url: String,

    #[arg(short = 'd', long = "daemonize", help = "Run in the background.")]
    daemonize: bool,
    
    #[arg(short = 'l', long = "loglevel", default_value_t = 2, help = "Set initial log level [0,4].")]
    loglevel: u8,
}

unsafe extern "C" {
    fn emane_rs_ffi_build_phy_layer(id: u16, libFile: *const std::ffi::c_char, req: FfiConfigUpdateReq, skip_configure: bool) -> *mut std::ffi::c_void;
    fn emane_rs_ffi_build_mac_layer(id: u16, libFile: *const std::ffi::c_char, req: FfiConfigUpdateReq, skip_configure: bool) -> *mut std::ffi::c_void;
    fn emane_rs_ffi_build_shim_layer(id: u16, libFile: *const std::ffi::c_char, req: FfiConfigUpdateReq, skip_configure: bool) -> *mut std::ffi::c_void;
    fn emane_rs_ffi_build_transport_layer(id: u16, libFile: *const std::ffi::c_char, req: FfiConfigUpdateReq, skip_configure: bool) -> *mut std::ffi::c_void;
    fn emane_rs_ffi_build_nem(id: u16, layers: *const *mut std::ffi::c_void, num_layers: usize, req: FfiConfigUpdateReq, ext_transport: bool) -> *mut std::ffi::c_void;
}

fn parse_config_xml(path: &str) -> HashMap<String, Vec<String>> {
    let mut config = HashMap::new();
    let mut content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return config,
    };
    
    // Strip <!DOCTYPE ...> since roxmltree doesn't like it
    if let Some(start) = content.find("<!DOCTYPE") {
        if let Some(end) = content[start..].find('>') {
            content.replace_range(start..start+end+1, "");
        }
    }
    
    if let Ok(doc) = Document::parse(&content) {
        for node in doc.descendants() {
            if node.has_tag_name("param") {
                if let (Some(name), Some(value)) = (node.attribute("name"), node.attribute("value")) {
                    config.insert(name.to_string(), vec![value.to_string()]);
                }
            } else if node.has_tag_name("paramlist") {
                if let Some(name) = node.attribute("name") {
                    let mut values = Vec::new();
                    for item in node.children() {
                        if item.has_tag_name("item") {
                            if let Some(value) = item.attribute("value") {
                                values.push(value.to_string());
                            }
                        }
                    }
                    config.insert(name.to_string(), values);
                }
            }
        }
    }
    config
}

fn get_library_from_xml(filename: &str) -> String {
    if let Ok(mut content) = std::fs::read_to_string(filename) {
        if let Some(idx) = content.find("<!DOCTYPE") {
            if let Some(end_idx) = content[idx..].find('>') {
                content.replace_range(idx..idx + end_idx + 1, "");
            }
        }
        if let Ok(doc) = Document::parse(&content) {
            let root = doc.root_element();
            if let Some(lib) = root.attribute("library") {
                return lib.to_string();
            }
        }
    }
    String::new()
}

struct CStringConfig {
    items: Vec<(CString, Vec<CString>)>,
}

impl CStringConfig {
    fn from_hashmap(map: &HashMap<String, Vec<String>>) -> Self {
        let mut items = Vec::new();
        for (k, v) in map {
            let k_c = CString::new(k.clone()).unwrap();
            let v_c: Vec<CString> = v.iter().map(|s| CString::new(s.clone()).unwrap()).collect();
            items.push((k_c, v_c));
        }
        Self { items }
    }

    fn to_ffi(&self) -> (Vec<FfiConfigUpdateReqItem>, Vec<Vec<*const std::ffi::c_char>>) {
        let mut ffi_items = Vec::new();
        let mut ptr_arrays = Vec::new();

        for (name, vals) in &self.items {
            let mut ptrs = Vec::new();
            for val in vals {
                ptrs.push(val.as_ptr());
            }
            let ffi_vals = FfiStringArray {
                data: ptrs.as_ptr(),
                len: ptrs.len(),
            };
            ffi_items.push(FfiConfigUpdateReqItem {
                name: name.as_ptr(),
                values: ffi_vals,
            });
            ptr_arrays.push(ptrs);
        }
        (ffi_items, ptr_arrays)
    }
}

fn build_layer(
    layer_type: &str,
    id: u16,
    definition: &str,
    overrides: &HashMap<String, Vec<String>>
) -> *mut std::ffi::c_void {
    let mut config = parse_config_xml(definition);
    for (k, v) in overrides {
        config.insert(k.clone(), v.clone());
    }
    let cstr_config = CStringConfig::from_hashmap(&config);
    let (ffi_items, _ptrs) = cstr_config.to_ffi();
    let ffi_req = FfiConfigUpdateReq {
        data: ffi_items.as_ptr(),
        len: ffi_items.len(),
    };

    let mut lib_name = get_library_from_xml(definition);
    if lib_name.is_empty() {
        // Fallback for cases where library might not be in the XML or is bypassphy (which is native)
        lib_name = if definition.contains("bypassmac") {
            "bypassmaclayer".to_string()
        } else if definition.contains("bypassphy") {
            "".to_string()
        } else if definition.contains("rfpipemac") {
            "rfpipemaclayer".to_string()
        } else if definition.contains("ieee80211abgmac") {
            "ieee80211abgmaclayer".to_string()
        } else if definition.contains("transvirtual") {
            "transvirtual".to_string()
        } else {
            "".to_string()
        };
    }
    
    let c_lib_name = CString::new(lib_name).unwrap();

    unsafe {
        match layer_type {
            "mac" => emane_rs_ffi_build_mac_layer(id, c_lib_name.as_ptr(), ffi_req, false),
            "phy" => emane_rs_ffi_build_phy_layer(id, c_lib_name.as_ptr(), ffi_req, false),
            "transport" => emane_rs_ffi_build_transport_layer(id, c_lib_name.as_ptr(), ffi_req, false),
            "shim" => emane_rs_ffi_build_shim_layer(id, c_lib_name.as_ptr(), ffi_req, false),
            _ => std::ptr::null_mut(),
        }
    }
}

fn main() {
    let args = Args::parse();
    println!("Starting emane-rs orchestrator with config: {}", args.config_url);
    
    let mut xml_content = std::fs::read_to_string(&args.config_url).expect("Failed to read config file");
    
    // Strip <!DOCTYPE ...> since roxmltree doesn't like it
    if let Some(start) = xml_content.find("<!DOCTYPE") {
        if let Some(end) = xml_content[start..].find('>') {
            xml_content.replace_range(start..start+end+1, "");
        }
    }

    let doc = Document::parse(&xml_content).expect("Failed to parse XML");
    
    let uuid = [0u8; 16]; // Default UUID
    let manager = emane_rs_nem_manager_create(uuid.as_ptr());

    // Parse platform parameters
    for node in doc.descendants().filter(|n| n.has_tag_name("param") && n.parent().map_or(false, |p| p.has_tag_name("platform"))) {
        if let (Some(name), Some(value)) = (node.attribute("name"), node.attribute("value")) {
            let c_name = CString::new(name).unwrap();
            let c_value = CString::new(value).unwrap();
            unsafe {
                emane_rs_nem_manager_set_config_str(manager, c_name.as_ptr(), c_value.as_ptr());
            }
            if name == "controlportendpoint" {
                emane_core::control_port::start_control_port(value);
            }
        }
    }

    for node in doc.descendants().filter(|n| n.has_tag_name("nem")) {
        let id = node.attribute("id").unwrap_or("0").parse::<u16>().unwrap_or(0);
        println!("Orchestrating NEM id: {}", id);
        
        let mut layers = Vec::new();
        let empty_map = HashMap::new();

        // 1. Transport
        for child in node.children() {
            if child.has_tag_name("transport") {
                if let Some(def) = child.attribute("definition") {
                    let mut path = std::path::PathBuf::from(&args.config_url);
                    path.pop();
                    path.push(def);
                    let def_str = path.to_str().unwrap().to_string();
                    let layer = build_layer("transport", id, &def_str, &empty_map); 
                    if !layer.is_null() { layers.push(layer); }
                }
            }
        }
        
        // 2. MAC
        for child in node.children() {
            if child.has_tag_name("mac") {
                if let Some(def) = child.attribute("definition") {
                    let mut path = std::path::PathBuf::from(&args.config_url);
                    path.pop();
                    path.push(def);
                    let def_str = path.to_str().unwrap().to_string();
                    let layer = build_layer("mac", id, &def_str, &empty_map);
                    if !layer.is_null() { layers.push(layer); }
                }
            }
        }

        // 3. PHY
        for child in node.children() {
            if child.has_tag_name("phy") {
                if let Some(def) = child.attribute("definition") {
                    let mut path = std::path::PathBuf::from(&args.config_url);
                    path.pop();
                    path.push(def);
                    let def_str = path.to_str().unwrap().to_string();
                    let layer = build_layer("phy", id, &def_str, &empty_map);
                    if !layer.is_null() { layers.push(layer); }
                }
            }
        }

        // Build NEM
        let nem_config = CStringConfig::from_hashmap(&empty_map);
        let (ffi_items, _ptrs) = nem_config.to_ffi();
        let ffi_req = FfiConfigUpdateReq {
            data: ffi_items.as_ptr(),
            len: ffi_items.len(),
        };

        let nem = unsafe { emane_rs_ffi_build_nem(id, layers.as_ptr(), layers.len(), ffi_req, false) };
        
        unsafe {
            emane_rs_nem_manager_add(manager, id, nem);
        }
    }
    
    println!("Starting simulation...");
    unsafe {
        emane_rs_nem_manager_start(manager);
        emane_rs_nem_manager_post_start(manager);
    }
    
    // Park thread
    std::thread::park();
}
