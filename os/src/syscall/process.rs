//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}
use crate::syscall::SYSCALL_WRITE;
use crate::syscall::SYSCALL_EXIT;
use crate::syscall::SYSCALL_YIELD;
use crate::syscall::SYSCALL_GET_TIME;
use crate::syscall::SYSCALL_TRACE;
use crate::task::get_syscall_count;
use crate::task::add_syscall_count;
use crate::mm::VirtAddr;
use crate::task::current_user_token;
use core::mem;
/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let us = crate::timer::get_time_us();
    let token = current_user_token();
    let pagetable = crate::mm::PageTable::from_token(token);
    let sec_addr = _ts as usize;
    let usec_addr = sec_addr + mem::size_of::<usize>();
    let sec_page = VirtAddr::from(sec_addr).floor();
    let usec_page = VirtAddr::from(usec_addr).floor();
    if let Some(pte) = pagetable.find_pte(sec_page) {
        if !pte.is_valid() || !pte.writable() {
            return -1;
        }
    } else {
        return -1;
    }

    if sec_page != usec_page {
        if let Some(pte) = pagetable.find_pte(usec_page) {
            if !pte.is_valid() || !pte.writable() {
                return -1;
            }
        } else {
            return -1;
        }
    }

    unsafe {
        if !_ts.is_null() {
            (*_ts).sec = us / 1_000_000;
            (*_ts).usec = us % 1_000_000;
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.

// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    let token = current_user_token();
    let pagetable = crate::mm::PageTable::from_token(token);
    match _trace_request {
        0 => {
            let pte = pagetable.find_pte(VirtAddr::from(_id as usize).floor()).unwrap();
            if !pte.is_valid() || !pte.readable() {
                return -1;
            }
            let res = unsafe{ *(_id as *const u8)};
            res as isize
        }
        1 => {
            unsafe {
                let pte = pagetable.find_pte(VirtAddr::from(_id as usize).floor()).unwrap();
                if !pte.is_valid() || !pte.writable() {
                    return -1;
                }
                *(_id as *mut u8) = _data as u8;
            }
            0
        }
        2 => {
            match _id {
                SYSCALL_WRITE =>  { get_syscall_count(SYSCALL_WRITE) as isize },
                SYSCALL_EXIT =>  { get_syscall_count(SYSCALL_EXIT) as isize },
                SYSCALL_YIELD =>  { get_syscall_count(SYSCALL_YIELD) as isize },
                SYSCALL_GET_TIME =>  { get_syscall_count(SYSCALL_GET_TIME) as isize },
                SYSCALL_TRACE =>  { add_syscall_count(SYSCALL_TRACE); get_syscall_count(SYSCALL_TRACE) as isize },
                _ => panic!("Unsupported syscall_id: {}", _id),
            }
        }
        _ => {
            -1
        }
    }   
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    let token = current_user_token();
    let mut pagetable = crate::mm::PageTable::from_token(token);
    for i in 0..(_len + 4095) / 4096 {
        let addr = _start + i * 4096;
        match pagetable.find_pte_create(VirtAddr::from(addr).floor()){
            None => return -1,
            Some(pte) => {
                if !pte.is_valid() {
                    *pte = crate::mm::PageTableEntry::new(
            crate::mm::PhysPageNum(_port + i),
            crate::mm::PTEFlags::R | crate::mm::PTEFlags::W | crate::mm::PTEFlags::X | crate::mm::PTEFlags::U | crate::mm::PTEFlags::V,
        );
                }
            }
        }
        
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    let token = current_user_token();
    let mut pagetable = crate::mm::PageTable::from_token(token);
    for i in 0..(_len + 4095) / 4096 {
        let addr = _start + i * 4096;
        match pagetable.find_pte_create(VirtAddr::from(addr).floor()){
            None => return -1,
            Some(pte) => {
                if pte.is_valid() {
                    *pte = crate::mm::PageTableEntry::empty();
                }
            }
        }
        
    }
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
