use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_char;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::SystemTime;

struct LoggerState {
    level: i32, // 0 = NONE, 1 = ABORT, 2 = ERROR, 3 = INFO, 4 = DEBUG
    file: Option<File>,
    syslog: bool,
}

fn get_logger() -> &'static Mutex<LoggerState> {
    static LOGGER: OnceLock<Mutex<LoggerState>> = OnceLock::new();
    LOGGER.get_or_init(|| {
        Mutex::new(LoggerState {
            level: 1, // ABORT_LEVEL default
            file: None,
            syslog: false,
        })
    })
}

const LEVEL_STRINGS: [&str; 5] = ["NONE", "ABORT", "ERROR", "INFO", "DEBUG"];

#[no_mangle]
pub extern "C" fn emane_rs_log(level: i32, msg: *const c_char) {
    if !emane_rs_log_is_allowed(level) {
        return;
    }
    let msg_str = if !msg.is_null() {
        unsafe { CStr::from_ptr(msg).to_string_lossy() }
    } else {
        return;
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let total_secs = now.as_secs();
    let micros = now.subsec_micros();

    let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
    let time_t = total_secs as libc::time_t;
    unsafe {
        libc::localtime_r(&time_t, &mut tm);
    }

    let lvl_str = if (0..=4).contains(&level) {
        LEVEL_STRINGS[level as usize]
    } else {
        "?"
    };

    let formatted = format!(
        "{:02}:{:02}:{:02}.{:06} {:5} {}\n",
        tm.tm_hour, tm.tm_min, tm.tm_sec, micros, lvl_str, msg_str
    );

    let mut state = get_logger().lock().unwrap();
    if state.syslog {
        let Ok(message) = std::ffi::CString::new(msg_str.as_ref()) else {
            return;
        };
        let priority = match level {
            1 => libc::LOG_CRIT,
            2 => libc::LOG_ERR,
            3 => libc::LOG_INFO,
            _ => libc::LOG_DEBUG,
        };
        unsafe {
            libc::syslog(priority, c"%s".as_ptr(), message.as_ptr());
        }
    } else if let Some(ref mut f) = state.file {
        let _ = f.write_all(formatted.as_bytes());
    } else {
        print!("{}", formatted);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_log_set_level(level: i32) {
    let mut state = get_logger().lock().unwrap();
    state.level = level;
}

#[no_mangle]
pub extern "C" fn emane_rs_log_redirect(file: *const c_char) {
    if file.is_null() {
        return;
    }
    let f_str = unsafe { CStr::from_ptr(file).to_string_lossy() };
    if let Ok(f) = File::create(f_str.as_ref()) {
        let mut state = get_logger().lock().unwrap();
        state.file = Some(f);
        state.syslog = false;
    }
}

pub fn redirect_to_syslog(application: &str) -> Result<(), String> {
    let application = std::ffi::CString::new(application)
        .map_err(|_| "syslog application name contains a NUL byte".to_string())?;
    unsafe {
        libc::openlog(application.as_ptr(), libc::LOG_PID, libc::LOG_DAEMON);
    }
    // openlog retains the identifier pointer on some implementations. Passing
    // null makes libc use the executable name and avoids retaining Rust-owned
    // storage after this call.
    unsafe {
        libc::closelog();
        libc::openlog(std::ptr::null(), libc::LOG_PID, libc::LOG_DAEMON);
    }
    let mut state = get_logger()
        .lock()
        .map_err(|_| "logger lock poisoned".to_string())?;
    state.file = None;
    state.syslog = true;
    Ok(())
}

#[no_mangle]
pub extern "C" fn emane_rs_log_open() {
    // Nothing needed
}

#[no_mangle]
pub extern "C" fn emane_rs_log_is_allowed(level: i32) -> bool {
    let state = get_logger().lock().unwrap();
    level <= state.level
}
