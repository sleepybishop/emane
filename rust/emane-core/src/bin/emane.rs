use emane_core::nem_manager::NemManager;
use emane_core::xml_parser::parse_platform;
use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_shutdown(_: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

fn install_signal_handlers() {
    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_shutdown as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            handle_shutdown as *const () as libc::sighandler_t,
        );
    }
}

fn usage() {
    println!("Usage: emane [OPTIONS] platform.xml");
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let mut config_url = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-d" | "--daemonize" | "-r" | "--realtime" | "--syslog" => {}
            "-f" | "--logfile" | "-l" | "--loglevel" | "--pidfile" | "--priority"
            | "--uuidfile" => {
                index += 1;
                if index == args.len() {
                    return Err(format!("{} requires a value", args[index - 1]));
                }
            }
            "-v" | "--version" => {
                println!("EMANE 1.2.5 (Rust Port)");
                return Ok(());
            }
            "-h" | "--help" => {
                usage();
                return Ok(());
            }
            option if option.starts_with('-') => return Err(format!("unknown option: {option}")),
            value => {
                if config_url.replace(value.to_string()).is_some() {
                    return Err("only one platform configuration may be supplied".to_string());
                }
            }
        }
        index += 1;
    }

    let config_url = config_url.ok_or_else(|| "missing platform configuration".to_string())?;
    let platform = parse_platform(Path::new(&config_url)).map_err(|error| error.to_string())?;
    if platform.nems.is_empty() {
        return Err("platform contains no NEMs".to_string());
    }

    let mut manager = NemManager::new(rand::random());
    let mut nem_ids = HashSet::new();
    for nem in platform.nems {
        if !nem_ids.insert(nem.id) {
            return Err(format!("duplicate NEM id {}", nem.id));
        }
        for layer in nem.layers {
            let expected_type = match layer.layer_type.as_str() {
                "mac" => 1,
                "phy" => 2,
                "shim" => 3,
                "transport" => 4,
                other => return Err(format!("NEM {} has unknown layer type {other}", nem.id)),
            };
            let default_plugin = match expected_type {
                2 => "emanephy",
                4 => "virtualtransport",
                _ => "",
            };
            let plugin = layer.plugin.as_deref().unwrap_or(default_plugin);
            if plugin.is_empty() {
                return Err(format!(
                    "NEM {} {} layer has no plugin",
                    nem.id, layer.layer_type
                ));
            }
            let config: Vec<_> = layer
                .params
                .into_iter()
                .map(|(name, values)| (name, values.values))
                .collect();
            manager
                .add_layer_configured(nem.id, plugin, expected_type, &config)
                .map_err(|error| {
                    format!(
                        "failed to construct NEM {} {} layer ({plugin}): {error}",
                        nem.id, layer.layer_type
                    )
                })?;
        }
    }

    install_signal_handlers();
    manager.start()?;
    manager.post_start();
    println!(
        "EMANE started with {} NEM(s); press Ctrl-C to stop",
        nem_ids.len()
    );
    while !SHUTDOWN.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
    }
    manager.stop();
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("emane: {error}");
            ExitCode::FAILURE
        }
    }
}
