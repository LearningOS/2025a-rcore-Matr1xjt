//! Process management syscalls
use alloc::sync::Arc;
const BIG_STRIDE: usize = 0x7fffffff;
use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    // println!(
    //     "kernel:pid[{}] sys_waitpid called with pid {} and exit_code_ptr {:x}",
    //     current_task().unwrap().pid.0,
    //     pid,
    //     exit_code_ptr as usize
    // );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        // println!(
        //     "kernel:pid[{}] sys_waitpid found no child with pid {}",
        //     task.pid.0,
        //     pid
        // );
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        // println!(
        //     "kernel:pid[{}] sys_waitpid found zombie child pid[{}] with exit code {}",
        //     task.pid.0,
        //     found_pid,
        //     exit_code
        // );
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}
use crate::mm::VirtAddr;
// use crate::task::current_user_token;
use crate::timer::get_time;
use core::mem;
// use crate::mm::page_table::translated_refmut;
/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let token = current_user_token();
    let pagetable = crate::mm::page_table::PageTable::from_token(token);

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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    if _start % 4096 != 0 || _len == 0 {
        return -1;
    }
    // 权限检查
    if _port == 0 || (_port & !0x7) != 0 {
        return -1;
    }

    let token = current_user_token();
    let mut pagetable = crate::mm::page_table::PageTable::from_token(token);
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

    let token = current_user_token();
    let pagetable = crate::mm::page_table::PageTable::from_token(token);
    let page_count = (_len + 4095) / 4096;
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
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
use crate::task::TaskControlBlock;
/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task = Arc::new(TaskControlBlock::new(data));
        let new_pid = new_task.pid.0;
        let current_task = current_task().unwrap();
        new_task.inner_exclusive_access().parent = Some(Arc::downgrade(&current_task));
        current_task
            .inner_exclusive_access()
            .children
            .push(Arc::clone(&new_task));
        add_task(new_task.into());
        // new_task.exec(data);
        return new_pid as isize;
    } else {
        return -1;
    }
}


// syscall ID：140
// 设置当前进程优先级为 prio
// 参数：prio 进程优先级，要求 prio >= 2
// 返回值：如果输入合法则返回 prio，否则返回 -1
// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    let current_task = current_task().unwrap();
    if _prio < 2 {
        return -1;
    } else {
        current_task.inner_exclusive_access().pass = BIG_STRIDE / (_prio as usize);
        return _prio;
    }

}
