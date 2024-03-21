use alloc::boxed::Box;
use smoltcp::time::Instant;
use async_runtime::{coroutine_run_until_complete, coroutine_spawn_with_prio};
use async_runtime::utils::yield_now;
use sel4::cap_type::Notification;
use sel4::LocalCPtr;

use sel4_logging::log::debug;
use sel4_root_task::debug_println;
use uintr::register_receiver;
use crate::async_lib::{register_recv_cid, uintr_handler};
use crate::device::{init_net_interrupt_handler, INTERFACE, NET_DEVICE, SOCKET_SET};
pub fn init() -> LocalCPtr<Notification> {
    let (_net_handler, net_ntfn) = init_net_interrupt_handler();
    let tcb = sel4::BootInfo::init_thread_tcb();
    tcb.tcb_bind_notification(net_ntfn).unwrap();
    register_receiver(tcb, net_ntfn, uintr_handler as usize).unwrap();

    let cid = coroutine_spawn_with_prio(Box::pin(net_poll()), 0);
    let badge = register_recv_cid(&cid).unwrap() as u64;
    assert_eq!(badge, 0);
    return net_ntfn;
}

async fn net_poll() {
    debug!("net_interrupt_handler");
    loop {
        INTERFACE.lock().poll(
            Instant::ZERO,
            unsafe { &mut *NET_DEVICE.as_mut_ptr() },
            &mut SOCKET_SET.lock(),
        );

        for (_handler, socket) in SOCKET_SET.lock().iter() {
            debug_println!("get socket, socket: {:?}", socket);
        }
        let _ = yield_now().await;
    }
}
