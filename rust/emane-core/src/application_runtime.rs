use crate::log_service::{
    emane_rs_log_open, emane_rs_log_redirect, emane_rs_log_set_level, redirect_to_syslog,
};
use std::ffi::CString;
use std::fs;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub daemonize: bool,
    pub realtime: bool,
    pub syslog: bool,
    pub logfile: Option<String>,
    pub loglevel: i32,
    pub pidfile: Option<String>,
    pub priority: i32,
    pub uuidfile: Option<String>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            daemonize: false,
            realtime: false,
            syslog: false,
            logfile: None,
            loglevel: 2,
            pidfile: None,
            priority: 50,
            uuidfile: None,
        }
    }
}

pub fn consume_option(
    args: &[String],
    index: &mut usize,
    options: &mut RuntimeOptions,
) -> Result<bool, String> {
    let option = args[*index].as_str();
    match option {
        "-d" | "--daemonize" => options.daemonize = true,
        "-r" | "--realtime" => options.realtime = true,
        "--syslog" => options.syslog = true,
        "-f" | "--logfile" => {
            *index += 1;
            options.logfile = Some(required_value(args, *index, option)?.to_string());
        }
        "-l" | "--loglevel" => {
            *index += 1;
            options.loglevel = required_value(args, *index, option)?
                .parse::<i32>()
                .map_err(|_| "invalid log level".to_string())?;
            if !(0..=4).contains(&options.loglevel) {
                return Err("log level must be between 0 and 4".to_string());
            }
        }
        "--pidfile" => {
            *index += 1;
            options.pidfile = Some(required_value(args, *index, option)?.to_string());
        }
        "-p" | "--priority" => {
            *index += 1;
            options.priority = required_value(args, *index, option)?
                .parse::<i32>()
                .map_err(|_| "invalid realtime priority".to_string())?;
            if !(0..=99).contains(&options.priority) {
                return Err("realtime priority must be between 0 and 99".to_string());
            }
        }
        "--uuidfile" => {
            *index += 1;
            options.uuidfile = Some(required_value(args, *index, option)?.to_string());
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn required_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

pub fn prepare(application: &str, options: &RuntimeOptions, uuid: [u8; 16]) -> Result<(), String> {
    if options.daemonize {
        if options.logfile.is_none() && !options.syslog && options.loglevel != 0 {
            return Err(
                "unable to daemonize: log level must be 0 when logging to stdout".to_string(),
            );
        }
        if unsafe { libc::daemon(1, 0) } != 0 {
            return Err(format!(
                "unable to daemonize: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    if let Some(path) = &options.logfile {
        let path = CString::new(path.as_str())
            .map_err(|_| "log file path contains a NUL byte".to_string())?;
        emane_rs_log_redirect(path.as_ptr());
    } else if options.syslog {
        redirect_to_syslog(application)?;
    }
    emane_rs_log_set_level(options.loglevel);
    if options.loglevel > 0 {
        emane_rs_log_open();
    }

    if options.realtime {
        let parameter = libc::sched_param {
            sched_priority: options.priority,
        };
        if unsafe { libc::sched_setscheduler(0, libc::SCHED_RR, &parameter) } != 0 {
            return Err(format!(
                "unable to set realtime scheduler: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    if let Some(path) = &options.pidfile {
        fs::write(path, format!("{}\n", std::process::id()))
            .map_err(|error| format!("failed to write pid file {path}: {error}"))?;
    }
    if let Some(path) = &options.uuidfile {
        fs::write(path, format!("{}\n", format_uuid(uuid)))
            .map_err(|error| format!("failed to write UUID file {path}: {error}"))?;
    }
    Ok(())
}

pub fn format_uuid(uuid: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid[0],
        uuid[1],
        uuid[2],
        uuid[3],
        uuid[4],
        uuid[5],
        uuid[6],
        uuid[7],
        uuid[8],
        uuid[9],
        uuid[10],
        uuid[11],
        uuid[12],
        uuid[13],
        uuid[14],
        uuid[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumes_and_validates_common_options() {
        let args = vec![
            "emane".to_string(),
            "--loglevel".to_string(),
            "4".to_string(),
            "-p".to_string(),
            "75".to_string(),
        ];
        let mut options = RuntimeOptions::default();
        let mut index = 1;
        assert!(consume_option(&args, &mut index, &mut options).unwrap());
        index += 1;
        assert!(consume_option(&args, &mut index, &mut options).unwrap());
        assert_eq!(options.loglevel, 4);
        assert_eq!(options.priority, 75);
    }

    #[test]
    fn formats_uuid_in_legacy_text_form() {
        assert_eq!(
            format_uuid([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            "00010203-0405-0607-0809-0a0b0c0d0e0f"
        );
    }
}
