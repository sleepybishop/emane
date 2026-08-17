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

    // Use a hardcoded map for library names based on the definition filename for now, 
    // similar to how EMANE's XML parser resolves it.
    let lib_name = if definition.contains("bypassmac") {
        "bypassmaclayer"
    } else if definition.contains("bypassphy") {
        "" // Framework PHY
    } else if definition.contains("rfpipemac") {
        "rfpipemaclayer"
    } else if definition.contains("ieee80211abgmac") {
        "ieee80211abgmaclayer"
    } else if definition.contains("transvirtual") {
        "transvirtual"
    } else {
        ""
    };

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

    for node in doc.descendants().filter(|n| n.has_tag_name("nem")) {
        let id = node.attribute("id").unwrap_or("0").parse::<u16>().unwrap_or(0);
        println!("Orchestrating NEM id: {}", id);
        
        let mut layers = Vec::new();
        let empty_map = HashMap::new();

        // 1. Transport
        for child in node.children() {
            if child.has_tag_name("transport") {
                let definition = child.attribute("definition").unwrap_or("");
                let layer = build_layer("transport", id, definition, &empty_map); // Simplified override handling
                if !layer.is_null() { layers.push(layer); }
            }
        }
        
        // 2. MAC
        for child in node.children() {
            if child.has_tag_name("mac") {
                let definition = child.attribute("definition").unwrap_or("");
                let layer = build_layer("mac", id, definition, &empty_map);
                if !layer.is_null() { layers.push(layer); }
            }
        }

        // 3. PHY
        for child in node.children() {
            if child.has_tag_name("phy") {
                let definition = child.attribute("definition").unwrap_or("");
                let layer = build_layer("phy", id, definition, &empty_map);
                if !layer.is_null() { layers.push(layer); }
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
