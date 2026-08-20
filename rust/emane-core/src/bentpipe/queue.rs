use std::collections::BTreeMap;
use std::ffi::c_void;

pub struct QueueEntry {
    pub pkt_ptr: *mut c_void,
    pub index: usize,
    pub offset: usize,
    pub length: usize, // Cache the length from C++
}

pub struct Queue {
    pub queue: BTreeMap<u64, QueueEntry>,
    pub queue_depth: u16,
    pub fragment: bool,
    pub aggregate: bool,
    pub counter: u64,
    pub current_bytes: usize,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            queue: BTreeMap::new(),
            queue_depth: 0,
            fragment: false,
            aggregate: false,
            counter: 0,
            current_bytes: 0,
        }
    }

    pub fn enqueue(&mut self, pkt_ptr: *mut c_void, length: usize, dropped_pkt: &mut *mut c_void) -> bool {
        let mut dropped = false;
        
        if self.queue.len() == self.queue_depth as usize {
            // first candidate for overflow discard, oldest packet
            let mut key_to_remove = None;
            for (k, v) in self.queue.iter() {
                if v.index == 0 {
                    key_to_remove = Some(*k);
                    break;
                }
            }
            
            let k = key_to_remove.unwrap_or_else(|| *self.queue.keys().next().unwrap());
            let entry = self.queue.remove(&k).unwrap();
            
            *dropped_pkt = entry.pkt_ptr;
            dropped = true;
            
            self.current_bytes -= entry.length - entry.offset;
        }

        self.current_bytes += length;
        
        self.queue.insert(self.counter, QueueEntry {
            pkt_ptr,
            index: 0,
            offset: 0,
            length,
        });
        
        self.counter += 1;
        
        dropped
    }

    pub fn dequeue_impl(&mut self, requested_bytes: usize, b_drop: bool, transponder_index: u16) -> (Vec<super::queue_manager::DequeueAction>, usize) {
        let mut actions = Vec::new();
        let mut total_bytes = 0;

        while total_bytes <= requested_bytes {
            if self.queue.is_empty() {
                break;
            }
            
            let k = *self.queue.keys().next().unwrap();
            let mut entry = self.queue.remove(&k).unwrap();
            
            if entry.length - entry.offset <= requested_bytes - total_bytes {
                if entry.offset > 0 {
                    let amount = entry.length - entry.offset;
                    total_bytes += amount;
                    actions.push(super::queue_manager::DequeueAction {
                        action_type: 2, // Fragment
                        transponder_index,
                        pkt_ptr: entry.pkt_ptr,
                        seq: k,
                        fragment_index: entry.index,
                        fragment_offset: entry.offset,
                        fragment_size: amount,
                        more_fragments: false,
                    });
                } else {
                    let amount = entry.length - entry.offset;
                    total_bytes += amount;
                    actions.push(super::queue_manager::DequeueAction {
                        action_type: 1, // Full
                        transponder_index,
                        pkt_ptr: entry.pkt_ptr,
                        seq: k,
                        fragment_index: entry.index,
                        fragment_offset: entry.offset,
                        fragment_size: amount,
                        more_fragments: false,
                    });
                }
                
                if !self.aggregate {
                    break;
                }
            } else {
                if self.fragment {
                    let amount = requested_bytes - total_bytes;
                    total_bytes += amount;
                    
                    let act = super::queue_manager::DequeueAction {
                        action_type: 2, // Fragment
                        transponder_index,
                        pkt_ptr: entry.pkt_ptr,
                        seq: k,
                        fragment_index: entry.index,
                        fragment_offset: entry.offset,
                        fragment_size: amount,
                        more_fragments: true,
                    };
                    
                    entry.offset += amount;
                    entry.index += 1;
                    
                    self.queue.insert(k, entry);
                    actions.push(act);
                    
                    break;
                } else {
                    if b_drop && actions.is_empty() {
                        self.current_bytes -= entry.length;
                        actions.push(super::queue_manager::DequeueAction {
                            action_type: 0, // Drop
                            transponder_index,
                            pkt_ptr: entry.pkt_ptr,
                            seq: k,
                            fragment_index: 0,
                            fragment_offset: 0,
                            fragment_size: 0,
                            more_fragments: false,
                        });
                    } else {
                        self.queue.insert(k, entry);
                        break;
                    }
                }
            }
        }

        self.current_bytes -= total_bytes;
        (actions, total_bytes)
    }
}
