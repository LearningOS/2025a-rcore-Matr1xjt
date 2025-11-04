//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}

use alloc::vec;
/// Deadlock detection struct
pub struct DeadlockDetect {
    available: Vec<usize>,
    allocated: Vec<Vec<usize>>,
    need : Vec<Vec<usize>>,
}
use alloc::vec::Vec;
use lazy_static::lazy_static;    

impl DeadlockDetect {
    pub fn available_init(&mut self, resource_num: usize, total: usize) {
        self.available[resource_num] = total;
    }
    
    pub fn mutex_request(&mut self, thread_id: usize, request_id: usize, request: usize) -> bool {
        if request > self.available[request_id] {
            self.need[thread_id][request_id] += request;
            if self.detect_deadlock() {
                //println!("DeadlockDetect: thread {} request {} denied (deadlock)", thread_id, request);
                return false;
            }
            //println!("DeadlockDetect: thread {} request {} blocked ,request_id {}, resource {}", thread_id, request,request_id, self.available[request_id]);
            return true;
        }

        self.need[thread_id][request_id] += request;
        if !self.detect_deadlock() {
            self.available[request_id] -= request;
            self.allocated[thread_id][request_id] += request;
            self.need[thread_id][request_id] -= request;
            //println!("DeadlockDetect: thread {} request {} granted ,request_id {}, resource {}", thread_id, request,request_id, self.available[request_id]);
            true
        } else {
            self.need[thread_id][request_id] -= request;
            //println!("DeadlockDetect: thread {} request {} request_id {} resource {} denied (deadlock)", thread_id, request,request_id, self.available[request_id]);
            false
        }
    }

    pub fn mutex_release(&mut self, thread_id: usize, release_id: usize, release: usize) {
        if release > self.allocated[thread_id][release_id] {
            return;
        }
        // if self.detect_deadlock() {
        //     println!("DeadlockDetect: thread {} release {} denied (deadlock)", thread_id, release);
        //     return;
        // }
        self.allocated[thread_id][release_id] -= release;
        self.available[release_id] += release;
        //println!("DeadlockDetect: thread {} release {} released", thread_id, release);
    }
    pub fn detect_deadlock(&self) -> bool {
        let mut work = self.available.clone();
        //println!("available: {:?}", work);
        let mut finish = vec![false; self.allocated.len()];
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..self.allocated.len() {
                //println!("need[i][j] <= work[j] ? {:?} <= {:?}", self.need[i][0], work);
                if !finish[i] && self.need[i].iter().zip(work.iter()).all(|(n, w)| n <= w) {
                    for j in 0..work.len() {
                        work[j] += self.allocated[i][j];
                    }
                    finish[i] = true;
                    changed = true;
                }
            }
        }
        !finish.iter().all(|&f| f) // 返回true表示存在死锁
    }
}

lazy_static! {
    /// Deadlock detection instance
    pub static ref DEADLOCK_DETECT: UPSafeCell<DeadlockDetect> = unsafe {
            UPSafeCell::new(DeadlockDetect {
                available: vec![0; 256],
                allocated: vec![vec![0;256];256],
                need: vec![vec![0;256];256],
            })
        };
    }