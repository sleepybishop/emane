use emane_plugin_api::FfiFileDescriptorCallback;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};

pub const DESCRIPTOR_READ: u32 = 1;
pub const DESCRIPTOR_WRITE: u32 = 2;
pub const DESCRIPTOR_EXCEPTION: u32 = 4;

struct Registration {
    cancel: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}

fn registrations() -> &'static Mutex<HashMap<u64, Registration>> {
    static REGISTRATIONS: OnceLock<Mutex<HashMap<u64, Registration>>> = OnceLock::new();
    REGISTRATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register(
    fd: i32,
    interests: u32,
    callback_context: *mut c_void,
    callback: FfiFileDescriptorCallback,
) -> Option<u64> {
    if fd < 0 || interests == 0 || interests & !7 != 0 {
        return None;
    }
    static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed).max(1);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = Arc::clone(&cancel);
    let callback_context = callback_context as usize;
    let worker = thread::Builder::new()
        .name(format!("emane-fd-{fd}"))
        .spawn(move || {
            let requested = ((interests & DESCRIPTOR_READ != 0) as i16 * libc::POLLIN)
                | ((interests & DESCRIPTOR_WRITE != 0) as i16 * libc::POLLOUT)
                | ((interests & DESCRIPTOR_EXCEPTION != 0) as i16 * libc::POLLPRI);
            let mut descriptor = libc::pollfd {
                fd,
                events: requested,
                revents: 0,
            };
            while !worker_cancel.load(Ordering::Acquire) {
                let result = unsafe { libc::poll(&mut descriptor, 1, 100) };
                if result < 0 {
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    break;
                }
                if result == 0 {
                    continue;
                }
                let mut events = 0;
                if descriptor.revents & libc::POLLIN != 0 {
                    events |= DESCRIPTOR_READ;
                }
                if descriptor.revents & libc::POLLOUT != 0 {
                    events |= DESCRIPTOR_WRITE;
                }
                if descriptor.revents & (libc::POLLPRI | libc::POLLERR | libc::POLLHUP) != 0 {
                    events |= DESCRIPTOR_EXCEPTION;
                }
                if events != 0 && !worker_cancel.load(Ordering::Acquire) {
                    crate::nem_manager::with_component_execution(|| {
                        callback(callback_context as *mut c_void, fd, events);
                    });
                }
                descriptor.revents = 0;
            }
        })
        .ok()?;
    registrations()
        .lock()
        .ok()?
        .insert(handle, Registration { cancel, worker });
    Some(handle)
}

pub fn unregister(handle: u64) -> bool {
    let registration = registrations()
        .lock()
        .ok()
        .and_then(|mut registrations| registrations.remove(&handle));
    let Some(registration) = registration else {
        return false;
    };
    registration.cancel.store(true, Ordering::Release);
    if registration.worker.thread().id() != thread::current().id()
        && !crate::nem_manager::component_execution_active()
    {
        let _ = registration.worker.join();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    extern "C" fn ready(context: *mut c_void, _: i32, events: u32) {
        if events & DESCRIPTOR_READ != 0 {
            unsafe { &*(context as *const AtomicUsize) }.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn dispatches_readable_descriptor_and_unregisters() {
        let mut descriptors = [0; 2];
        if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
            return;
        }
        let calls = AtomicUsize::new(0);
        let handle = register(
            descriptors[0],
            DESCRIPTOR_READ,
            &calls as *const AtomicUsize as *mut c_void,
            ready,
        )
        .unwrap();
        let byte = [1u8];
        unsafe {
            libc::write(descriptors[1], byte.as_ptr().cast(), 1);
        }
        for _ in 0..50 {
            if calls.load(Ordering::Relaxed) != 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_ne!(calls.load(Ordering::Relaxed), 0);
        assert!(unregister(handle));
        unsafe {
            libc::close(descriptors[0]);
            libc::close(descriptors[1]);
        }
    }
}
