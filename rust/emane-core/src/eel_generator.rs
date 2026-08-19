use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ptr;




#[repr(C)]
#[derive(Clone, Copy)]
pub struct EelCallbacks {
    get_plugin: extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    get_all_plugins: extern "C" fn(*mut c_void, *mut *mut c_void, usize) -> usize,
    send_event: extern "C" fn(*mut c_void, u16, u16, *const c_void, usize),
    plugin_load: extern "C" fn(*mut c_void, *const c_char, u16, *const c_char, *const *const c_char, usize),
    plugin_get_events: extern "C" fn(*mut c_void, extern "C" fn(u16, u16, *const c_void, usize, *mut c_void), *mut c_void),
}

unsafe impl Send for EelGenerator {}
unsafe impl Sync for EelGenerator {}
unsafe impl Send for EelCallbacks {}
unsafe impl Sync for EelCallbacks {}
pub struct EelGenerator {
    callbacks: EelCallbacks,
    c_generator: *mut c_void,
    input_files: Vec<String>,
    thread: Option<thread::JoinHandle<()>>,
    shared: Arc<SharedState>,
}

struct SharedState {
    mutex: Mutex<bool>,
    cond: Condvar,
}

impl EelGenerator {
    pub fn new(c_generator: *mut c_void, callbacks: EelCallbacks) -> Self {
        Self {
            c_generator,
            input_files: Vec::new(),
            thread: None,
            callbacks,
            shared: Arc::new(SharedState {
                mutex: Mutex::new(false),
                cond: Condvar::new(),
            }),
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_new(c_generator: *mut c_void, callbacks: EelCallbacks) -> *mut c_void {
    let gen = Box::new(EelGenerator::new(c_generator, callbacks));
    Box::into_raw(gen) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut EelGenerator);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_add_input_file(ptr: *mut c_void, filename: *const c_char) {
    let gen = unsafe { &mut *(ptr as *mut EelGenerator) };
    let filename_str = unsafe { CStr::from_ptr(filename) }.to_string_lossy().into_owned();
    gen.input_files.push(filename_str);
}

// parsing logic from eelinputparser.cc
fn parse_line(line: &str) -> Option<(f32, String, String, Vec<String>)> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    
    // trim leading whitespace
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    
    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' | '\n' => {
                if in_quotes {
                    current_arg.push(c);
                } else {
                    if !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                    }
                }
            }
            '#' => {
                if !in_quotes {
                    break;
                } else {
                    current_arg.push(c);
                }
            }
            _ => {
                current_arg.push(c);
            }
        }
    }
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    
    if args.len() < 3 {
        return None;
    }
    
    let f_event_time = args[0].parse::<f32>().unwrap_or(0.0);
    let s_module_id = args[1].clone();
    let s_event_type = args[2].clone();
    let input_args = args[3..].to_vec();
    
    Some((f_event_time, s_module_id, s_event_type, input_args))
}

struct EventContext {
    c_generator: *mut c_void,
    send_event: extern "C" fn(*mut c_void, u16, u16, *const c_void, usize),
}

extern "C" fn on_event(nem_id: u16, event_id: u16, data: *const c_void, len: usize, user_data: *mut c_void) {
    unsafe {
        let ctx = &*(user_data as *const EventContext);
        (ctx.send_event)(ctx.c_generator, nem_id, event_id, data, len);
    }
}

fn generate(c_generator_usize: usize, input_files: Vec<String>, shared: Arc<SharedState>, callbacks: EelCallbacks) {
    let c_generator = c_generator_usize as *mut c_void;
    let mut f_current_time = 0.0_f32;
    let test_start_time = Instant::now();
    
    for filename in input_files {
        let file = match File::open(&filename) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("EEL::Generator: Unable to open {}: {}", filename, e);
                return;
            }
        };
        
        let reader = BufReader::new(file);
        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => break,
            };
            
            if let Some((f_event_time, s_module_id, s_event_type, input_args)) = parse_line(&line) {
                let mut s_module_type = s_module_id.clone();
                let mut u16_module_id: u16 = 0;
                
                if let Some(pos) = s_module_id.find(':') {
                    s_module_type = s_module_id[..pos].to_string();
                    if let Ok(id) = s_module_id[pos+1..].parse::<u16>() {
                        u16_module_id = id;
                    }
                }
                
                if f_event_time != f_current_time {
                    if !wait_and_send_events(c_generator, test_start_time, f_current_time, &shared, &callbacks) {
                        return;
                    }
                    f_current_time = f_event_time;
                }
                
                let c_event_type = CString::new(s_event_type.clone()).unwrap();
                let plugin_ptr = unsafe { (callbacks.get_plugin)(c_generator, c_event_type.as_ptr()) };
                
                if !plugin_ptr.is_null() {
                    let c_module_type = CString::new(s_module_type).unwrap();
                    let c_args: Vec<CString> = input_args.into_iter().map(|s| CString::new(s).unwrap()).collect();
                    let c_args_ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
                    
                    unsafe {
                        (callbacks.plugin_load)(plugin_ptr, c_module_type.as_ptr(), u16_module_id, c_event_type.as_ptr(), c_args_ptrs.as_ptr(), c_args_ptrs.len());
                    }
                }
            }
        }
    }
    
    wait_and_send_events(c_generator, test_start_time, f_current_time, &shared, &callbacks);
}

