use emane_core::native_configuration::ConfigurationValue;
use emane_core::nem_manager::{
    canonical_plugin_name, component_configuration_defaults, component_modifiable_parameters,
    resolve_plugin_path, NemManager,
};
use emane_core::plugin_interface::{PluginApi, PluginEntryFunc, PLUGIN_ABI_VERSION};
use emane_core::statistics::{
    emane_rs_statistic_free_manifest, emane_rs_statistic_free_table_manifest,
    emane_rs_statistic_get_manifest, emane_rs_statistic_get_table_manifest,
};
use libloading::{Library, Symbol};
use roxmltree::{Document, Node};
use std::collections::HashSet;
use std::env;
use std::ffi::CStr;
use std::fs;
use std::process::{self, ExitCode};

fn usage() {
    println!("usage: emaneinfo [OPTIONS]... <plugin>|'nemmanager'|'transportmanager'|");
    println!("                              'eventagentmanager'|'eventgeneratormanager'");
    println!();
    println!(" CONFIG_URI                    URI of XML configuration file.");
    println!();
    println!("options:");
    println!("  -h, --help                     Print this message and exit.");
    println!("  -m, --manifest                 Print manifest.");
    println!("  -v, --version                  Print version and exit.");
    println!("  -c, --configuration            Parse configuration.");
}

fn plugin_type_name(plugin_type: u32) -> &'static str {
    match plugin_type {
        1 => "mac",
        2 => "phy",
        3 => "shim",
        4 => "transport",
        _ => "unknown",
    }
}

