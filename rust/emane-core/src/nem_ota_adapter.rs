struct OpaqueItem(*mut std::os::raw::c_void);
unsafe impl Send for OpaqueItem {}

use std::os::raw::c_void;
use std::sync::{Arc, Mutex, Condvar};
use std::thread;

extern "C" {
    fn emane_c_nem_ota_adapter_process_item(id: u16, item: *mut c_void);
}

pub struct NemOtaAdapterState {
    id: u16,
    queue: Mutex<Vec<OpaqueItem>>,
    cond: Condvar,
    cancel: Mutex<bool>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_ota_adapter_new(id: u16) -> *mut Arc<NemOtaAdapterState> {
    let state = Arc::new(NemOtaAdapterState {
        id,
        queue: Mutex::new(Vec::new()),
        cond: Condvar::new(),
        cancel: Mutex::new(false),
        thread_handle: Mutex::new(None),
    });
    Box::into_raw(Box::new(state))
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_ota_adapter_free(ptr: *mut Arc<NemOtaAdapterState>) {
    if !ptr.is_null() {
        let state_box = unsafe { Box::from_raw(ptr) };
        let state = *state_box; // This is an Arc
        
        {
            let mut cancel = state.cancel.lock().unwrap();
            *cancel = true;
            state.cond.notify_all();
        }
        
        let handle = state.thread_handle.lock().unwrap().take();
        if let Some(h) = handle {
            let _ = h.join();
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_ota_adapter_open(ptr: *mut Arc<NemOtaAdapterState>) {
    let state = unsafe { &*ptr }.clone();
    
    {
        let mut cancel = state.cancel.lock().unwrap();
        *cancel = false;
    }
    
    let state_clone = state.clone();
    let handle = thread::spawn(move || {
        loop {
            let mut queue = state_clone.queue.lock().unwrap();
            while queue.is_empty() && !*state_clone.cancel.lock().unwrap() {
                queue = state_clone.cond.wait(queue).unwrap();
            }
            
            if *state_clone.cancel.lock().unwrap() && queue.is_empty() {
                break;
            }
            
            let items: Vec<OpaqueItem> = queue.drain(..).collect();
            drop(queue);
            
            for item in items {
                unsafe {
                    emane_c_nem_ota_adapter_process_item(state_clone.id, item.0);
                }
            }
        }
    });
    
    let mut thread_guard = state.thread_handle.lock().unwrap();
    *thread_guard = Some(handle);
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_ota_adapter_close(ptr: *mut Arc<NemOtaAdapterState>) {
    let state = unsafe { &*ptr };
    {
        let mut cancel = state.cancel.lock().unwrap();
        *cancel = true;
        state.cond.notify_all();
    }
    let handle = state.thread_handle.lock().unwrap().take();
    if let Some(h) = handle {
        let _ = h.join();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_ota_adapter_push(ptr: *mut Arc<NemOtaAdapterState>, item: *mut c_void) {
    let state = unsafe { &*ptr };
    let mut queue = state.queue.lock().unwrap();
    queue.push(OpaqueItem(item));
    state.cond.notify_one();
}
