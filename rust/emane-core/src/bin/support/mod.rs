use roxmltree::Document;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_shutdown(_: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

fn install_signal_handlers() {
    unsafe {
        let handler = handle_shutdown as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
    }
}

fn parse_start_time(value: &str) -> Result<u64, String> {
    let fields: Vec<_> = value.split(':').collect();
    if fields.len() != 3 {
        return Err("--starttime must use HH:MM:SS".to_string());
    }
    let hour: u64 = fields[0]
        .parse()
        .map_err(|_| "invalid start hour".to_string())?;
    let minute: u64 = fields[1]
        .parse()
        .map_err(|_| "invalid start minute".to_string())?;
    let second: u64 = fields[2]
        .parse()
        .map_err(|_| "invalid start second".to_string())?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err("--starttime is outside 00:00:00..23:59:59".to_string());
    }
    Ok(hour * 3600 + minute * 60 + second)
}

pub fn run_event_application(expected_root: &str, component_tag: &str) -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    let mut config = None;
    let mut start_time = None;
    let mut next_day = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => {
                println!("usage: {} [OPTIONS] CONFIG_URL", args[0]);
                return Ok(());
            }
            "-v" | "--version" => {
                println!("EMANE 1.2.5 (Rust Port)");
                return Ok(());
            }
            "-s" | "--starttime" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--starttime requires a value".to_string())?;
                start_time = Some(parse_start_time(value)?);
            }
            "-n" | "--nextday" => next_day = true,
            "-d" | "--daemonize" | "-r" | "--realtime" | "--syslog" => {}
            "-f" | "--logfile" | "-l" | "--loglevel" | "--pidfile" | "-p" | "--priority"
            | "--uuidfile" => {
                index += 1;
                if index == args.len() {
                    return Err(format!("{} requires a value", args[index - 1]));
                }
            }
            option if option.starts_with('-') => return Err(format!("unknown option {option}")),
            value => {
                if config.replace(value.to_string()).is_some() {
                    return Err("only one configuration may be supplied".to_string());
                }
            }
        }
        index += 1;
    }

    let config = config.ok_or_else(|| "missing CONFIG_URL".to_string())?;
    let path = Path::new(&config);
    let content =
        fs::read_to_string(path).map_err(|error| format!("failed to read {config}: {error}"))?;
    let document =
        Document::parse(&content).map_err(|error| format!("failed to parse {config}: {error}"))?;
    let root = document.root_element();
    if root.tag_name().name() != expected_root {
        return Err(format!("configuration root must be {expected_root}"));
    }
    let mut component_count = 0;
    for component in root
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == component_tag)
    {
        if let Some(definition) = component.attribute("definition") {
            let definition_path = path.parent().unwrap_or(Path::new("")).join(definition);
            let definition_content = fs::read_to_string(&definition_path).map_err(|error| {
                format!("failed to read {}: {error}", definition_path.display())
            })?;
            Document::parse(&definition_content).map_err(|error| {
                format!("failed to parse {}: {error}", definition_path.display())
            })?;
        }
        component_count += 1;
    }
    if component_count == 0 {
        return Err(format!(
            "configuration contains no {component_tag} components"
        ));
    }

    if let Some(target) = start_time {
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        let mut local: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&now, &mut local) }.is_null() {
            return Err("unable to determine local time".to_string());
        }
        let current = (local.tm_hour * 3600 + local.tm_min * 60 + local.tm_sec) as u64;
        let delay = if next_day {
            24 * 3600 - current + target
        } else if target >= current {
            target - current
        } else {
            return Err("start time is in the past (use --nextday)".to_string());
        };
        std::thread::sleep(Duration::from_secs(delay));
    }

    install_signal_handlers();
    println!(
        "{} started with {} {} component(s)",
        expected_root, component_count, component_tag
    );
    while !SHUTDOWN.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}
