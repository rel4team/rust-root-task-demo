use alloc::sync::Arc;
use sel4_root_task::debug_println;

use smoltcp::iface::{Config, Interface};
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use spin::{Lazy, Mutex};
use sel4::{BootInfo, LocalCPtr};
use sel4::cap_type::{IRQHandler, Notification};
use crate::device::net::virtio_net::get_net_device;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
// pub use virtio_net::{NET_DEVICE, interrupt_handler};
pub use axi_net::{NET_DEVICE, interrupt_handler};

pub use axi_net::{transmit_test, recv_test};

mod virtio_net;
mod axi_net;

static PLIC_NET_IRQ: u64 = 1;

pub fn init(boot_info: &BootInfo) {
    // virtio_net::init(boot_info);
    axi_net::init(boot_info);
}

pub static INTERFACE: Lazy<Arc<Mutex<Interface>>> = Lazy::new(|| Arc::new(Mutex::new(
    Interface::new(
        Config::new(NET_DEVICE.mac()),
        unsafe { &mut *NET_DEVICE.as_mut_ptr() },
        Instant::ZERO
    )
)));

pub fn init_net_interrupt_handler() -> (LocalCPtr<IRQHandler>, LocalCPtr<Notification>) {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let irq_ctrl = BootInfo::irq_control();
    let irq_handler = BootInfo::init_cspace_local_cptr::<IRQHandler>(obj_allocator.lock().get_empty_slot());
    irq_ctrl.irq_control_get(PLIC_NET_IRQ, &BootInfo::init_thread_cnode().relative(irq_handler)).unwrap();

    let handler_ntfn = obj_allocator.lock().alloc_ntfn().unwrap();
    irq_handler.irq_handler_set_notification(handler_ntfn).unwrap();
    (irq_handler, handler_ntfn)
}