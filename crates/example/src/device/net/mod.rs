use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use sel4_root_task::debug_println;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use spin::{Lazy, Mutex};
use virtio_drivers::device::net::{RxBuffer, TxBuffer};
use sel4::{BootInfo, LocalCPtr};
use sel4::cap_type::{IRQHandler, Notification};
use crate::device::config::NET_CONFIG;
use crate::device::net::virtio_net::{get_net_device, PLIC_NET_IRQ, VIRT_IO_NET_DEVICE};
use crate::net::snoop_tcp_packet;
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
                debug_println!("net read not ready");
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


pub static INTERFACE: Lazy<Arc<Mutex<Interface>>> = Lazy::new(|| Arc::new(Mutex::new(
    Interface::new(
        Config::new(EthernetAddress(get_net_device().lock().mac_address()).into()),
        unsafe { &mut *NET_DEVICE.as_mut_ptr() },
        Instant::ZERO
    )
)));

pub struct RxTokenWrapper(NetDevice, RxBuffer);

impl RxToken for RxTokenWrapper {
    fn preprocess(&self, sockets: &mut SocketSet<'_>) {
        // debug_println!("preprocess");
        snoop_tcp_packet(self.1.packet(), sockets).ok();
    }
    fn consume<R, F>(self, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        let mut buf = self.1;
        let res = f(&mut (buf.packet_mut()));
        get_net_device().lock().recycle_rx_buffer(buf).unwrap();
        res
    }
}

impl TxToken for NetDevice {
    fn consume<R, F>(self, len: usize, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        // debug_println!("txtoken consume");
        let mut tx_frame = Box::pin(vec![0u8; len]);
        let res = f((*tx_frame).as_mut());
        get_net_device().lock().send(TxBuffer::from(tx_frame.as_mut_slice())).expect("can't send data");
        res
    }
}

impl Device for NetDevice {
    type RxToken<'a> = RxTokenWrapper;
    type TxToken<'a> = Self;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        return if get_net_device().lock().can_recv() {
            let buf = get_net_device().lock().receive().unwrap();
            // debug!("NetDevice buf: {:?}", buf.as_bytes());
            Some((RxTokenWrapper(self.clone(), buf), self.clone()))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        return if get_net_device().lock().can_send() {
            Some(self.clone())
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = NET_CONFIG.mtu;
        caps.max_burst_size = Some(1);
        caps
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