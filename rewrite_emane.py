import sys

with open("rust/emane-core/src/bin/emane.rs", "r") as f:
    content = f.read()

# I will write a simple emane.rs that parses the args and XML,
# and uses NemManager.

new_content = """use emane_core::nem_manager::NemManager;
use emane_core::xml_parser::{parse_platform, ParamMap};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut config_url = String::new();

    // Very naive argument parser
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--daemonize" => {}
            "-r" | "--realtime" => {}
            "--syslog" => {}
            "-f" | "--logfile" => {
                i += 1;
            }
            "-l" | "--loglevel" => {
                i += 1;
            }
            "--pidfile" => {
                i += 1;
            }
            "--priority" => {
                i += 1;
            }
            "--uuidfile" => {
                i += 1;
            }
            "-v" | "--version" => {
                println!("EMANE 1.2.5 (Rust Port)");
                return;
            }
            "-h" | "--help" => {
                println!("Usage: emane [OPTIONS] platform.xml");
                return;
            }
            s if s.starts_with("-") => {
                eprintln!("Unknown option: {}", s);
                return;
            }
            s => {
                config_url = s.to_string();
            }
        }
        i += 1;
    }

    if config_url.is_empty() {
        eprintln!("Usage: emane [OPTIONS] platform.xml");
        return;
    }

    println!("Starting EMANE with config: {}", config_url);

    let platform = match parse_platform(Path::new(&config_url)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse platform XML: {:?}", e);
            return;
        }
    };

    let mut nem_manager = NemManager::new();

    for nem in platform.nems {
        for layer in nem.layers {
            // Very hacky mapping of old C++ plugin names to our new cdylib filenames!
            let lib_name = match layer.plugin_name.as_str() {
                "ieee80211abgmaclayer" => "libieee80211abg.so",
                "emane-model-bentpipe" => "libbentpipe.so",
                "rfpipemaclayer" => "librfpipe.so",
                "tdmaeventschedulerradiomodel" => "libtdma.so",
                "virtualtransport" => "libvirtual_transport.so",
                _ => {
                    println!("WARNING: Unsupported layer {}", layer.plugin_name);
                    continue;
                }
            };
            
            // Assume the libraries are available in the target/debug dir or system path
            let lib_path = format!("../../target/debug/{}", lib_name);
            nem_manager.add_layer(nem.id, &lib_path);
        }
    }

    println!("Emulator started! Parking thread.");
    std::thread::park();
}
"""

with open("rust/emane-core/src/bin/emane.rs", "w") as f:
    f.write(new_content)
