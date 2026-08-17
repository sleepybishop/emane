use std::sync::{Arc, Mutex, Condvar, Once};
use std::thread;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::os::raw::c_void;

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
        other.expire_time_micros.cmp(&self.expire_time_micros)
             .then_with(|| other.event_id.cmp(&self.event_id))
    }
}

struct TimerState {
    timers: BinaryHeap<TimerEntry>,
    cancelled: HashSet<usize>,
    next_event_id: usize,
    shutdown: bool,
}

static TIMER_STATE: std::sync::OnceLock<Arc<(Mutex<TimerState>, Condvar)>> = std::sync::OnceLock::new();

fn get_timer_state() -> Arc<(Mutex<TimerState>, Condvar)> {
    TIMER_STATE.get_or_init(|| Arc::new((Mutex::new(TimerState {
        timers: BinaryHeap::new(),
        cancelled: HashSet::new(),
        next_event_id: 1,
        shutdown: false,
    }), Condvar::new()))).clone()
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

            let now_micros = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;

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
                    top.p_user
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
    
    let now_micros = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;
    
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
