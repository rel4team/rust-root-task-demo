use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::Poll;
use crate::coroutine::{Coroutine, CoroutineId};
use sel4::get_clock;
use crate::utils::{BitMap, BitMap4096};


const ARRAY_REPEAT_VALUE: Option<Arc<Coroutine>> = None;

pub struct Executor {
    pub ready_queue: BitMap4096,
    coroutine_num: usize,
    pub current: Option<CoroutineId>,
    pub tasks: [Option<Arc<Coroutine>>; 1024],
    pub immediate_value: [Option<u64>; 1024],
    tasks_bak: Vec<Arc<Coroutine>>,
}


impl Executor {

    pub const fn new() -> Self {
        Self {
            coroutine_num: 0,
            current: None,
            tasks: [ARRAY_REPEAT_VALUE; 1024],
            immediate_value: [None; 1024],
            ready_queue: BitMap4096::new(),
            tasks_bak: Vec::new(),
        }
    }

    pub fn spawn(&mut self, future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>) -> CoroutineId {
        let task = Coroutine::new(future);
        let cid = task.cid;
        // self.ready_queue.push_back(cid);
        self.ready_queue.set(cid.0 as usize);
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
    pub fn fetch(&mut self) -> Option<Arc<Coroutine>> {
        if let Some(cid) = self.ready_queue.fetch() {
            // sel4::debug_println!("fetch cid: {:?}", cid);
            let task = self.tasks[cid].clone().unwrap();
            self.current = Some(CoroutineId::from_val(cid as u32));
            Some(task)
        } else {
            None
        }
    }

    #[inline]
    pub fn wake(&mut self, cid: &CoroutineId) {
        // todo:  need to fix bugs
        // sel4::debug_println!("[wake] cid: {:?}", cid);
        // assert!(self.tasks.contains_key(cid));

        self.ready_queue.set(cid.0 as usize);
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