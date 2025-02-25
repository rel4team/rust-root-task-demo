use sel4::{Error, Notification, TCB, with_ipc_buffer};
use taic_driver::LocalQueue;
use crate::device::taic::TAIC;
use alloc::collections::BTreeMap;
use spin::{Lazy, Mutex};
use alloc::sync::Arc;
use async_runtime::CoroutineId;
use sel4_root_task::debug_println;
pub use super::utrap_handler::register_usoft_handler;
pub static mut LQ_MAP: BTreeMap<usize, Arc<LocalQueue>> = BTreeMap::new();

#[thread_local]
pub static mut process_id: usize = 0;
pub fn register_receiver(tcb: TCB, ntfn: Notification, hart_id: usize) -> Result<usize, Error> {
    super::init_utrap_handler();
    ntfn.register_receiver(tcb.cptr())?;
    let mut recv_idx = 0;
    with_ipc_buffer(|buffer| {
        unsafe {
            recv_idx = buffer.inner().uintr_flag as usize;
            debug_println!("Registering receiver: {}", recv_idx);
            process_id = recv_idx;
            let lq = Arc::new(TAIC.alloc_lq(1, recv_idx).unwrap());
            lq.whart(hart_id);
            LQ_MAP.insert(recv_idx, lq);
            debug_println!("Registering receiver end");
        }
    });
    Ok(recv_idx)
}

#[inline]
pub fn register_sender(recv_idx: usize) {
    // debug_println!("Registering sender: {}, {}", recv_idx, sender_idx);
    unsafe {
        let recv_lq = LQ_MAP.get(&recv_idx).unwrap();
        recv_lq.register_receiver(1, process_id, 0x109);
        let sender_lq = LQ_MAP.get(&process_id).unwrap();
        sender_lq.register_sender(1, recv_idx);
    }
}

#[inline]
pub fn re_register(send_idx: usize) {
    unsafe {
        let recv_lq = LQ_MAP.get(&process_id).unwrap();
        recv_lq.register_receiver(1, send_idx, 0x109);
    }
}

#[inline]
pub fn send_signal(recv_process_id: usize) {
    unsafe {
        // debug_println!("send_signal: {} --> {}", process_id, recv_process_id);
        let lq = LQ_MAP.get(&process_id).unwrap();
        lq.send_intr(1, recv_process_id)
    }
}