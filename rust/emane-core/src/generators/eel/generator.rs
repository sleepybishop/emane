use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::factory::EelLoaderPluginFactory;
use super::parser::EelInputParser;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EelCallbacks {
    send_event: extern "C" fn(*mut c_void, u16, u16, *const c_void, usize),
    plugin_load:
        extern "C" fn(*mut c_void, *const c_char, u16, *const c_char, *const *const c_char, usize),
    plugin_get_events: extern "C" fn(
        *mut c_void,
        extern "C" fn(u16, u16, *const c_void, usize, *mut c_void),
        *mut c_void,
    ),
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
    factories: Vec<EelLoaderPluginFactory>,
    plugins: HashMap<String, (usize, u8)>, // eventType -> (plugin_ptr, publishMode)
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
            factories: Vec::new(),
            plugins: HashMap::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_new(
    c_generator: *mut c_void,
    callbacks: EelCallbacks,
) -> *mut EelGenerator {
    let gen = Box::new(EelGenerator::new(c_generator, callbacks));
    Box::into_raw(gen)
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_free(ptr: *mut EelGenerator) {
    if !ptr.is_null() {
        unsafe {
            let gen = Box::from_raw(ptr);
            // Destroy plugins
            for (factory, plugin_ptr) in gen.factories.iter().zip(gen.plugins.values().map(|v| v.0))
            {
                factory.destroy_plugin(plugin_ptr as *mut c_void);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_add_input_file(
    ptr: *mut EelGenerator,
    filename: *const c_char,
) {
    let gen = unsafe { &mut *ptr };
    let filename_str = unsafe { CStr::from_ptr(filename) }
        .to_string_lossy()
        .into_owned();
    gen.input_files.push(filename_str);
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_add_loader(
    ptr: *mut EelGenerator,
    loader_str: *const c_char,
    error_out: *mut *mut c_char,
) -> bool {
    let gen = unsafe { &mut *ptr };
    let loader = unsafe { CStr::from_ptr(loader_str) }
        .to_string_lossy()
        .into_owned();

    // format: EventTypes:LibraryName[:PublishMode]
    let parts: Vec<&str> = loader.split(':').collect();
    if parts.len() < 2 {
        let err = format!(
            "EEL::Generator: Bad configuration 'loader' format {}",
            loader
        );
        unsafe {
            *error_out = CString::new(err).unwrap().into_raw();
        }
        return false;
    }

    let event_types = parts[0];
    let library_name = parts[1];
    let mut publish_mode = 0; // DELTA

    if parts.len() > 2 {
        let mode_str = parts[2];
        if mode_str == "delta" {
            publish_mode = 0;
        } else if mode_str == "full" {
            publish_mode = 1;
        } else {
            let err = format!("EEL::Generator: Unknown 'loader' publish mode {}", mode_str);
            unsafe {
                *error_out = CString::new(err).unwrap().into_raw();
            }
            return false;
        }
    }

    let lib_filename = format!("lib{}.so", library_name);
    match EelLoaderPluginFactory::construct(&lib_filename) {
        Ok(factory) => {
            let plugin_ptr = factory.create_plugin();
            gen.factories.push(factory);

            for ev_type in event_types.split(',') {
                gen.plugins
                    .insert(ev_type.to_string(), (plugin_ptr as usize, publish_mode));
            }
            true
        }
        Err(e) => {
            let err = format!("EEL::Generator: Factory exception {}", e);
            unsafe {
                *error_out = CString::new(err).unwrap().into_raw();
            }
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_get_plugin(
    ptr: *const EelGenerator,
    event_type: *const c_char,
) -> *mut c_void {
    let gen = unsafe { &*ptr };
    let ev_str = unsafe { CStr::from_ptr(event_type) }.to_string_lossy();
    if let Some(&(p, _)) = gen.plugins.get(ev_str.as_ref()) {
        p as *mut c_void
    } else {
        ptr::null_mut()
    }
}

struct EventContext {
    c_generator: *mut c_void,
    send_event: extern "C" fn(*mut c_void, u16, u16, *const c_void, usize),
}

extern "C" fn on_event(
    nem_id: u16,
    event_id: u16,
    data: *const c_void,
    len: usize,
    user_data: *mut c_void,
) {
    unsafe {
        let ctx = &*(user_data as *const EventContext);
        (ctx.send_event)(ctx.c_generator, nem_id, event_id, data, len);
    }
}

fn generate(
    c_generator_usize: usize,
    input_files: Vec<String>,
    shared: Arc<SharedState>,
    callbacks: EelCallbacks,
    plugins_map: HashMap<String, (usize, u8)>,
) {
    let c_generator = c_generator_usize as *mut c_void;
    let mut f_current_time = 0.0_f32;
    let test_start_time = Instant::now();

    // gather unique plugins
    let mut unique_plugins = Vec::new();
    for &val in plugins_map.values() {
        if !unique_plugins.contains(&val) {
            unique_plugins.push(val);
        }
    }

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

            if let Ok(Some((f_event_time, s_module_id, s_event_type, input_args))) =
                EelInputParser::parse(&line)
            {
                if f_event_time == 0.0
                    && s_module_id.is_empty()
                    && s_event_type.is_empty()
                    && input_args.is_empty()
                {
                    continue;
                }

                let mut s_module_type = s_module_id.clone();
                let mut u16_module_id: u16 = 0;

                if let Some(pos) = s_module_id.find(':') {
                    s_module_type = s_module_id[..pos].to_string();
                    if let Ok(id) = s_module_id[pos + 1..].parse::<u16>() {
                        u16_module_id = id;
                    }
                }

                if f_event_time != f_current_time {
                    if !wait_and_send_events(
                        c_generator,
                        test_start_time,
                        f_current_time,
                        &shared,
                        &callbacks,
                        &unique_plugins,
                    ) {
                        return;
                    }
                    f_current_time = f_event_time;
                }

                if let Some(&(plugin_ptr_usize, _)) = plugins_map.get(&s_event_type) {
                    let c_event_type = CString::new(s_event_type).unwrap();
                    let c_module_type = CString::new(s_module_type).unwrap();
                    let c_args: Vec<CString> = input_args
                        .into_iter()
                        .map(|s| CString::new(s).unwrap())
                        .collect();
                    let c_args_ptrs: Vec<*const c_char> =
                        c_args.iter().map(|s| s.as_ptr()).collect();

                    (callbacks.plugin_load)(
                        plugin_ptr_usize as *mut c_void,
                        c_module_type.as_ptr(),
                        u16_module_id,
                        c_event_type.as_ptr(),
                        c_args_ptrs.as_ptr(),
                        c_args_ptrs.len(),
                    );
                }
            }
        }
    }

    wait_and_send_events(
        c_generator,
        test_start_time,
        f_current_time,
        &shared,
        &callbacks,
        &unique_plugins,
    );
}

fn wait_and_send_events(
    c_generator: *mut c_void,
    test_start_time: Instant,
    f_current_time: f32,
    shared: &Arc<SharedState>,
    callbacks: &EelCallbacks,
    plugins: &Vec<(usize, u8)>,
) -> bool {
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

    for &(p_usize, _publish_mode) in plugins {
        let p = p_usize as *mut c_void;
        if !p.is_null() {
            let mut ctx = EventContext {
                c_generator,
                send_event: callbacks.send_event,
            };
            (callbacks.plugin_get_events)(p, on_event, &mut ctx as *mut _ as *mut c_void);
        }
    }

    true
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_start(ptr: *mut EelGenerator) {
    let gen = unsafe { &mut *ptr };
    let c_generator = gen.c_generator;
    let input_files = gen.input_files.clone();
    let shared = gen.shared.clone();
    let callbacks = gen.callbacks;
    let plugins = gen.plugins.clone();

    let c_generator_usize = c_generator as usize;
    let c_gen_sync = c_generator as usize;
    gen.thread = Some(thread::spawn(move || {
        let _c_generator = c_gen_sync as *mut c_void;
        generate(c_generator_usize, input_files, shared, callbacks, plugins);
    }));
}

#[no_mangle]
pub extern "C" fn emane_rs_eel_generator_stop(ptr: *mut EelGenerator) {
    let gen = unsafe { &mut *ptr };
    {
        let mut cancel = gen.shared.mutex.lock().unwrap();
        *cancel = true;
        gen.shared.cond.notify_all();
    }
    if let Some(th) = gen.thread.take() {
        let _ = th.join();
    }
}