fn load_api(plugin: &str) -> Result<(Library, *const PluginApi), String> {
    let path = resolve_plugin_path(plugin)?;
    let library = unsafe { Library::new(&path) }
        .map_err(|error| format!("failed to load {}: {error}", path.display()))?;
    let entry: Symbol<PluginEntryFunc> = unsafe { library.get(b"emane_plugin_create") }
        .map_err(|error| format!("{} has no emane_plugin_create: {error}", path.display()))?;
    let api = entry();
    if api.is_null() {
        return Err(format!("{} returned a null plugin API", path.display()));
    }
    Ok((library, api))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn parameter_values(node: Node<'_, '_>) -> Vec<String> {
    if let Some(value) = node.attribute("value") {
        return vec![value.to_string()];
    }
    node.children()
        .filter(|child| child.is_element() && child.tag_name().name() == "value")
        .filter_map(|child| child.attribute("value").or_else(|| child.text()))
        .map(str::to_string)
        .collect()
}

fn configuration_from_xml(root: Node<'_, '_>) -> Vec<(String, Vec<String>)> {
    root.children()
        .filter(|node| node.is_element() && node.tag_name().name() == "param")
        .filter_map(|node| {
            node.attribute("name")
                .map(|name| (name.to_string(), parameter_values(node)))
        })
        .collect()
}

fn required_parameters(plugin: &str) -> Vec<(&'static str, ConfigurationValue)> {
    use ConfigurationValue::{String as Text, UInt16, UInt64};
    match canonical_plugin_name(plugin) {
        "emanephy" => vec![("subid", UInt16(0))],
        "rfpipe" | "ieee80211abg" | "tdma" => {
            vec![("pcrcurveuri", Text(String::new()))]
        }
        "bentpipe" => vec![
            ("pcrcurveuri", Text(String::new())),
            ("antenna.defines", Text(String::new())),
            ("transponder.receive.frequency", Text(String::new())),
            ("transponder.receive.bandwidth", Text(String::new())),
            ("transponder.receive.antenna", Text(String::new())),
            ("transponder.receive.action", Text(String::new())),
            ("transponder.receive.enable", Text(String::new())),
            ("transponder.transmit.pcrcurveindex", Text(String::new())),
            ("transponder.transmit.frequency", Text(String::new())),
            ("transponder.transmit.bandwidth", Text(String::new())),
            ("transponder.transmit.antenna", Text(String::new())),
            ("transponder.transmit.ubend.delay", Text(String::new())),
            ("transponder.transmit.datarate", Text(String::new())),
            ("transponder.transmit.power", Text(String::new())),
            ("transponder.transmit.tosmap", Text(String::new())),
            ("transponder.transmit.slotperframe", Text(String::new())),
            ("transponder.transmit.slotsize", Text(String::new())),
            ("transponder.transmit.txslots", Text(String::new())),
            ("transponder.transmit.mtu", Text(String::new())),
            ("transponder.transmit.enable", Text(String::new())),
        ],
        "phyapitestshim" => vec![("bandwidth", UInt64(0))],
        _ => Vec::new(),
    }
}

fn optional_without_defaults(plugin: &str) -> Vec<(&'static str, ConfigurationValue, usize)> {
    use ConfigurationValue::String as Text;
    match canonical_plugin_name(plugin) {
        "virtualtransport" => vec![
            ("ethernet.type.unknown.priority", Text(String::new()), 255),
            ("address", Text(String::new()), 1),
            ("mask", Text(String::new()), 1),
        ],
        "rawtransport" => vec![
            ("device", Text(String::new()), 1),
            ("ethernet.type.unknown.priority", Text(String::new()), 255),
        ],
        "commeffectshim" => vec![("filterfile", Text(String::new()), 1)],
        "phyapitestshim" => vec![
            ("frequency", Text(String::new()), 255),
            ("transmitter", Text(String::new()), 255),
            ("antennaprofile", Text(String::new()), 1),
        ],
        _ => Vec::new(),
    }
}

fn inspection_required_values(plugin: &str) -> Vec<(String, Vec<String>)> {
    let value = |name: &str, value: &str| (name.to_string(), vec![value.to_string()]);
    match canonical_plugin_name(plugin) {
        "emanephy" => vec![value("subid", "1")],
        "rfpipe" | "ieee80211abg" | "tdma" => {
            vec![value("pcrcurveuri", "file:///dev/null")]
        }
        "phyapitestshim" => vec![value("bandwidth", "1")],
        "bentpipe" => vec![
            value("pcrcurveuri", "file:///dev/null"),
            value("antenna.defines", "1:omni;0;0"),
            value("transponder.receive.frequency", "1:1"),
            value("transponder.receive.bandwidth", "1:1"),
            value("transponder.receive.antenna", "1:1"),
            value("transponder.receive.action", "1:process"),
            value("transponder.receive.enable", "1:off"),
            value("transponder.transmit.pcrcurveindex", "1:0"),
            value("transponder.transmit.frequency", "1:1"),
            value("transponder.transmit.bandwidth", "1:1"),
            value("transponder.transmit.antenna", "1:1"),
            value("transponder.transmit.ubend.delay", "1:na"),
            value("transponder.transmit.datarate", "1:1"),
            value("transponder.transmit.power", "1:0"),
            value("transponder.transmit.tosmap", "1:na"),
            value("transponder.transmit.slotperframe", "1:na"),
            value("transponder.transmit.slotsize", "1:na"),
            value("transponder.transmit.txslots", "1:na"),
            value("transponder.transmit.mtu", "1:1"),
            value("transponder.transmit.enable", "1:off"),
        ],
        _ => Vec::new(),
    }
}

fn type_name(value: &ConfigurationValue) -> &'static str {
    match value {
        ConfigurationValue::Int8(_) => "int8",
        ConfigurationValue::UInt8(_) => "uint8",
        ConfigurationValue::Int16(_) => "int16",
        ConfigurationValue::UInt16(_) => "uint16",
        ConfigurationValue::Int32(_) => "int32",
        ConfigurationValue::UInt32(_) => "uint32",
        ConfigurationValue::Int64(_) => "int64",
        ConfigurationValue::UInt64(_) => "uint64",
        ConfigurationValue::Float(_) => "float",
        ConfigurationValue::Double(_) => "double",
        ConfigurationValue::String(_) => "string",
        ConfigurationValue::Boolean(_) => "bool",
        ConfigurationValue::InetAddr(_) => "inetaddr",
    }
}

