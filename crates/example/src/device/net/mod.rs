use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use sel4_root_task::debug_println;
use spin::{Lazy, Mutex};
use virtio_drivers::device::net::{RxBuffer, TxBuffer};
use sel4::{BootInfo, LocalCPtr};
use sel4::cap_type::{IRQHandler, Notification};
use crate::device::config::NET_CONFIG;
use crate::device::net::virtio_net::{get_net_device, PLIC_NET_IRQ, VIRT_IO_NET_DEVICE};
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;

mod virtio_net;
pub fn init(boot_info: &BootInfo) {
    virtio_net::init(boot_info);
}



#[derive(Clone)]
pub struct NetDevice {
    pub net_device_addr: usize,
}

pub static NET_DEVICE: Lazy<NetDevice> = Lazy::new(|| NetDevice { net_device_addr: unsafe { VIRT_IO_NET_DEVICE } });

impl NetDevice {
    pub fn transmit(&self, data: &[u8]) {
        let net = get_net_device();
        net.lock().send(TxBuffer::from(data)).expect("can't send data");
    }

    pub fn receive(&self) -> Option<RxBuffer> {
        let net = get_net_device();
        match net.lock().receive() {
            Ok(buf) => {
                Some(buf)
            }
            Err(virtio_drivers::Error::NotReady) => {
                // debug_println!("net read not ready");
                None
            }
            Err(err) => {
                panic!("net failed to recv: {:?}", err)
            }
        }
    }

    pub fn recycle_rx_buffer(&self, buf: RxBuffer) {
        let net = get_net_device();
        net.lock().recycle_rx_buffer(buf);
    }
}



pub fn init_net_interrupt_handler() -> (LocalCPtr<IRQHandler>, LocalCPtr<Notification>) {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let irq_ctrl = BootInfo::irq_control();
    let irq_handler = BootInfo::init_cspace_local_cptr::<IRQHandler>(obj_allocator.lock().get_empty_slot());
    irq_ctrl.irq_control_get(PLIC_NET_IRQ, &BootInfo::init_thread_cnode().relative(irq_handler)).unwrap();

    let handler_ntfn = obj_allocator.lock().alloc_ntfn().unwrap();
    irq_handler.irq_handler_set_notification(handler_ntfn).unwrap();
    (irq_handler, handler_ntfn)
}