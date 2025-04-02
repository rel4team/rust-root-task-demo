use super::irq_allocator::{self, IrqAllocator};
pub use super::utrap_handler::register_usoft_handler;
use crate::device::taic::TAIC;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use async_runtime::{local_queue_init, CoroutineId};
use sel4::{with_ipc_buffer, Error, Notification, TCB};
use sel4_root_task::debug_println;
use spin::{Lazy, Mutex};
use taic_driver::LocalQueue;

//防止多个线程同时使用TAIC
pub static LOCK: Mutex<()> = Mutex::new(());
// pid,LQ
// pub static mut LQ_MAP: BTreeMap<usize, Arc<LocalQueue>> = BTreeMap::new();

struct LQManager {
    lq: Arc<LocalQueue>,
    irq_allocator: IrqAllocator,
}

// pub static mut IRQ_ALLOCER_MAP: BTreeMap<usize, IrqAllocator> = BTreeMap::new();

#[thread_local]
static mut LQ_MANAGER: Option<LQManager> = None;

// pub static mut process_id: usize = 0; //线程的ID

pub fn alloc_receiver(tcb: TCB, ntfn: Notification, hart_id: usize) -> Result<usize, Error> {
    // super::init_utrap_handler();
    ntfn.register_receiver(tcb.cptr())?; //获取一个buffer?
    let mut recv_idx = 0; //
                          //闭包操作buffer
    with_ipc_buffer(|buffer| {
        unsafe {
            //一个buffer对应一个queue
            //拿当前线程的id
            recv_idx = buffer.inner().uintr_flag as usize;
            let lq = Arc::new(TAIC.alloc_lq(1, recv_idx).unwrap());
            LQ_MANAGER = Some(LQManager {
                lq: lq.clone(),
                irq_allocator: IrqAllocator::new(),
            });
            // process_id = recv_idx;
            // let lq =  //taic 分配一个本地队列
            // lq.whart(hart_id);
            //写入当前的LQ
            // LQ_MAP.insert(recv_idx, lq.clone());
            // let allocer = IrqAllocator::new();
            // IRQ_ALLOCER_MAP.insert(process_id, allocer);
            //Executor的初始化，给ready_queue赋值
            local_queue_init(lq);
        }
    });
    Ok(recv_idx)
}

#[inline]
pub fn register_receiver(
    sender_idx: usize,
    irq: usize,
    handler: usize,
    preempt: bool,
    reusable: bool,
) {
    let _lock = LOCK.lock();
    unsafe {
        // let lq = LQ_MAP.get(&process_id).unwrap();
        let _handler = handler << 2 | ((reusable as usize) << 1)| (preempt as usize);
        // debug_println!("_handler  {:#b}", _handler);
        LQ_MANAGER
            .as_ref()
            .unwrap()
            .lq
            .register_receiver(1, sender_idx, irq, _handler);
    }
}

#[inline]
pub fn register_sender(recv_idx: usize, irq: usize) {
    // debug_println!("Registering sender: {}, {}", recv_idx, sender_idx);
    unsafe {
        // let lq = LQ_MAP.get(&process_id).unwrap();
        LQ_MANAGER
            .as_ref()
            .unwrap()
            .lq
            .register_sender(1, recv_idx, irq);
    }
}

#[inline]
pub fn send_signal(recv_process_id: usize, irq: usize) {
    let _lock = LOCK.lock();
    unsafe {
        // debug_println!("send_signal: {} --> {}", process_id, recv_process_id);
        // let lq = LQ_MAP.get(&process_id).unwrap();
        LQ_MANAGER
            .as_ref()
            .unwrap()
            .lq
            .send_intr(1, recv_process_id, irq);
    }
}

#[inline]
pub fn alloc_vec() -> Option<usize> {
    unsafe {
        // let allocer = IRQ_ALLOCER_MAP.get_mut(&process_id).unwrap();
        LQ_MANAGER.as_mut().unwrap().irq_allocator.alloc()
    }
}
#[inline]
pub fn free_vec(vec: usize) {
    unsafe {
        // let allocer = IRQ_ALLOCER_MAP.get_mut(&process_id).unwrap();
        LQ_MANAGER.as_mut().unwrap().irq_allocator.free(vec);
    }
}
