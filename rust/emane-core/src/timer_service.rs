use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::os::raw::c_void;
use std::sync::{Arc, Condvar, Mutex, Once};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub type TimerCallback = extern "C" fn(
    event_id: usize,
    expire_time_micros: u64,
    schedule_time_micros: u64,
    fire_time_micros: u64,
    arg: *const c_void,
    p_timer_service_user: *mut c_void,
);

pub type TimerFreeCallback = extern "C" fn(p_user: *mut c_void);

#[derive(Clone)]
struct TimerEntry {
    event_id: usize,
    expire_time_micros: u64,
    schedule_time_micros: u64,
    interval_micros: u64,
    arg: *const c_void,
    p_user: *mut c_void,
    callback: TimerCallback,
    free_callback: Option<TimerFreeCallback>,
}

unsafe impl Send for TimerEntry {}
unsafe impl Sync for TimerEntry {}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.expire_time_micros == other.expire_time_micros && self.event_id == other.event_id
    }
}
impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (earliest expiration time first)
        other
            .expire_time_micros
            .cmp(&self.expire_time_micros)
            .then_with(|| other.event_id.cmp(&self.event_id))
    }
}

struct TimerState {
    timers: BinaryHeap<TimerEntry>,
    cancelled: HashSet<usize>,
    next_event_id: usize,
    shutdown: bool,
}

static TIMER_STATE: std::sync::OnceLock<Arc<(Mutex<TimerState>, Condvar)>> =
    std::sync::OnceLock::new();

fn get_timer_state() -> Arc<(Mutex<TimerState>, Condvar)> {
    TIMER_STATE
        .get_or_init(|| {
            Arc::new((
                Mutex::new(TimerState {
                    timers: BinaryHeap::new(),
                    cancelled: HashSet::new(),
                    next_event_id: 1,
                    shutdown: false,
                }),
                Condvar::new(),
            ))
        })
        .clone()
}

static INIT_THREAD: Once = Once::new();