fn numeric_bounds(value: &ConfigurationValue) -> Option<(String, String)> {
    match value {
        ConfigurationValue::Int8(_) => Some((i8::MIN.to_string(), i8::MAX.to_string())),
        ConfigurationValue::UInt8(_) => Some((u8::MIN.to_string(), u8::MAX.to_string())),
        ConfigurationValue::Int16(_) => Some((i16::MIN.to_string(), i16::MAX.to_string())),
        ConfigurationValue::UInt16(_) => Some((u16::MIN.to_string(), u16::MAX.to_string())),
        ConfigurationValue::Int32(_) => Some((i32::MIN.to_string(), i32::MAX.to_string())),
        ConfigurationValue::UInt32(_) => Some((u32::MIN.to_string(), u32::MAX.to_string())),
        ConfigurationValue::Int64(_) => Some((i64::MIN.to_string(), i64::MAX.to_string())),
        ConfigurationValue::UInt64(_) => Some((u64::MIN.to_string(), u64::MAX.to_string())),
        ConfigurationValue::Float(_) => Some((f32::MIN.to_string(), f32::MAX.to_string())),
        ConfigurationValue::Double(_) => Some((f64::MIN.to_string(), f64::MAX.to_string())),
        ConfigurationValue::Boolean(_) => Some(("false".to_string(), "true".to_string())),
        ConfigurationValue::String(_) | ConfigurationValue::InetAddr(_) => None,
    }
}

fn parameter_bounds(
    plugin: &str,
    name: &str,
    prototype: &ConfigurationValue,
) -> Option<(String, String)> {
    let bounds = match canonical_plugin_name(plugin) {
        "ieee80211abg" => match name {
            "mode" => Some(("0", "3")),
            "unicastrate" | "multicastrate" => Some(("1", "12")),
            "flowcontroltokens" | "cwmin0" | "cwmin1" | "cwmin2" | "cwmin3" | "cwmax0"
            | "cwmax1" | "cwmax2" | "cwmax3" => Some(("1", "65535")),
            "aifs0" | "aifs1" | "aifs2" | "aifs3" => Some(("0", "0.000255")),
            "txop0" | "txop1" | "txop2" | "txop3" => Some(("0", "1")),
            "channelactivityestimationtimer" => Some(("0.001", "1")),
            "neighbortimeout" => Some(("0", "3600")),
            "radiometricreportinterval" => Some(("0.1", "60")),
            "neighbormetricdeletetime" => Some(("1", "3660")),
            _ => None,
        },
        "rfpipe" => match name {
            "datarate" => Some(("1", "18446744073709551615")),
            "jitter" | "delay" => Some(("0", "340282350000000000000000000000000000000")),
            "radiometricreportinterval" => Some(("0.1", "60")),
            "neighbormetricdeletetime" => Some(("1", "3660")),
            _ => None,
        },
        "tdma" => match name {
            "neighbormetricdeletetime" => Some(("1", "3660")),
            "neighbormetricupdateinterval" => Some(("0.1", "60")),
            "queue.aggregationslotthreshold" => Some(("0", "100")),
            _ => None,
        },
        "emanephy" => match name {
            "compatibilitymode" => Some(("1", "2")),
            "processingpoolsize" => Some(("0", "65535")),
            _ => None,
        },
        _ => None,
    };
    bounds
        .map(|(minimum, maximum)| (minimum.to_string(), maximum.to_string()))
        .or_else(|| numeric_bounds(prototype))
}

fn print_parameter(
    plugin: &str,
    name: &str,
    prototype: &ConfigurationValue,
    defaults: &[ConfigurationValue],
    required: bool,
    modifiable: bool,
    max_occurs: usize,
) {
    println!(
        "      <parameter name=\"{}\" default=\"{}\" required=\"{}\" modifiable=\"{}\">",
        xml_escape(name),
        if defaults.is_empty() { "no" } else { "yes" },
        if required { "yes" } else { "no" },
        if modifiable { "yes" } else { "no" }
    );
    let bounds = parameter_bounds(plugin, name, prototype);
    let tag = if bounds.is_some() {
        "numeric"
    } else {
        "nonnumeric"
    };
    if let Some((minimum, maximum)) = bounds {
        println!(
            "        <{tag} type=\"{}\" minValue=\"{minimum}\" maxValue=\"{maximum}\">",
            type_name(prototype)
        );
    } else {
        println!("        <{tag} type=\"{}\">", type_name(prototype));
    }
    println!(
        "          <values minOccurs=\"{}\" maxOccurs=\"{max_occurs}\">",
        usize::from(required)
    );
    for value in defaults {
        println!("            <value>{}</value>", xml_escape(&value.text()));
    }
    println!("          </values>");
    println!("        </{tag}>");
    println!("      </parameter>");
}

