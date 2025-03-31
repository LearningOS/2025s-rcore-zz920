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


pub struct TraceInfo {
    stack_cnt: usize,
    stack_ptr: [usize; 16],
    func_cnt: [isize; 512 * 16],
    _idx: usize,
}


static mut SYSCALL_RECORD: TraceInfo = TraceInfo {
    stack_cnt: 0,
    stack_ptr: [0; 16],
    func_cnt: [0; 512 * 16],
    _idx: 0,
};
    

impl TraceInfo {
    fn write_stack_func_count(&mut self, stack_ptr: usize, func_id: usize) {
        if let Some(idx) = (0..self.stack_cnt).find(|idx| self.stack_ptr[*idx] == stack_ptr) {
            self.func_cnt[idx * 512 + func_id] += 1; 
            self._idx = idx;
        } else {
            let idx = self.stack_cnt;
            self.stack_cnt += 1;
            self.stack_ptr[idx] = stack_ptr;
            self.func_cnt[idx * 512 + func_id] += 1;
            self._idx = idx;
        }
    }

    fn read_stack_func_count(&self, func_id: usize) -> isize {
        // println!("Read: stack_ptr: {}, func_id: {}, cnt: {}", stack_ptr, func_id, self.stack_cnt);
        return self.func_cnt[self._idx * 512 + func_id];
    }
}

pub fn write_stack_func_count(stack_ptr: usize, func_id: usize) {
    unsafe {
        SYSCALL_RECORD.write_stack_func_count(stack_ptr, func_id)
    }
}

pub fn read_stack_func_count(func_id: usize) -> isize {
    unsafe {
        SYSCALL_RECORD.read_stack_func_count(func_id)
    }
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

// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    unsafe {
        match _trace_request {
            0 => {
                // 如果 trace_request 为 0，则 id 应被视作 *const u8 ，
                // 表示读取当前任务 id 地址处一个字节的无符号整数值。
                // 此时应忽略 data 参数。返回值为 id 地址处的值
                return (_id as *const u8).read_volatile() as isize;
            },
            1 => {
                // 如果 trace_request 为 1，则 id 应被视作 *const u8 ，
                // 表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。
                // 返回值应为0。
                (_id as *mut u8).write_volatile(_data as u8);
                return 0;
            },
            2 => {
                // 如果 trace_request 为 2，
                // 表示查询当前任务调用编号为 id 的系统调用的次数，
                // 返回值为这个调用次数。本次调用也计入统计 。
                read_stack_func_count(_id)
            },
            _ => {
                return -1;
            }
        }
    }
    
}