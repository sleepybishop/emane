use std::env;
use std::process;
use std::thread;
use std::time::Duration;

pub struct EventServiceConfig {
    pub filename: String,
}

impl EventServiceConfig {
    pub fn new(filename: String) -> Self {
        Self { filename }
    }
}

pub struct EventGeneratorManager {}

impl EventGeneratorManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(&self) {
        println!("EventGeneratorManager started");
    }

    pub fn stop(&self) {
        println!("EventGeneratorManager stopped");
    }

    pub fn post_start(&self) {
        println!("EventGeneratorManager post_start");
    }
}

pub struct EventDirector {
    config: EventServiceConfig,
}

impl EventDirector {
    pub fn new(filename: String) -> Self {
        Self {
            config: EventServiceConfig::new(filename),
        }
    }

    pub fn construct(&self) -> EventGeneratorManager {
        EventGeneratorManager::new()
    }
}

fn usage(app_name: &str) {
    println!("usage: {} [OPTIONS]... CONFIG_URL", app_name);
    println!();
    println!(" CONFIG_URL                      URL of XML configuration file.");
    println!();
    println!("options:");
    println!("  -d, --daemonize                Run in the background.");
    println!("  -h, --help                     Print this message and exit.");
    println!("  -f, --logfile FILE             Log to a file instead of stdout.");
    println!("  -l, --loglevel [0,4]           Set initial log level.");
    println!("                                  default: 2");
    println!("  --pidfile FILE                 Write application pid to file.");
    println!("  -p, --priority [0,99]          Set realtime priority level.");
    println!("                                 Only used with -r, --realtime.");
    println!("                                  default: 50");
    println!("  -r, --realtime                 Set realtime scheduling.");
    println!("  --syslog                       Log to syslog instead of stdout.");
    println!("  --uuidfile FILE                Write the application instance UUID to file.");
    println!("  -v, --version                  Print version and exit.");
    println!();
    println!("additional options:");
    println!("  -s, --starttime TIME           Set the start time HH:MM:SS");
    println!("  -n, --nextday                  Set the start time to the next day");
    println!();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let app_name = args
        .get(0)
        .cloned()
        .unwrap_or_else(|| "emaneeventservice".to_string());

    let mut config_url = None;
    let mut starttime: Option<Duration> = None;
    let mut nextday = false;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-h" || arg == "--help" {
            usage(&app_name);
            process::exit(0);
        } else if arg == "-v" || arg == "--version" {
            println!("0.1.0"); // Dummy version
            process::exit(0);
        } else if arg == "-s" || arg == "--starttime" {
            i += 1;
            if i < args.len() {
                let time_str = &args[i];
                let parts: Vec<&str> = time_str.split(':').collect();
                if parts.len() == 3 {
                    if let (Ok(h), Ok(m), Ok(s)) = (
                        parts[0].parse::<u64>(),
                        parts[1].parse::<u64>(),
                        parts[2].parse::<u64>(),
                    ) {
                        starttime = Some(Duration::from_secs(h * 3600 + m * 60 + s));
                    } else {
                        eprintln!(
                            "Error in format of start time {} --starttime HH:MM:SS",
                            time_str
                        );
                        process::exit(1);
                    }
                } else {
                    eprintln!(
                        "Error in format of start time {} --starttime HH:MM:SS",
                        time_str
                    );
                    process::exit(1);
                }
            } else {
                eprintln!("option -s requires an argument");
                process::exit(1);
            }
        } else if arg == "-n" || arg == "--nextday" {
            nextday = true;
        } else if arg == "-d"
            || arg == "--daemonize"
            || arg == "-r"
            || arg == "--realtime"
            || arg == "--syslog"
        {
            // Flags without arguments
        } else if arg.starts_with("-") {
            // Options with arguments
            if arg == "-f"
                || arg == "--logfile"
                || arg == "-l"
                || arg == "--loglevel"
                || arg == "--pidfile"
                || arg == "-p"
                || arg == "--priority"
                || arg == "--uuidfile"
            {
                i += 1; // skip argument value
            } else {
                eprintln!("unknown option {}", arg);
                process::exit(1);
            }
        } else {
            if config_url.is_none() {
                config_url = Some(arg.clone());
            } else {
                eprintln!("Unexpected argument: {}", arg);
                process::exit(1);
            }
        }
        i += 1;
    }

    let config_url = match config_url {
        Some(url) => url,
        None => {
            eprintln!("Missing CONFIG_URL");
            process::exit(1);
        }
    };

    let director = EventDirector::new(config_url);
    let manager = director.construct();

    if let Some(st) = starttime {
        let t = unsafe { libc::time(std::ptr::null_mut()) };
        let tm_ptr = unsafe { libc::localtime(&t) };
        let seconds_passed = (unsafe { (*tm_ptr).tm_hour } * 3600
            + unsafe { (*tm_ptr).tm_min } * 60
            + unsafe { (*tm_ptr).tm_sec }) as u64;

        let mut st_secs = st.as_secs();
        if nextday {
            st_secs += 24 * 60 * 60 - seconds_passed;
        } else {
            if st_secs >= seconds_passed {
                st_secs -= seconds_passed;
            } else {
                eprintln!("Startime in the past");
                process::exit(1);
            }
        }

        thread::sleep(Duration::from_secs(st_secs));
    }

    manager.start();
    manager.post_start();

    // Typically wait here for signals before stopping

    manager.stop();
}
