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
use crate::timer::get_time;
use core::mem;
use crate::mm::PageTable;
use crate::mm::page_table::translated_refmut;
/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let token = current_user_token();
    let pagetable = PageTable::from_token(token);

    // 检查 TimeVal 结构体可能跨的两页
    let start_va = VirtAddr::from(_ts as usize).floor();
    let end_va = VirtAddr::from(_ts as usize + mem::size_of::<TimeVal>() - 1).floor();

    for va in [start_va, end_va] {
        match pagetable.find_pte(va) {
            Some(pte) if pte.is_valid() && pte.writable() => {},
            _ => return -1, // 无效或不可写
        }
    }
    let ts = translated_refmut::<TimeVal>(token, _ts);
    // 获取当前时间（假设 get_time 返回毫秒）
    let current_time = get_time();
    // 写入用户空间
    
        *ts = TimeVal {
            sec: current_time / 1000,
            usec: (current_time % 1000) * 1000,
        };
    

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
            if _id >= isize::MAX as usize
            {
                return -1;
            }
            match pagetable.find_pte(VirtAddr::from(_id as usize).floor()) {
                None => return -1,
                Some(pte) => {
                    if !pte.is_valid() || !pte.readable() {
                        return -1;
                    }
                }
            }
            let id = translated_refmut::<u8>(token, _id as *mut u8);
            let res = unsafe{ *(id as *const u8)};
            res as isize
        }
        1 => {
            unsafe {
                match pagetable.find_pte(VirtAddr::from(_id as usize).floor()) {
                    None => return -1,
                    Some(pte) => {
                        if !pte.is_valid() || !pte.writable() {
                            return -1;
                        }
                    }
                }
                let id = translated_refmut::<u8>(token, _id as *mut u8);
                *(id as *mut u8) = _data as u8;
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
                //SYSCALL_GET_TIME =>  { get_syscall_count(SYSCALL_GET_TIME) as isize },
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
    if _start % 4096 != 0 || _len == 0 {
        return -1;
    }
    // 权限检查
    if _port == 0 || (_port & !0x7) != 0 {
        return -1;
    }

    let token = current_user_token();
    let mut pagetable = crate::mm::PageTable::from_token(token);
    let page_count = (_len + 4095) / 4096;

    match pagetable.find_pte(VirtAddr::from(_start+(_len-1)).floor()) {
        Some(pte) if pte.is_valid() => return -1, // 已映射
        _ => {}
    }

    for i in 0..page_count {
        let addr = _start + i * 4096;
        match pagetable.find_pte_create(VirtAddr::from(addr).floor()) {
            None => return -1,
            Some(pte) => {
                if pte.is_valid() {
                    return -1; // 已映射
                }

                let frame = match crate::mm::frame_alloc() {
                    Some(f) => f,
                    None => return -1,
                };

                let mut flags = crate::mm::page_table::PTEFlags::U | crate::mm::page_table::PTEFlags::V;
                if (_port & 1) != 0 { flags |= crate::mm::page_table::PTEFlags::R; }
                if (_port & 2) != 0 { flags |= crate::mm::page_table::PTEFlags::W; }
                if (_port & 4) != 0 { flags |= crate::mm::page_table::PTEFlags::X; }

                pagetable.map(VirtAddr::from(addr).floor(), frame.ppn, flags);
            }
        }
    }
    0
}

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    // if _start % 4096 != 0 || _len == 0 {
    //     return -1;
    // }

    // let token = current_user_token();
    // let pagetable = crate::mm::PageTable::from_token(token);
    // let page_count = (_len + 4095) / 4096;
    for i in 0..page_count {
        let addr = _start + i * 4096;
        match pagetable.find_pte(VirtAddr::from(addr).floor()) {
            None => return -1,
            Some(pte) => {
                if pte.is_valid() {
                    *pte = crate::mm::PageTableEntry::empty();
                }
                else {
                    return -1; // 未映射
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
