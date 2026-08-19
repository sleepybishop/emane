use emane_core::xml_parser::{parse_platform, ParamMap, ParamValues};
use emane_core::nem_manager::NemManager;
use std::env;
use std::path::Path;

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
                if i < args.len() { log_file = Some(args[i].clone()); }
            }
            "-l" | "--loglevel" => {
                i += 1;
                if i < args.len() { log_level = args[i].parse().unwrap_or(2); }
            }
            "--pidfile" => {
                i += 1;
                if i < args.len() { pid_file = Some(args[i].clone()); }
            }
            "-p" | "--priority" => {
                i += 1;
                if i < args.len() { priority = args[i].parse().unwrap_or(50); }
            }
            "--uuidfile" => {
                i += 1;
                if i < args.len() { uuid_file = Some(args[i].clone()); }
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
    
    println!("Loaded platform config with {} NEMs", platform_config.nems.len());
    
    for nem in platform_config.nems {
        println!("Loaded NEM id={}", nem.id);
        for layer in nem.layers {
             println!("  Layer: {} (def: {:?})", layer.layer_type, layer.definition_file);
        }
    }
}