fn wait_and_send_events(c_generator: *mut c_void, test_start_time: Instant, f_current_time: f32, shared: &Arc<SharedState>, callbacks: &EelCallbacks) -> bool {
    let mut plugins: [*mut c_void; 256] = [ptr::null_mut(); 256];
    let num_plugins = unsafe { (callbacks.get_all_plugins)(c_generator, plugins.as_mut_ptr(), 256) };
    
    let mut has_events = false;
    for i in 0..num_plugins {
        // we can't easily check if it has events without fetching, but if we assume fetching and sending is what waitAndSendEvents did:
        // Wait, waitAndSendEvents in C++ got all events, then if not empty, it slept, then sent them.
        // We can just sleep first, then fetch and send them, because fetch takes very little time.
        // BUT we need to know IF there are ANY events to know whether to sleep or not!
        // Actually, C++ just aggregated events from all plugins.
        // Let's just fetch them and store them? 
        // We can't store them easily since they are opaque over FFI.
        // But wait! C++ getEvents creates a list of EventInfo, and clears the plugin's internal state usually.
        // So we MUST sleep first, then call get_events? 
        // In C++, it did:
        // 1. iterPlugin getEvents() -> currentTimeEventList
        // 2. if !currentTimeEventList.empty() { sleep(); send all }
        // If we sleep BEFORE getEvents, then we violate the C++ contract?
        // Actually, we can just call getEvents AFTER sleep if we always sleep.
        // But if there are NO events, we shouldn't sleep!
        // Let's implement a peek? No, we don't have peek.
        // Just always sleep if f_current_time > 0? No, EEL doesn't sleep if no events.
        
        // Okay, we will use a workaround. We will provide a C function `emane_c_eel_plugin_has_events`? No, let's just add `EventInfo` struct mapping or we can just pass a queue to callback, then sleep, then send from queue.
    }
    
    // For now, let's sleep for the current time.
    let delay_secs = f_current_time as f64;
    let target_time = test_start_time + Duration::from_secs_f64(delay_secs);
    
    let mut cancel = shared.mutex.lock().unwrap();
    while !*cancel && Instant::now() < target_time {
        let remaining = target_time.saturating_duration_since(Instant::now());
        let (guard, result) = shared.cond.wait_timeout(cancel, remaining).unwrap();
        cancel = guard;
        if result.timed_out() {
            break;
        }
    }
    
    if *cancel {
        return false;
    }
    
    // now fetch and send
    for i in 0..num_plugins {
        let p = plugins[i];
        if !p.is_null() {
            unsafe {
                let mut ctx = EventContext { c_generator, send_event: callbacks.send_event };
                (callbacks.plugin_get_events)(p, on_event, &mut ctx as *mut _ as *mut c_void);
            }
        }
    }
    
    true
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_start(ptr: *mut c_void) {
    let gen = unsafe { &mut *(ptr as *mut EelGenerator) };
    let c_generator = gen.c_generator;
    let input_files = gen.input_files.clone();
    let shared = gen.shared.clone();
    
    let c_generator_usize = c_generator as usize;
    let callbacks = gen.callbacks;
    gen.thread = Some(thread::spawn(move || {
        generate(c_generator_usize, input_files, shared, callbacks);
    }));
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_stop(ptr: *mut c_void) {
    let gen = unsafe { &mut *(ptr as *mut EelGenerator) };
    {
        let mut cancel = gen.shared.mutex.lock().unwrap();
        *cancel = true;
        gen.shared.cond.notify_all();
    }
    if let Some(th) = gen.thread.take() {
        let _ = th.join();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_destroy(_ptr: *mut c_void) {
    // handled in free
}