fn timer_thread() {
    let pair = get_timer_state();
    let mut state = pair.0.lock().unwrap();

    loop {
        if state.shutdown {
            break;
        }

        if let Some(mut top) = state.timers.peek().cloned() {
            if state.cancelled.contains(&top.event_id) {
                state.timers.pop();
                state.cancelled.remove(&top.event_id);
                if let Some(free_cb) = top.free_callback {
                    drop(state);
                    free_cb(top.p_user);
                    state = pair.0.lock().unwrap();
                }
                continue;
            }

            let now_micros = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64;

            if top.expire_time_micros <= now_micros {
                // Fire timer
                state.timers.pop(); // Remove from queue

                // Drop lock while calling callback
                drop(state);
                (top.callback)(
                    top.event_id,
                    top.expire_time_micros,
                    top.schedule_time_micros,
                    now_micros, // fire_time
                    top.arg,
                    top.p_user,
                );

                // Re-acquire lock
                state = pair.0.lock().unwrap();

                // If interval > 0, re-schedule
                if top.interval_micros > 0 && !state.cancelled.contains(&top.event_id) {
                    top.schedule_time_micros = now_micros;
                    top.expire_time_micros = now_micros + top.interval_micros;
                    state.timers.push(top);
                } else {
                    state.cancelled.remove(&top.event_id);
                    if let Some(free_cb) = top.free_callback {
                        drop(state);
                        free_cb(top.p_user);
                        state = pair.0.lock().unwrap();
                    }
                }
            } else {
                // Wait until expiration
                let wait_dur = Duration::from_micros(top.expire_time_micros - now_micros);
                let (new_state, _timeout_result) = pair.1.wait_timeout(state, wait_dur).unwrap();
                state = new_state;
            }
        } else {
            // No timers, wait for new ones
            state = pair.1.wait(state).unwrap();
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_timer_schedule(
    expire_micros: u64,
    interval_micros: u64,
    arg: *const c_void,
    p_user: *mut c_void,
    callback: TimerCallback,
    free_callback: Option<TimerFreeCallback>,
) -> usize {
    INIT_THREAD.call_once(|| {
        thread::spawn(timer_thread);
    });

    let pair = get_timer_state();
    let mut state = pair.0.lock().unwrap();

    let event_id = state.next_event_id;
    state.next_event_id += 1;

    let now_micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    let entry = TimerEntry {
        event_id,
        expire_time_micros: expire_micros,
        schedule_time_micros: now_micros,
        interval_micros,
        arg,
        p_user,
        callback,
        free_callback,
    };

    // Check if we need to wake up thread immediately (if this timer is earlier than existing ones)
    let wake_thread = if let Some(top) = state.timers.peek() {
        expire_micros < top.expire_time_micros
    } else {
        true
    };

    state.timers.push(entry);

    if wake_thread {
        pair.1.notify_one();
    }

    event_id
}

#[no_mangle]
pub extern "C" fn emane_rs_timer_cancel(event_id: usize) -> bool {
    let pair = get_timer_state();
    let mut state = pair.0.lock().unwrap();

    state.cancelled.insert(event_id);
    // Notify thread so that it can remove the cancelled timer immediately if it was the head
    pair.1.notify_one();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    extern "C" fn test_callback(
        event_id: usize,
        _expire_time_micros: u64,
        _schedule_time_micros: u64,
        _fire_time_micros: u64,
        arg: *const c_void,
        _p_timer_service_user: *mut c_void,
    ) {
        let sender: &mpsc::Sender<usize> = unsafe { &*(arg as *const mpsc::Sender<usize>) };
        let _ = sender.send(event_id);
    }

    extern "C" fn test_free_callback(p_user: *mut c_void) {
        if !p_user.is_null() {
            unsafe {
                let boxed_arc = Box::from_raw(p_user as *mut Arc<AtomicBool>);
                boxed_arc.store(true, Ordering::SeqCst);
            }
        }
    }

    fn now_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    fn wait_for_bool(flag: &Arc<AtomicBool>, timeout_ms: u64) -> bool {
        let mut elapsed = 0;
        while !flag.load(Ordering::SeqCst) && elapsed < timeout_ms {
            std::thread::sleep(Duration::from_millis(10));
            elapsed += 10;
        }
        flag.load(Ordering::SeqCst)
    }

    #[test]
    fn test_timer_schedule() {
        let (tx, rx) = mpsc::channel::<usize>();
        let tx_box = Box::new(tx);
        let arg = Box::into_raw(tx_box) as *const c_void;

        let expire = now_micros() + 10_000; // 10ms

        let id = emane_rs_timer_schedule(expire, 0, arg, std::ptr::null_mut(), test_callback, None);

        let received_id = rx
            .recv_timeout(Duration::from_millis(1000))
            .expect("Timer did not fire");
        assert_eq!(id, received_id);

        unsafe {
            let _ = Box::from_raw(arg as *mut mpsc::Sender<usize>);
        }
    }

    #[test]
    fn test_timer_cancel() {
        let (tx, rx) = mpsc::channel::<()>();
        let tx_box = Box::new(tx);
        let arg = Box::into_raw(tx_box) as *const c_void;

        let expire = now_micros() + 100_000; // 100ms

        let id = emane_rs_timer_schedule(expire, 0, arg, std::ptr::null_mut(), test_callback, None);

        let cancelled = emane_rs_timer_cancel(id);
        assert!(cancelled);

        let result = rx.recv_timeout(Duration::from_millis(200));
        assert!(result.is_err(), "Timer should have been cancelled");

        unsafe {
            let _ = Box::from_raw(arg as *mut mpsc::Sender<usize>);
        }
    }

    #[test]
    fn test_timer_interval() {
        let (tx, rx) = mpsc::channel::<usize>();
        let tx_box = Box::new(tx);
        let arg = Box::into_raw(tx_box) as *const c_void;

        let expire = now_micros() + 20_000; // 20ms
        let interval = 20_000; // 20ms

        let id = emane_rs_timer_schedule(
            expire,
            interval,
            arg,
            std::ptr::null_mut(),
            test_callback,
            None,
        );

        let mut count = 0;
        for _ in 0..3 {
            let received_id = rx
                .recv_timeout(Duration::from_millis(1000))
                .expect("Interval timer did not fire");
            assert_eq!(id, received_id);
            count += 1;
        }
        assert_eq!(count, 3);

        emane_rs_timer_cancel(id);

        let result = rx.recv_timeout(Duration::from_millis(100));
        assert!(result.is_err(), "Interval timer should have been cancelled");

        unsafe {
            let _ = Box::from_raw(arg as *mut mpsc::Sender<usize>);
        }
    }

    #[test]
    fn test_timer_free_callback_on_fire() {
        let (tx, rx) = mpsc::channel::<usize>();
        let tx_box = Box::new(tx);
        let arg = Box::into_raw(tx_box) as *const c_void;

        let freed_flag = Arc::new(AtomicBool::new(false));
        let p_user = Box::into_raw(Box::new(freed_flag.clone())) as *mut c_void;

        let expire = now_micros() + 10_000; // 10ms

        let id = emane_rs_timer_schedule(
            expire,
            0,
            arg,
            p_user,
            test_callback,
            Some(test_free_callback),
        );

        let received_id = rx
            .recv_timeout(Duration::from_millis(1000))
            .expect("Timer did not fire");
        assert_eq!(id, received_id);

        assert!(
            wait_for_bool(&freed_flag, 1000),
            "Free callback was not called after timer fired"
        );

        unsafe {
            let _ = Box::from_raw(arg as *mut mpsc::Sender<usize>);
        }
    }

    #[test]
    fn test_timer_free_callback_on_cancel() {
        let (tx, _rx) = mpsc::channel::<usize>();
        let tx_box = Box::new(tx);
        let arg = Box::into_raw(tx_box) as *const c_void;

        let freed_flag = Arc::new(AtomicBool::new(false));
        let p_user = Box::into_raw(Box::new(freed_flag.clone())) as *mut c_void;

        let expire = now_micros() + 500_000; // 500ms

        let id = emane_rs_timer_schedule(
            expire,
            0,
            arg,
            p_user,
            test_callback,
            Some(test_free_callback),
        );

        emane_rs_timer_cancel(id);

        assert!(
            wait_for_bool(&freed_flag, 1500),
            "Free callback was not called after cancel"
        );

        unsafe {
            let _ = Box::from_raw(arg as *mut mpsc::Sender<usize>);
        }
    }
}
