use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;
use core::task::Poll;
use crate::coroutine::{Coroutine, CoroutineId};
use crate::utils::{BitMap, BitMap64, RingBuffer};


const ARRAY_REPEAT_VALUE: Option<Arc<Coroutine>> = None;

pub const MAX_TASK_NUM: usize = 2048;
pub const MAX_PRIO_NUM: usize = 8;
#[repr(align(4096))]
pub struct Executor {
    ready_queue: [RingBuffer<CoroutineId, MAX_TASK_NUM>; MAX_PRIO_NUM],
    prio_bitmap: BitMap64,
    coroutine_num: usize,
    pub current: Option<CoroutineId>,
    tasks: [Option<Arc<Coroutine>>; MAX_TASK_NUM],
    delay_wake_cids: AtomicU64,
    tasks_bak: Vec<Arc<Coroutine>>,
}


impl Executor {

    pub fn new() -> Self {
        Self {
            coroutine_num: 0,
            current: None,
            tasks: [ARRAY_REPEAT_VALUE; MAX_TASK_NUM],
            ready_queue: [RingBuffer::new(); MAX_PRIO_NUM],
            prio_bitmap: BitMap64::new(),
            tasks_bak: Vec::new(),
            delay_wake_cids: AtomicU64::new(0),
        }
    }

    pub fn get_ready_num(&self) -> usize {
        self.ready_queue[2].size()
    }

    pub fn init(&mut self) {
        *self = Self::new();
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

    #[inline]
    pub fn switch_possible(&mut self) -> bool {
        self.actual_wake();
        let task = self.tasks[self.current.unwrap().0 as usize].clone().unwrap();
        let prio = self.prio_bitmap.find_first_one();
        prio < task.prio
    }

    fn actual_wake(&mut self) {
        let mut delay_wake_cids = self.delay_wake_cids.swap(0, Relaxed);
        let mut index = 0;
        while delay_wake_cids != 0 {
            if delay_wake_cids & 1 != 0 {
                self.wake(&CoroutineId::from_val(index));
                // sel4::debug_println!("delay wake: {}", index);
                delay_wake_cids &= !(1 << index);
            }
            index += 1;
            delay_wake_cids >>= 1;
        }
    }

    pub fn fetch(&mut self) -> Option<Arc<Coroutine>> {
        // sel4::debug_println!("fetch, start: {:#x}, start: {}, end: {}", (&self.ready_queue[0]) as *const RingBuffer<CoroutineId, MAX_TASK_NUM_PER_PRIO> as usize,
        // self.ready_queue[0].start, self.ready_queue[0].end);
        self.actual_wake();
        let prio = self.prio_bitmap.find_first_one();
        
        if prio == 64 {
            return None;
        }
        if let Some(cid) = self.ready_queue[prio].pop() {
            if let Some(task) = self.tasks[cid.0 as usize].clone() {
                self.current = Some(cid);
                if self.ready_queue[prio].empty() {
                    self.prio_bitmap.clear(prio);
                }
                return Some(task);
            }
        }
        None
    }

    pub fn wake(&mut self, cid: &CoroutineId) {
        // todo:  need to fix bugs
        // assert!(self.tasks.contains_key(cid));
        let op_task = self.tasks[cid.0 as usize].clone();
        if op_task.is_some() {
            let prio = op_task.unwrap().prio;
            self.prio_bitmap.set(prio);
            // sel4::debug_println!("wake cid: {:?}, start: {:#x}, prio: {}", cid,(&self.ready_queue[prio]) as *const RingBuffer<CoroutineId, MAX_TASK_NUM_PER_PRIO> as usize, prio);
            self.ready_queue[prio].push(&cid).unwrap();
            // sel4::debug_println!("wake cid: {}, prio: {}, max_prio: {}", cid.0, prio, self.prio_bitmap.find_first_one());
            // for i in 0..MAX_PRIO_NUM {
            //     sel4::debug_println!("[fetch] prio: {}, start: {}, end: {}", i, self.ready_queue[i].start, self.ready_queue[i].end);
            // }
        }
        
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