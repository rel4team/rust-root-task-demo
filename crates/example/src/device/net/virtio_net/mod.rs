mod config;

use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::vec;
use alloc::sync::Arc;
use smoltcp::wire::HardwareAddress;
use core::alloc::Layout;
use core::ptr::NonNull;
use spin::{Lazy, Mutex};
use virtio_drivers::{BufferDirection, Hal};
use virtio_drivers::device::net::{RxBuffer, TxBuffer, VirtIONet};
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use sel4::BootInfo;
use sel4::cap_type::{Untyped, MegaPage};
use sel4::{FrameSize, ObjectBlueprint, ObjectBlueprintArch, VMAttributes, CapRights};
use sel4_logging::log::debug;
use sel4_root_task::debug_println;
use crate::image_utils::UserImageUtils;
use crate::net::snoop_tcp_packet;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
use smoltcp::iface::SocketSet;
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;

use self::config::NET_CONFIG;


pub static NET_DEVICE_ADDR: usize = 0x10008000;
pub(crate) const NET_QUEUE_SIZE: usize = 16;
pub(crate) const NET_BUFFER_LEN: usize = 2048;
pub struct VirtioHal;

pub static mut VIRT_IO_NET_DEVICE: usize = 0;

pub fn get_net_device() -> &'static mut Mutex<VirtIONet<VirtioHal, MmioTransport, NET_QUEUE_SIZE>> {
    unsafe {
        &mut *(VIRT_IO_NET_DEVICE as *mut Mutex<VirtIONet<VirtioHal, MmioTransport, NET_QUEUE_SIZE>>)
    }
}

fn get_mac_address() -> [u8; 6] {
    get_net_device().lock().mac_address()
}


#[derive(Clone)]
pub struct NetDevice {
    pub net_device_addr: usize,
}

impl NetDevice {
    pub fn mac(&self) -> HardwareAddress {        
        HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(get_mac_address()))
    }
}

pub static NET_DEVICE: Lazy<NetDevice> = Lazy::new(|| NetDevice { net_device_addr: unsafe { VIRT_IO_NET_DEVICE } });

pub fn interrupt_handler() {

}

impl NetDevice {
    pub fn transmit(&self, data: &[u8]) {
        let net: &mut spin::mutex::Mutex<VirtIONet<VirtioHal, MmioTransport, 16>> = get_net_device();
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

unsafe impl Hal for VirtioHal {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (usize, NonNull<u8>) {
        const PAGE_SIZE:usize = 4096;
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE)
            .expect("Failed to create layout for page aligned memory allocation");
        let vptr = unsafe {
            let ptr = alloc_zeroed(layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr as usize
        };
        let paddr = UserImageUtils.get_user_image_frame_paddr(vptr);
        // debug_println!("[dma_alloc] pages: {}, paddr: {:#x}, vaddr: {:#x}",pages, paddr, vptr);

        // debug!("[dma_alloc]paddr: {:#x}, vaddr: {:#x}", paddr, paddr + PPTR_BASE_OFFSET);
        (paddr, NonNull::new(vptr as _).unwrap())
    }

    unsafe fn dma_dealloc(_paddr: usize, _vaddr: NonNull<u8>, _pages: usize) -> i32 {
        // debug_println!("dma_dealloc");
        // trace!("dealloc DMA: paddr={:#x}, pages={}", paddr, pages);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: usize, _size: usize) -> NonNull<u8> {
        // debug_println!("mmio_phys_to_virt");
        NonNull::new(paddr as _).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> usize {
        // debug_println!("share");
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        // let len = buffer.len();
        // debug_println!("[share] vaddr: {:#x}, data: {}", vaddr, buffer.len());
        // Nothing to do, as the host already has access to all memory.
        UserImageUtils.get_user_image_frame_paddr(vaddr)
    }

    unsafe fn unshare(_paddr: usize, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // debug_println!("unshare");
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}


pub fn init(boot_info: &BootInfo) {
    init_mmio(boot_info);
    unsafe {
        let header = NonNull::new(NET_DEVICE_ADDR as *mut VirtIOHeader).unwrap();
        let transport = MmioTransport::new(header).unwrap();
        debug!("NET_DEVICE_ADDR: {:#x}", NET_DEVICE_ADDR);
        let virtio = VirtIONet::<VirtioHal, MmioTransport, NET_QUEUE_SIZE>
        ::new(transport, NET_BUFFER_LEN)
            .expect("can't create net device by virtio");
        let net = Arc::new(Mutex::new(virtio));
        VIRT_IO_NET_DEVICE = net.as_ref() as *const Mutex<VirtIONet<VirtioHal, MmioTransport, NET_QUEUE_SIZE>> as usize;
        core::mem::forget(net);
    }
}

fn init_mmio(boot_info: &BootInfo) {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let (mut virtio_untyped, mut virtio_untyped_bits) = (BootInfo::init_cspace_local_cptr::<Untyped>(0), 0);
    for (i, desc) in boot_info.untyped_list().iter().enumerate() {
        if desc.is_device() && desc.paddr() <= NET_DEVICE_ADDR && desc.paddr() + (1 << desc.size_bits()) > NET_DEVICE_ADDR {
            debug_println!(
                "VirtIO-net untyped: paddr: {:x}, size_bits: {}, is_device: {}",
                desc.paddr(),
                desc.size_bits(),
                desc.is_device()
            );
            virtio_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(boot_info.untyped().start + i);
            virtio_untyped_bits = desc.size_bits();
            break;
        }
    }
    let virtio_untyped_slot = obj_allocator.lock().get_empty_slot();
    let retype_bits = virtio_untyped_bits - FrameSize::MEGA_BITS;
    let retype_num = (1 << retype_bits) / 4;
    let bluprint = ObjectBlueprint::Untyped {
        size_bits: FrameSize::MEGA_BITS
    };

    let cnode = BootInfo::init_thread_cnode();

    virtio_untyped.untyped_retype(
        &bluprint,
        &cnode.relative_self(),
        virtio_untyped_slot,
        retype_num
    ).unwrap();

    for _ in 0..retype_num - 1 {
        let _ = obj_allocator.lock().get_empty_slot();
    }
    debug!("retype num: {}", retype_num);


    let virtio_frame_slot = obj_allocator.lock().get_empty_slot();

    for i in 0..retype_num {
        let bluprint = ObjectBlueprint::Arch(ObjectBlueprintArch::MegaPage);
        let virtio_frame_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(virtio_untyped_slot + i);
        virtio_frame_untyped.untyped_retype(
            &bluprint,
            &cnode.relative_self(),
            virtio_frame_slot + i,
            1
        ).unwrap();
        let _ = obj_allocator.lock().get_empty_slot();
        let virtio_frame = BootInfo::init_cspace_local_cptr::<MegaPage>(virtio_frame_slot + i);
        let paddr = virtio_frame.frame_get_address().unwrap();
        if paddr <=NET_DEVICE_ADDR && paddr + (1 << FrameSize::MEGA_BITS) > NET_DEVICE_ADDR {
            debug_println!("virtio_frame paddr: {:#x}", paddr);
            let vaddr = paddr;
            // let l2_page_table = obj_allocator.lock().alloc_page_table().unwrap();
            // l2_page_table.page_table_map(BootInfo::init_thread_vspace(), vaddr, VMAttributes::DEFAULT).unwrap();
            virtio_frame.frame_map(
                BootInfo::init_thread_vspace(),
                vaddr,
                CapRights::read_write(),
                VMAttributes::DEFAULT,
            ).unwrap();
            break;
        }

    }
}

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