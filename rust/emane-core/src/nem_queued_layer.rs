use libc::{
    epoll_create1, epoll_ctl, epoll_event, epoll_wait, eventfd, EPOLLIN, EPOLLOUT, EPOLL_CTL_ADD,
    EPOLL_CTL_DEL, EPOLL_CTL_MOD,
};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::os::raw::c_void;
use std::os::unix::io::RawFd;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct NemQueuedLayer {
    queue: Arc<Mutex<VecDeque<Box<dyn FnOnce() + Send>>>>,
    cancel: Arc<Mutex<bool>>,
    event_fd: RawFd,
    epoll_fd: RawFd,
    thread: Option<thread::JoinHandle<()>>,
    callbacks: Arc<Mutex<HashMap<RawFd, (u32, Arc<dyn Fn(RawFd) + Send + Sync>)>>>,
}

impl NemQueuedLayer {
    pub fn new(_id: u16) -> Self {
        unsafe {
            let event_fd = eventfd(0, 0);
            let epoll_fd = epoll_create1(0);

            let mut ev = epoll_event {
                events: EPOLLIN as u32,
                u64: event_fd as u64,
            };
            epoll_ctl(epoll_fd, EPOLL_CTL_ADD, event_fd, &mut ev);

            Self {
                queue: Arc::new(Mutex::new(VecDeque::new())),
                cancel: Arc::new(Mutex::new(false)),
                event_fd,
                epoll_fd,
                thread: None,
                callbacks: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    pub fn start(&mut self) {
        let queue = self.queue.clone();
        let cancel = self.cancel.clone();
        let event_fd = self.event_fd;
        let epoll_fd = self.epoll_fd;
        let callbacks = self.callbacks.clone();

        self.thread = Some(thread::spawn(move || unsafe {
            let mut events: [epoll_event; 32] = std::mem::zeroed();
            loop {
                let nfds = epoll_wait(epoll_fd, events.as_mut_ptr(), 32, -1);
                if nfds == -1 {
                    if *libc::__errno_location() == libc::EINTR {
                        continue;
                    }
                    break;
                }

                for i in 0..nfds {
                    let ev = &events[i as usize];
                    let fd = ev.u64 as RawFd;

                    if fd == event_fd {
                        let mut val: u64 = 0;
                        libc::read(event_fd, &mut val as *mut u64 as *mut c_void, 8);

                        let mut current_queue = VecDeque::new();
                        {
                            let mut q = queue.lock().unwrap();
                            std::mem::swap(&mut current_queue, &mut q);
                        }

                        if *cancel.lock().unwrap() {
                            return;
                        }

                        for task in current_queue {
                            task();
                        }
                    } else {
                        let cb = {
                            let cbs = callbacks.lock().unwrap();
                            cbs.get(&fd).map(|(_, f)| f.clone())
                        };
                        if let Some(f) = cb {
                            f(fd);
                        }
                    }
                }
            }
        }));
    }

    pub fn stop(&mut self) {
        unsafe {
            *self.cancel.lock().unwrap() = true;
            let val: u64 = 1;
            libc::write(self.event_fd, &val as *const u64 as *const c_void, 8);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        *self.cancel.lock().unwrap() = false;
    }

    pub fn enqueue(&mut self, task: Box<dyn FnOnce() + Send>) {
        self.queue.lock().unwrap().push_back(task);
        unsafe {
            let val: u64 = 1;
            libc::write(self.event_fd, &val as *const u64 as *const c_void, 8);
        }
    }

    pub fn add_fd(&mut self, fd: RawFd, is_read: bool, cb: Arc<dyn Fn(RawFd) + Send + Sync>) {
        let events = if is_read {
            EPOLLIN as u32
        } else {
            EPOLLOUT as u32
        };
        let mut callbacks = self.callbacks.lock().unwrap();

        unsafe {
            let mut ev = epoll_event {
                events,
                u64: fd as u64,
            };

            if callbacks.contains_key(&fd) {
                epoll_ctl(self.epoll_fd, EPOLL_CTL_MOD, fd, &mut ev);
            } else {
                epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, &mut ev);
            }
        }
        callbacks.insert(fd, (events, cb));
    }

    pub fn remove_fd(&mut self, fd: RawFd) {
        let mut callbacks = self.callbacks.lock().unwrap();
        if callbacks.remove(&fd).is_some() {
            unsafe {
                epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, fd, std::ptr::null_mut());
            }
        }
    }
}

impl Drop for NemQueuedLayer {
    fn drop(&mut self) {
        self.stop();
        unsafe {
            libc::close(self.event_fd);
            libc::close(self.epoll_fd);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_new(id: u16) -> *mut NemQueuedLayer {
    Box::into_raw(Box::new(NemQueuedLayer::new(id)))
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_free(ptr: *mut NemQueuedLayer) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_start(ptr: *mut NemQueuedLayer) {
    let layer = unsafe { &mut *ptr };
    layer.start();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_stop(ptr: *mut NemQueuedLayer) {
    let layer = unsafe { &mut *ptr };
    layer.stop();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_enqueue(
    ptr: *mut NemQueuedLayer,
    execute: extern "C" fn(*mut c_void),
    context: *mut c_void,
    destroy: extern "C" fn(*mut c_void),
) {
    let layer = unsafe { &mut *ptr };
    struct CTask {
        execute: extern "C" fn(*mut c_void),
        context: *mut c_void,
        destroy: extern "C" fn(*mut c_void),
    }
    unsafe impl Send for CTask {}
    impl Drop for CTask {
        fn drop(&mut self) {
            (self.destroy)(self.context);
        }
    }
    impl CTask {
        fn call(&self) {
            (self.execute)(self.context);
        }
    }
    let task = CTask {
        execute,
        context,
        destroy,
    };
    layer.enqueue(Box::new(move || {
        task.call();
    }));
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_add_fd(
    ptr: *mut NemQueuedLayer,
    fd: i32,
    is_read: bool,
    execute: extern "C" fn(i32, *mut c_void),
    context: *mut c_void,
    destroy: extern "C" fn(*mut c_void),
) {
    let layer = unsafe { &mut *ptr };
    struct CFdTask {
        execute: extern "C" fn(i32, *mut c_void),
        context: *mut c_void,
        destroy: extern "C" fn(*mut c_void),
    }
    unsafe impl Send for CFdTask {}
    unsafe impl Sync for CFdTask {}
    impl Drop for CFdTask {
        fn drop(&mut self) {
            (self.destroy)(self.context);
        }
    }
    impl CFdTask {
        fn call(&self, f: i32) {
            (self.execute)(f, self.context);
        }
    }
    let task = CFdTask {
        execute,
        context,
        destroy,
    };
    layer.add_fd(
        fd,
        is_read,
        Arc::new(move |f| {
            task.call(f);
        }),
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_queued_layer_remove_fd(ptr: *mut NemQueuedLayer, fd: i32) {
    let layer = unsafe { &mut *ptr };
    layer.remove_fd(fd);
}
