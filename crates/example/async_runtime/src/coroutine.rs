use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::task::Wake;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use sel4::get_clock;
use crate::utils::IndexAllocator;

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct CoroutineId(pub u32);


#[thread_local]
static mut CID_ALLOCATOR: IndexAllocator<4096> = IndexAllocator::new();

impl CoroutineId {
    /// 生成新的协程 Id
    pub fn generate() -> CoroutineId {
        // 任务编号计数器，任务编号自增
        let cid = unsafe { CID_ALLOCATOR.allocate() };
        CoroutineId(cid.unwrap() as u32)
    }
    /// 根据 usize 生成协程 Id
    pub const fn from_val(v: u32) -> Self {
        Self(v)
    }
    /// 获取协程 Id 的 usize
    pub fn get_val(&self) -> u32 {
        self.0
    }

    pub fn release(&self) {
        unsafe { CID_ALLOCATOR.release(self.0 as usize) }
    }
}

struct CoroutineWaker(CoroutineId);

impl CoroutineWaker {
    /// 新建协程 waker
    pub fn new(cid: CoroutineId) -> Waker {
        Waker::from(Arc::new(Self(cid)))
    }
}

impl Wake for CoroutineWaker {
    fn wake(self: Arc<Self>) { }
    fn wake_by_ref(self: &Arc<Self>) { }
}

pub struct Coroutine{
    /// 协程编号
    pub cid: CoroutineId,
    // 优先级
    pub prio: usize,
    /// future
    pub inner: RefCell<CoroutineInner>,
}

pub struct CoroutineInner {
    pub future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>,
    /// waker
    pub waker: Arc<Waker>,
}

impl Coroutine {
    /// 生成协程
    pub fn new(future: Pin<Box<dyn Future<Output=()> + Send + Sync>>, prio: usize) -> Arc<Self> {
        let cid = CoroutineId::generate();
        Arc::new(
            Coroutine {
                cid,
                inner: RefCell::new(
                    CoroutineInner {
                        future,
                        waker: Arc::new(CoroutineWaker::new(cid)),
                    }
                )
                ,prio
            }
        )
    }
    /// 执行
    #[inline]
    pub fn execute(self: Arc<Self>) -> Poll<()> {
        let waker = self.inner.borrow().waker.clone();
        let mut context = Context::from_waker(&*waker);

        self.inner.borrow_mut().future.as_mut().poll(&mut context)
    }
}
