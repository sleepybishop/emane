use emane_core::nem_manager::resolve_plugin_path;
use emane_core::plugin_interface::{PluginApi, PluginEntryFunc, PLUGIN_ABI_VERSION};
use libloading::{Library, Symbol};
use roxmltree::Document;
use std::env;
use std::ffi::CStr;
use std::fs;
use std::process::{self, ExitCode};

fn usage() {
    println!("usage: emaneinfo [OPTIONS] <plugin|configuration.xml>");
    println!("  -c, --configuration  Treat the argument as a layer configuration");
    println!("  -m, --manifest       Print a machine-readable plugin manifest");
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

    let (plugin, declared_type) = if configuration {
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
        (plugin.to_string(), Some(layer_type))
    } else {
        (target, None)
    };

    if plugin == "emanephy" || plugin == "virtualtransport" || plugin == "rawtransport" {
        let kind = declared_type.unwrap_or_else(|| {
            if plugin == "emanephy" {
                "phy".to_string()
            } else {
                "transport".to_string()
            }
        });
        if manifest {
            println!("<plugin name=\"{plugin}\" type=\"{kind}\" builtin=\"true\"/>");
        } else {
            println!("name: {plugin}\ntype: {kind}\nbuiltin: true");
        }
        return Ok(());
    }

    let (_library, api_ptr) = load_api(&plugin)?;
    let api = unsafe { &*api_ptr };
    if api.abi_version != PLUGIN_ABI_VERSION || api.struct_size != std::mem::size_of::<PluginApi>()
    {
        return Err(format!(
            "plugin ABI mismatch: version {}, size {}",
            api.abi_version, api.struct_size
        ));
    }
    let name = if api.name.is_null() {
        return Err(format!("plugin {plugin} returned a null name"));
    } else {
        unsafe { CStr::from_ptr(api.name) }.to_string_lossy()
    };
    let kind = plugin_type_name(api.plugin_type);
    if let Some(declared_type) = declared_type {
        if declared_type != kind {
            return Err(format!(
                "configuration declares {declared_type}, but plugin reports {kind}"
            ));
        }
    }
    if manifest {
        println!("<plugin name=\"{name}\" type=\"{kind}\" builtin=\"false\"/>");
    } else {
        println!("name: {name}\ntype: {kind}\nbuiltin: false");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("emaneinfo: {error}");
            process::ExitCode::FAILURE
        }
    }
}
