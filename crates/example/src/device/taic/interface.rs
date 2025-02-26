use sel4::{Error, Notification, TCB, with_ipc_buffer};
use taic_driver::LocalQueue;
use crate::device::taic::TAIC;
use alloc::collections::BTreeMap;
use spin::{Lazy, Mutex};
use alloc::sync::Arc;
use async_runtime::{local_queue_init, CoroutineId};
use sel4_root_task::debug_println;
pub use super::utrap_handler::register_usoft_handler;
pub static mut LQ_MAP: BTreeMap<usize, Arc<LocalQueue>> = BTreeMap::new();

#[thread_local]
pub static mut process_id: usize = 0;
pub fn alloc_receiver(tcb: TCB, ntfn: Notification, hart_id: usize) -> Result<usize, Error> {
    // super::init_utrap_handler();
    ntfn.register_receiver(tcb.cptr())?;
    let mut recv_idx = 0;
    with_ipc_buffer(|buffer| {
        unsafe {
            recv_idx = buffer.inner().uintr_flag as usize;
            process_id = recv_idx;
            let lq = Arc::new(TAIC.alloc_lq(1, recv_idx).unwrap());
            lq.whart(hart_id);
            LQ_MAP.insert(recv_idx, lq.clone());
            local_queue_init(lq);
        }
    });
    Ok(recv_idx)
}


pub fn register_receiver(sender_idx: usize, handler: usize) {
    unsafe {
        let lq = LQ_MAP.get(&process_id).unwrap();
        lq.register_receiver(1, sender_idx, handler);
    }
}

#[inline]
pub fn register_sender(recv_idx: usize) {
    // debug_println!("Registering sender: {}, {}", recv_idx, sender_idx);
    unsafe {
        let lq = LQ_MAP.get(&process_id).unwrap();
        lq.register_sender(1, recv_idx);
    }
}

#[inline]
pub fn send_signal(recv_process_id: usize) {
    unsafe {
        // debug_println!("send_signal: {} --> {}", process_id, recv_process_id);
        let lq = LQ_MAP.get(&process_id).unwrap();
        lq.send_intr(1, recv_process_id);
    }
}