fn statistic_type_name(value: i32) -> &'static str {
    match value {
        1 => "int8",
        2 => "uint8",
        3 => "int16",
        4 => "uint16",
        5 => "int32",
        6 => "uint32",
        7 => "int64",
        8 => "uint64",
        9 => "float",
        10 => "double",
        11 => "string",
        12 => "bool",
        13 => "inetaddr",
        _ => "unknown",
    }
}

fn print_manifest(plugin: &str, build_id: Option<u16>) {
    let defaults = component_configuration_defaults(plugin);
    let modifiable: HashSet<_> = component_modifiable_parameters(plugin)
        .into_iter()
        .collect();
    let required = required_parameters(plugin);
    let required_names: HashSet<_> = required.iter().map(|(name, _)| *name).collect();
    println!("<?xml version=\"1.0\"?>");
    println!("<manifest>");
    println!("  <plugin name=\"{}\">", xml_escape(plugin));
    println!("    <configuration>");
    for (name, values) in defaults {
        if required_names.contains(name.as_str()) {
            continue;
        }
        if let Some(prototype) = values.first() {
            print_parameter(
                plugin,
                &name,
                prototype,
                &values,
                false,
                modifiable.contains(&name),
                if name == "frequencyofinterest" {
                    255
                } else {
                    1
                },
            );
        }
    }
    for (name, prototype) in required {
        print_parameter(
            plugin,
            name,
            &prototype,
            &[],
            true,
            modifiable.contains(name),
            if canonical_plugin_name(plugin) == "bentpipe" && name != "pcrcurveuri" {
                255
            } else {
                1
            },
        );
    }
    for (name, prototype, max_occurs) in optional_without_defaults(plugin) {
        print_parameter(plugin, name, &prototype, &[], false, false, max_occurs);
    }
    println!("    </configuration>");
    println!("    <statistics>");
    if let Some(build_id) = build_id {
        let manifest = emane_rs_statistic_get_manifest(build_id);
        let entries = if manifest.len == 0 || manifest.data.is_null() {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(manifest.data, manifest.len) }
        };
        for entry in entries {
            let name = unsafe { CStr::from_ptr(entry.name) }.to_string_lossy();
            let description = unsafe { CStr::from_ptr(entry.description) }.to_string_lossy();
            println!(
                "      <element name=\"{}\" type=\"{}\" clearable=\"{}\">",
                xml_escape(&name),
                statistic_type_name(entry.any_type),
                if entry.is_clearable { "yes" } else { "no" }
            );
            if !description.is_empty() {
                println!(
                    "        <description>{}</description>",
                    xml_escape(&description)
                );
            }
            println!("      </element>");
        }
        emane_rs_statistic_free_manifest(manifest);
    }
    println!("    </statistics>");
    println!("    <statistictables>");
    if let Some(build_id) = build_id {
        let manifest = emane_rs_statistic_get_table_manifest(build_id);
        let entries = if manifest.len == 0 || manifest.data.is_null() {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(manifest.data, manifest.len) }
        };
        for entry in entries {
            let name = unsafe { CStr::from_ptr(entry.name) }.to_string_lossy();
            let description = unsafe { CStr::from_ptr(entry.description) }.to_string_lossy();
            println!(
                "      <table name=\"{}\" clearable=\"{}\">",
                xml_escape(&name),
                if entry.is_clearable { "yes" } else { "no" }
            );
            if !description.is_empty() {
                println!(
                    "        <description>{}</description>",
                    xml_escape(&description)
                );
            }
            println!("      </table>");
        }
        emane_rs_statistic_free_table_manifest(manifest);
    }
    println!("    </statistictables>");
    println!("  </plugin>");
    println!("</manifest>");
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let mut configuration = false;
    let mut manifest = false;
    let mut target = None;
    for argument in &args[1..] {
        match argument.as_str() {
            "-h" | "--help" => {
                usage();
                return Ok(());
            }
            "-v" | "--version" => {
                println!("EMANE 1.2.5 (Rust Port)");
                return Ok(());
            }
            "-c" | "--configuration" => configuration = true,
            "-m" | "--manifest" => manifest = true,
            option if option.starts_with('-') => return Err(format!("unknown option: {option}")),
            value => {
                if target.replace(value.to_string()).is_some() {
                    return Err("only one plugin may be inspected".to_string());
                }
            }
        }
    }
    let target = target.ok_or_else(|| "missing plugin".to_string())?;
    let manager_names = [
        "nemmanager",
        "transportmanager",
        "eventgeneratormanager",
        "eventagentmanager",
    ];
    if !configuration && manager_names.contains(&target.as_str()) {
        if manifest {
            print_manifest(&target, None);
        }
        return Ok(());
    }

    let (plugin, declared_type, config) = if configuration {
        let content = fs::read_to_string(&target)
            .map_err(|error| format!("failed to read {target}: {error}"))?;
        let document = Document::parse(&content)
            .map_err(|error| format!("failed to parse {target}: {error}"))?;
        let root = document.root_element();
        let layer_type = root.tag_name().name().to_string();
        if !matches!(layer_type.as_str(), "mac" | "phy" | "shim" | "transport") {
            return Err(format!(
                "configuration root must be a layer, got {layer_type}"
            ));
        }
        let plugin = root
            .attribute("library")
            .or_else(|| root.attribute("plugin"))
            .unwrap_or(if layer_type == "phy" { "emanephy" } else { "" });
        if plugin.is_empty() {
            return Err(format!("{layer_type} configuration has no plugin"));
        }
        (
            plugin.to_string(),
            Some(layer_type),
            configuration_from_xml(root),
        )
    } else {
        (target.clone(), None, Vec::new())
    };

    let (kind, reported_name, _library) = if matches!(
        plugin.as_str(),
        "emanephy" | "virtualtransport" | "rawtransport" | "transvirtual" | "transraw"
    ) {
        let kind = if plugin == "emanephy" {
            "phy"
        } else {
            "transport"
        };
        (kind.to_string(), plugin.clone(), None)
    } else {
        let (library, api_ptr) = load_api(&plugin)?;
        let api = unsafe { &*api_ptr };
        if api.abi_version != PLUGIN_ABI_VERSION
            || api.struct_size != std::mem::size_of::<PluginApi>()
        {
            return Err(format!(
                "plugin ABI mismatch: version {}, size {}",
                api.abi_version, api.struct_size
            ));
        }
        let name = if api.name.is_null() {
            return Err(format!("plugin {plugin} returned a null name"));
        } else {
            unsafe { CStr::from_ptr(api.name) }
                .to_string_lossy()
                .into_owned()
        };
        (
            plugin_type_name(api.plugin_type).to_string(),
            name,
            Some(library),
        )
    };
    if let Some(declared_type) = declared_type {
        if declared_type != kind {
            return Err(format!(
                "configuration declares {declared_type}, but plugin reports {kind}"
            ));
        }
    }

    let mut inspection_config = config;
    if !configuration {
        for required in inspection_required_values(&reported_name) {
            if !inspection_config
                .iter()
                .any(|(name, _)| name == &required.0)
            {
                inspection_config.push(required);
            }
        }
    }
    if configuration {
        let supplied: HashSet<_> = inspection_config
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        for (name, _) in required_parameters(&reported_name) {
            if !supplied.contains(name) {
                return Err(format!(
                    "required configuration parameter {name} is missing"
                ));
            }
        }
    }
    let expected_type = match kind.as_str() {
        "mac" => 1,
        "phy" => 2,
        "shim" => 3,
        "transport" => 4,
        _ => return Err(format!("unsupported plugin type {kind}")),
    };
    let mut manager = NemManager::new([0; 16]);
    manager.add_layer_configured(1, &plugin, expected_type, &inspection_config)?;
    let build_id = manager
        .layer_build_ids(1)
        .into_iter()
        .next()
        .ok_or_else(|| "plugin did not receive a build id".to_string())?;
    if manifest {
        print_manifest(&reported_name, Some(build_id));
    }
    Ok(())
}

fn main() -> ExitCode {
    #[cfg(unix)]
    unsafe {
        // Match normal Unix command-line behavior when manifest output is
        // consumed by tools such as head(1), instead of panicking on EPIPE.
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("emaneinfo: {error}");
            process::ExitCode::FAILURE
        }
    }
}
