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
}

fn get_logger() -> &'static Mutex<LoggerState> {
    static LOGGER: OnceLock<Mutex<LoggerState>> = OnceLock::new();
    LOGGER.get_or_init(|| {
        Mutex::new(LoggerState {
            level: 1, // ABORT_LEVEL default
            file: None,
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

    let lvl_str = if level >= 0 && level <= 4 {
        LEVEL_STRINGS[level as usize]
    } else {
        "?"
    };

    let formatted = format!(
        "{:02}:{:02}:{:02}.{:06} {:5} {}\n",
        tm.tm_hour, tm.tm_min, tm.tm_sec, micros, lvl_str, msg_str
    );

    let mut state = get_logger().lock().unwrap();
    if let Some(ref mut f) = state.file {
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
    }
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
