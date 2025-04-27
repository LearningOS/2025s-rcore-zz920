//! Synchronization and interior mutability primitives

mod condvar;
mod mutex;
mod semaphore;
mod up;

use alloc::vec::Vec;
pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking, MutexSpin};
pub use semaphore::Semaphore;
pub use up::UPSafeCell;

/// Resource Deadlock detector
pub struct Resource {
    /// resource available count
    pub available_vec: Vec<usize>,
    /// tid allocated resource count
    pub allocation_vec: Vec<Vec<usize>>,
    /// tid need resource count
    pub need_vec: Vec<(usize, usize, usize)>,
    /// tid mapper
    pub request_vec: Vec<usize>,
}

fn resize_vec<T>(vec: &mut Vec<T>, new_size: usize, value: T) where T: Clone{
    if vec.len() < new_size {
        vec.resize(new_size, value);
    }
}

impl Resource {
    /// create new Resource Object
    pub fn new() -> Self {
        Self{
            allocation_vec: Vec::new(),
            available_vec: Vec::new(),
            need_vec: Vec::new(),
            request_vec: Vec::new(),
        }
    }

    /// load in new resource
    pub fn create(&mut self, resource: usize, num: usize) -> isize {
        resize_vec(&mut self.available_vec, resource + 1, 0);
        self.available_vec[resource] += num;
        0
    }

    fn load_tid(&mut self, tid: usize) -> usize {
        if let Some((idx, _)) = self.request_vec.iter().enumerate().find(|(_, _tid)| **_tid == tid) {
            return idx;
        }
        self.request_vec.push(tid);
        self.request_vec.len() - 1
    }

    /// alloc resource to tid
    pub fn alloc(&mut self, resource: usize, num: usize, tid: usize) -> isize {
        if self.available_vec.len() <= resource {
            // no resource available
            return -1;
        }

        let tid_idx = self.load_tid(tid);

        if self.available_vec[resource] >= num {
            self.available_vec[resource] -= num;


            resize_vec(&mut self.allocation_vec, tid_idx + 1, Vec::new());
            resize_vec(&mut self.allocation_vec[tid_idx], resource + 1, 0);

            self.allocation_vec[tid_idx][resource] += num;

            // request affordable
            return 1;
        } else {
            // tid need {num} resource
            let mut work = self.available_vec.clone();

            let mut finish = Vec::new();
            resize_vec(&mut finish, self.request_vec.len(), false);

            let id = self.load_tid(tid);
            // update need
            self.need_vec.push((id, resource, num));


            loop {
                if let Some((idx, _)) = finish.iter().enumerate().find(|(idx, done)| {
                    return !**done && self.need_vec.iter()
                            .filter(|(_id, _, _)| *_id == *idx)
                            .all(|(_, _r, _num)| work[*_r] >= *_num);
                }) {
                    // release allocated resource
                    resize_vec(&mut self.allocation_vec[idx], resource + 1, 0);
                    self.allocation_vec[idx]
                        .iter()
                        .enumerate()
                        .for_each(|(_i, val)| {
                            work[_i] += val;
                        });
                    finish[idx] = true;
                } else {
                    break;
                }
            }

            if finish.iter().any(|done| !done) {
                return -1;
            }

        }

        // wait
        0
    }

    /// dealloc from tid
    pub fn dealloc(&mut self, resource: usize, num: usize, tid: usize) -> isize {
        let tid_idx = self.load_tid(tid);
        resize_vec(&mut self.allocation_vec, tid_idx + 1, Vec::new());
        resize_vec(&mut self.allocation_vec[tid_idx], resource + 1, 0);

        if self.allocation_vec[tid_idx][resource] < num {
            return -1;
        }

        self.available_vec[resource] += num;
        self.allocation_vec[tid_idx][resource] -= num;

        // update need
        if let Some((idx, _)) = self.need_vec.iter().enumerate()
            .filter(|(_, (_, _r, _num))| *_r == resource && *_num <= num).next() {
                let (new_idx, r, new_num) = self.need_vec.remove(idx);

                self.available_vec[r] -= new_num;
                self.allocation_vec[new_idx][r] += new_num; 
        }
        
        0
    }
}