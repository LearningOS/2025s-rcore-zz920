//! Process management syscalls
use core::mem;

use crate::{mm::{frame_alloc, frame_dealloc, translated_byte_buffer, PTEFlags, VirtAddr, VirtPageNum}, task::{change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next}, timer::get_time_us};
use crate::mm::PageTable;

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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let size = mem::size_of::<TimeVal>();
   
    // 0x822bcee0, 16
    // 0x822cfef0, 16
    unsafe {
        let time_val = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
        
        let raw_bytes = core::slice::from_raw_parts((&time_val as *const TimeVal) as *const u8, size);

        let buffers = translated_byte_buffer(current_user_token(), _ts as *const u8, size);
        
        let mut offset = 0;
        for buffer in buffers {
            buffer.copy_from_slice(&raw_bytes[offset..(offset + buffer.len())]);
            offset += buffer.len();
        }
    };
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    unsafe {
        match _trace_request {
            0 => {
                // 如果 trace_request 为 0，则 id 应被视作 *const u8 ，
                // 表示读取当前任务 id 地址处一个字节的无符号整数值。
                // 此时应忽略 data 参数。返回值为 id 地址处的值
                let buffers = translated_byte_buffer(current_user_token(), _id as *const u8, 1);
                if buffers.is_empty() {
                    return -1;
                }
                return buffers[0].as_ptr().read_volatile() as isize;
            },
            1 => {
                // 如果 trace_request 为 1，则 id 应被视作 *const u8 ，
                // 表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。
                // 返回值应为0。
                let mut buffers = translated_byte_buffer(current_user_token(), _id as *const u8, 1);
                if buffers.is_empty() {
                    return -1;
                }
                buffers[0].as_mut_ptr().write_volatile(_data as u8);
                return 0;
            },
            2 => {
                // 如果 trace_request 为 2，
                // 表示查询当前任务调用编号为 id 的系统调用的次数，
                // 返回值为这个调用次数。本次调用也计入统计。
                return read_stack_func_count(_id);
            },
            _ => {
                return -1;
            }
        }
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    
    if _port & !0x7 != 0 || _port & 0x7 == 0 {
        // println!("Invalid PTE {:?}.", _port);
        return -1;
    }

    let mut page_table = PageTable::from_token(current_user_token());
    
    let start_address = VirtAddr::from(_start);
    let end_address = VirtAddr::from(_start + _len - 1);

    if !start_address.aligned() {
        // println!("_start address {:?} not aligned.", _start);
        return -1;
    }

    for vpn in VirtPageNum::from(start_address).0..=end_address.floor().0 {
        if page_table.translate(vpn.into()).is_some() {
            // println!("VPN {:?} has already been occupied.", vpn);
            return -1;
        }

        if let Some(frame) = frame_alloc() {
            page_table.map(vpn.into(), frame.ppn, PTEFlags::from_bits(_port as u8).unwrap() | PTEFlags::U);
        } else {
            // println!("Out of Memory!");
            return -1;
        }
    }

   
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");

    let mut page_table = PageTable::from_token(current_user_token());
    
    let start_address = VirtAddr::from(_start);
    let end_address = VirtAddr::from(_start + _len - 1);

    for vpn in start_address.floor().0..=end_address.floor().0 {
        if let Some(pte) = page_table.translate(vpn.into()) {
            page_table.unmap(vpn.into());
            frame_dealloc(pte.ppn());
        } else {
            // println!("VPN {:?} not alloc.", vpn);
            return -1;
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
