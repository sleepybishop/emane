use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice;

pub struct EelInputParser {}

impl EelInputParser {
    pub fn parse(input: &str) -> Result<Option<(f32, String, String, Vec<String>)>, String> {
        let mut args = Vec::new();
        let mut pos = 0;
        let bytes = input.as_bytes();

        // Skip leading whitespace
        while pos < bytes.len()
            && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n')
        {
            pos += 1;
        }

        while pos < bytes.len() {
            let mut arg_start = pos;
            let mut arg_end = pos;
            let mut is_quote = false;

            if bytes[pos] == b'\"' {
                is_quote = true;
                pos += 1;
                arg_start = pos;
                let mut found_close = false;
                while pos < bytes.len() {
                    if bytes[pos] == b'\"' {
                        arg_end = pos;
                        found_close = true;
                        pos += 1;
                        break;
                    }
                    pos += 1;
                }
                if !found_close {
                    return Err(format!("Unterminated string: {}", input));
                }
            } else if bytes[pos] == b'#' {
                break;
            } else {
                while pos < bytes.len() {
                    let c = bytes[pos];
                    if c == b' ' || c == b'\t' || c == b'\n' || c == b'#' || c == b'\"' {
                        break;
                    }
                    pos += 1;
                }
                arg_end = pos;
                if pos < bytes.len() && bytes[pos] == b'\"' && arg_start != pos {
                    return Err(format!("Invalid start of string: {}", input));
                }
            }

            if arg_start < arg_end {
                let arg = std::str::from_utf8(&bytes[arg_start..arg_end])
                    .unwrap()
                    .to_string();
                args.push(arg);
            } else if is_quote {
                args.push(String::new());
            }

            if pos < bytes.len() && bytes[pos] == b'#' {
                break;
            }

            while pos < bytes.len()
                && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n')
            {
                pos += 1;
            }
            if pos < bytes.len() && bytes[pos] == b'#' {
                break;
            }
        }

        if args.is_empty() {
            return Ok(None);
        }

        if args.len() < 3 {
            return Ok(Some((0.0, String::new(), String::new(), vec![]))); // Will be handled as false in C++ wrapper
        }

        let f_event_time = args[0].parse::<f32>().unwrap_or(0.0);
        let s_module_id = args[1].clone();
        let s_event_type = args[2].clone();
        let input_args = args[3..].to_vec();

        Ok(Some((f_event_time, s_module_id, s_event_type, input_args)))
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_input_parser_parse(
    input: *const c_char,
    f_event_time: *mut f32,
    s_event_type_out: *mut *mut c_char,
    s_module_id_out: *mut *mut c_char,
    args_out: *mut *mut *mut c_char,
    args_len_out: *mut usize,
    error_out: *mut *mut c_char,
) -> bool {
    let input_str = unsafe { CStr::from_ptr(input) }.to_string_lossy();

    match EelInputParser::parse(&input_str) {
        Ok(Some((time, module_id, event_type, args))) => {
            if time == 0.0 && module_id.is_empty() && event_type.is_empty() && args.is_empty() {
                return false;
            }
            unsafe {
                *f_event_time = time;
                *s_module_id_out = CString::new(module_id).unwrap().into_raw();
                *s_event_type_out = CString::new(event_type).unwrap().into_raw();

                let mut args_c = Vec::with_capacity(args.len());
                for a in args {
                    args_c.push(CString::new(a).unwrap().into_raw());
                }

                let mut args_c_boxed = args_c.into_boxed_slice();
                *args_out = args_c_boxed.as_mut_ptr();
                *args_len_out = args_c_boxed.len();
                std::mem::forget(args_c_boxed);
            }
            true
        }
        Ok(None) => false,
        Err(e) => {
            unsafe {
                *error_out = CString::new(e).unwrap().into_raw();
            }
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_input_parser_free_strings(
    s_event_type: *mut c_char,
    s_module_id: *mut c_char,
    args: *mut *mut c_char,
    args_len: usize,
    error: *mut c_char,
) {
    unsafe {
        if !s_event_type.is_null() {
            drop(CString::from_raw(s_event_type));
        }
        if !s_module_id.is_null() {
            drop(CString::from_raw(s_module_id));
        }
        if !error.is_null() {
            drop(CString::from_raw(error));
        }

        if !args.is_null() {
            let slice = slice::from_raw_parts_mut(args, args_len);
            for &mut ptr in slice.iter_mut() {
                if !ptr.is_null() {
                    drop(CString::from_raw(ptr));
                }
            }
            drop(Box::from_raw(slice as *mut _ as *mut [*mut c_char]));
        }
    }
}
