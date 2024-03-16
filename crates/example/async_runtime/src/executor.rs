use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicU64};
use core::sync::atomic::Ordering::Relaxed;
use core::task::Poll;
use crate::coroutine::{Coroutine, CoroutineId};
use sel4::get_clock;
use crate::utils::{BitMap, BitMap4096, BitMap64, RingBuffer};


const ARRAY_REPEAT_VALUE: Option<Arc<Coroutine>> = None;

const MAX_TASK_NUM_PER_PRIO: usize = 1024;

#[repr(C)]
pub struct Executor {
    ready_queue: [RingBuffer<CoroutineId, MAX_TASK_NUM_PER_PRIO>; 64],
    prio_bitmap: BitMap64,
    coroutine_num: usize,
    pub current: Option<CoroutineId>,
    tasks: [Option<Arc<Coroutine>>; 1024],
    pub immediate_value: [Option<u64>; 1024],
    delay_wake_cids: AtomicU64,
    tasks_bak: Vec<Arc<Coroutine>>,
}


impl Executor {

    pub fn new() -> Self {
        Self {
            coroutine_num: 0,
            current: None,
            tasks: [ARRAY_REPEAT_VALUE; 1024],
            immediate_value: [None; 1024],
            ready_queue: [RingBuffer::new(); 64],
            prio_bitmap: BitMap64::new(),
            tasks_bak: Vec::new(),
            delay_wake_cids: AtomicU64::new(0),
        }
    }

    pub fn spawn(&mut self, future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize) -> CoroutineId {
        let task = Coroutine::new(future, prio);
        let cid = task.cid;
        self.prio_bitmap.set(prio);
        self.ready_queue[prio].push(&cid).unwrap();
        self.tasks[cid.0 as usize] = Some(task.clone());
        self.coroutine_num += 1;
        self.tasks_bak.push(task.clone());
        return cid;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.coroutine_num == 0
    }


    pub fn fetch(&mut self) -> Option<Arc<Coroutine>> {
        // sel4::debug_println!("fetch, start: {:#x}, start: {}, end: {}", (&self.ready_queue[0]) as *const RingBuffer<CoroutineId, MAX_TASK_NUM_PER_PRIO> as usize,
        // self.ready_queue[0].start, self.ready_queue[0].end);
        let mut delay_wake_cids = self.delay_wake_cids.swap(0, Relaxed);
        let mut index = 0;
        while delay_wake_cids != 0 {
            if delay_wake_cids & 1 != 0 {
                self.wake(&CoroutineId::from_val(index));
                delay_wake_cids &= !(1 << index);
            }
            index += 1;
            delay_wake_cids >> 1;
        }

        let prio = self.prio_bitmap.find_first_one();
        if prio == 64 {
            return None;
        }
        if let Some(cid) = self.ready_queue[prio].pop() {
            // sel4::debug_println!("fetch cid: {:?}", cid);
            let task = self.tasks[cid.0 as usize].clone().unwrap();
            self.current = Some(cid);
            if self.ready_queue[prio].empty() {
                self.prio_bitmap.clear(prio);
            }
            Some(task)
        } else {
            None
        }
    }

    pub fn wake(&mut self, cid: &CoroutineId) {
        // todo:  need to fix bugs
        // assert!(self.tasks.contains_key(cid));
        let prio = self.tasks[cid.0 as usize].clone().unwrap().prio;
        self.prio_bitmap.set(prio);
        // sel4::debug_println!("wake cid: {:?}, start: {:#x}, prio: {}", cid,(&self.ready_queue[prio]) as *const RingBuffer<CoroutineId, MAX_TASK_NUM_PER_PRIO> as usize, prio);
        self.ready_queue[prio].push(&cid).unwrap();
        // sel4::debug_println!("wake cid: {:?}", cid);
    }

    #[inline]
    pub fn delay_wake(&mut self, cid: &CoroutineId) {
        assert!(cid.0 < 64);
        let current= self.delay_wake_cids.load(Relaxed);
        self.delay_wake_cids.store(current | 1 << cid.0, Relaxed);
    }


    #[inline]
    pub fn remove_task(&mut self, cid: CoroutineId) {
        self.tasks[cid.0 as usize] = None;
        self.coroutine_num -= 1;
        cid.release();
    }

    pub fn run_until_complete(&mut self) {
        while !self.is_empty() {
            self.run_until_blocked();
        }
    }

    pub fn run_until_blocked(&mut self) {
        while let Some(task) = self.fetch() {
            let cid = task.cid;
            // sel4::debug_println!("run_until_blocked loop");
            match task.execute() {
                Poll::Ready(_) => {
                    self.remove_task(cid);
                }
                Poll::Pending => {
                    // self.pending(cid);
                }
            }
        }
    }
}