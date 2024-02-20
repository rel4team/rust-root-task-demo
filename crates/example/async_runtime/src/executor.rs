use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::Poll;
use crate::coroutine::{Coroutine, CoroutineId};

pub struct Executor {
    pub current: Option<CoroutineId>,
    pub tasks: BTreeMap<CoroutineId, Arc<Coroutine>>,
    pub ready_queue: VecDeque<CoroutineId>,
    pub pending_set: BTreeSet<CoroutineId>,
}

impl Executor {
    pub const fn new() -> Self {
        Self {
            current: None,
            tasks: BTreeMap::new(),
            ready_queue: VecDeque::new(),
            pending_set: BTreeSet::new(),
        }
    }

    pub fn spawn(&mut self, future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>) -> usize {
        let task = Coroutine::new(future);
        let cid = task.cid;
        self.ready_queue.push_back(cid);
        self.tasks.insert(cid, task);
        return cid.0;
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn fetch(&mut self) -> Option<Arc<Coroutine>> {
        let cid = self.ready_queue.pop_front().unwrap();
        let task = self.tasks.get(&cid).unwrap().clone();
        self.current = Some(cid);
        Some(task)
    }

    #[inline]
    pub fn pending(&mut self, cid: CoroutineId) {
        self.pending_set.insert(cid);
    }

    #[inline]
    pub fn is_pending(&self, cid: CoroutineId) -> bool {
        self.pending_set.contains(&cid)
    }

    pub fn wake(&mut self, cid: CoroutineId) {
        // todo:  need to fix bugs
        assert!(self.tasks.contains_key(&cid));
        self.ready_queue.push_back(cid);
    }

    #[inline]
    pub fn remove_task(&mut self, cid: CoroutineId) {
        self.tasks.remove(&cid);
    }

    pub fn run_until_complete(&mut self) {
        while !self.is_empty() {
            self.run_until_blocked();
        }
    }

    pub fn run_until_blocked(&mut self) {
        while let Some(task) = self.fetch() {
            let cid = task.cid;
            match task.execute() {
                Poll::Ready(_) => {
                    self.remove_task(cid);
                }
                Poll::Pending => {
                    self.pending(cid);
                }
            }
        }
    }
}