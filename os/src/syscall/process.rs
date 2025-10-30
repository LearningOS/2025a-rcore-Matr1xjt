//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}
use crate::syscall::WRITE_COUNT;
use crate::syscall::EXIT_COUNT;
use crate::syscall::YIELD_COUNT;
use crate::syscall::GET_TIME_COUNT;
use crate::syscall::TRACE_COUNT;
use crate::syscall::SYSCALL_WRITE;
use crate::syscall::SYSCALL_EXIT;
use crate::syscall::SYSCALL_YIELD;
use crate::syscall::SYSCALL_GET_TIME;
use crate::syscall::SYSCALL_TRACE;
// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    match _trace_request {
        0 => {
            let res = unsafe{ *(_id as *const u8)};
            res as isize
        }
        1 => {
            unsafe {
                *(_id as *mut u8) = _data as u8;
            }
            0
        }
        2 => {
            match _id {
                SYSCALL_WRITE => unsafe { WRITE_COUNT as isize },
                SYSCALL_EXIT => unsafe { EXIT_COUNT as isize },
                SYSCALL_YIELD => unsafe { YIELD_COUNT as isize },
                SYSCALL_GET_TIME => unsafe { GET_TIME_COUNT as isize },
                SYSCALL_TRACE => unsafe { TRACE_COUNT as isize },
                _ => panic!("Unsupported syscall_id: {}", _id),
            }
        }
        _ => {
            -1
        }
    }   
}
