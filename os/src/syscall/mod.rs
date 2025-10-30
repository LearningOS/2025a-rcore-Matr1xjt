//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// trace syscall
const SYSCALL_TRACE: usize = 410;

mod fs;
mod process;

use fs::*;
use process::*;
/// 记录写操作的计数
pub static mut WRITE_COUNT :usize = 0;
/// 记录退出操作的计数
pub static mut EXIT_COUNT :usize = 0;
/// 记录让出操作的计数
pub static mut YIELD_COUNT :usize = 0;
/// 记录获取时间操作的计数
pub static mut GET_TIME_COUNT :usize = 0; 
/// 记录跟踪操作的计数
pub static mut TRACE_COUNT :usize = 0;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => {
            let res = sys_write(args[0], args[1] as *const u8, args[2]);
            unsafe{WRITE_COUNT += 1;}
            res
        }
        SYSCALL_EXIT => {
            unsafe{EXIT_COUNT += 1;}
            sys_exit(args[0] as i32);
        }
        SYSCALL_YIELD => {
            let res = sys_yield();
            unsafe{YIELD_COUNT += 1;}
            res
        }
        SYSCALL_GET_TIME => {
            let res = sys_get_time(args[0] as *mut TimeVal, args[1]);
            unsafe{GET_TIME_COUNT += 1;}
            res
        }
        SYSCALL_TRACE => {
            let res = sys_trace(args[0], args[1], args[2]);
            unsafe{TRACE_COUNT += 1;}
            res
        }
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
