mod config;

use core::ptr::NonNull;

use alloc::borrow::ToOwned;
use alloc::slice;
use alloc::{boxed::Box, sync::Arc, vec};
use async_runtime::utils::IndexAllocator;
use axi_ethernet::{AxiEthernet, XAE_JUMBO_OPTION, XAE_BROADCAST_OPTION, LinkStatus};
use lazy_static::lazy_static;
use sel4::BootInfo;
use sel4_root_task::debug_println;
use sel4::cap_type::{Untyped, MegaPage};
use sel4::{FrameSize, ObjectBlueprint, ObjectBlueprintArch, VMAttributes, CapRights};
use sel4::get_clock;
use smoltcp::iface::SocketSet;
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::wire::HardwareAddress;
use spin::{Lazy, Mutex};
use axi_dma::{AxiDma, BufPtr};
use crate::image_utils::UserImageUtils;
use crate::net::snoop_tcp_packet;
use crate::{device::net::axi_net::config::{DMA_ADDRESS, ETH_ADDRESS}, object_allocator::GLOBAL_OBJ_ALLOCATOR};

use self::config::{AXI_DMA_CONFIG, AXI_NET_CONFIG, MAC_ADDRESS, MTU};

pub fn init(boot_info: &BootInfo) {
    init_mmio(boot_info);
    dma_init();
    eth_init();
    debug_println!("net device init end");
}

#[derive(Clone)]
pub struct AxiNet {
    pub dma: Arc<AxiDma>,
    pub eth: Arc<Mutex<AxiEthernet>>,
}

impl AxiNet {
    pub const fn new(
        dma: Arc<AxiDma>,
        eth: Arc<Mutex<AxiEthernet>>
    ) -> Self {
        Self { dma, eth }
    }

    pub fn mac(&self) -> HardwareAddress {
        let mut addr = [0u8; 6];
        self.eth.lock().get_mac_address(&mut addr);
        HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(addr))
    }
}

impl Default for AxiNet {
    fn default() -> Self {
        debug_println!("Net device init================");
        AxiNet::new(AXI_DMA.clone(), AXI_ETH.clone())
    }
}

pub static AXI_DMA: Lazy<Arc<AxiDma>> = Lazy::new(|| Arc::new(AxiDma::new(AXI_DMA_CONFIG)));

pub static AXI_ETH: Lazy<Arc<Mutex<AxiEthernet>>> = Lazy::new(||  Arc::new(Mutex::new(AxiEthernet::new(
    AXI_NET_CONFIG.eth_baseaddr, AXI_NET_CONFIG.dma_baseaddr
))));


pub fn dma_init() {
    AXI_DMA.reset().unwrap();
    // enable cyclic mode
    AXI_DMA.cyclic_enable();

    // init cyclic block descriptor
    let _ = AXI_DMA.tx_channel_create_with_translate(AXI_NET_CONFIG.tx_bd_cnt, UserImageUtils::get_heap_paddr).unwrap();
    let _ = AXI_DMA.rx_channel_create_with_translate(AXI_NET_CONFIG.rx_bd_cnt, UserImageUtils::get_heap_paddr).unwrap();
    AXI_DMA.intr_enable();
}

pub fn eth_init() {
    let mut eth = AXI_ETH.lock();
    eth.reset();
    let options = eth.get_options();
    eth.set_options(options | XAE_JUMBO_OPTION);
    eth.clear_options(XAE_BROADCAST_OPTION);
    eth.detect_phy();
    let speed = eth.get_phy_speed_ksz9031();
    debug_println!("speed is: {}", speed);
    eth.set_operating_speed(speed as u16);
    if speed == 0 {
        eth.link_status = LinkStatus::EthLinkDown;
    } else {
        eth.link_status = LinkStatus::EthLinkUp;
    }
    eth.set_mac_address(&AXI_NET_CONFIG.mac_addr);
    debug_println!("link_status: {:?}", eth.link_status);
    eth.enable_rx_memovr();
    eth.clear_rx_memovr();
    eth.enable_rx_rject();
    eth.clear_rx_rject();
    eth.enable_rx_cmplt();
    // eth.enable_tx_cmplt();
    eth.clear_rx_cmplt();
    eth.clear_tx_cmplt();

    eth.start();
}

pub fn interrupt_handler() {
    if NET_DEVICE.eth.lock().is_rx_cmplt() {
        NET_DEVICE.eth.lock().clear_rx_cmplt();
    }
    if NET_DEVICE.eth.lock().is_tx_cmplt() {
        NET_DEVICE.eth.lock().clear_tx_cmplt();
    }
}


pub static NET_DEVICE: Lazy<AxiNet> = Lazy::new(|| AxiNet::default());
static mut DMA_BUFFER_POOL: [[u8; MTU]; 64] = [[0u8; MTU]; 64];
static mut DMA_BUFFER_ALLOCATOR: IndexAllocator<64> = IndexAllocator::new();


struct DMABufferHandler {
    handler: usize
}

impl DMABufferHandler {
    #[inline]
    pub fn new() -> Self {
        unsafe {
            let handler = DMA_BUFFER_ALLOCATOR.allocate().unwrap();
            Self {
                handler,
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        MTU
    }

    #[inline]
    pub fn get_mut_buffer(&mut self) -> *mut u8 {
        unsafe {
            DMA_BUFFER_POOL[self.handler].as_mut_ptr()
        }
    }

    #[inline]
    pub fn get_buffer(&self) -> *const u8 {
        unsafe {
            DMA_BUFFER_POOL[self.handler].as_ptr()
        }
    }
}

impl Drop for DMABufferHandler {
    fn drop(&mut self) {
        unsafe {
            DMA_BUFFER_ALLOCATOR.release(self.handler);
        }
    }
}

pub struct RxTokenWrapper(AxiNet, DMABufferHandler);

impl RxToken for RxTokenWrapper {
    fn preprocess(&self, sockets: &mut SocketSet<'_>) {
        // debug_println!("preprocess");
        let buffer: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(self.1.get_buffer() as usize as *mut u8, self.1.len())
        };
        snoop_tcp_packet(buffer, sockets).ok();
    }

    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let buffer: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(self.1.get_buffer() as usize as *mut u8, self.1.len())
        };
        
        f(buffer)
    }
}

impl TxToken for AxiNet {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        // let mut buffer = vec![0u8; len].into_boxed_slice();
        let mut buffer_handler = DMABufferHandler::new();
        let raw_buffer_ptr = buffer_handler.get_mut_buffer();
        let raw_buffer: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(raw_buffer_ptr, len)
        };
        let res = f(raw_buffer);
        // let res = f(&mut buffer);
        let len = raw_buffer.len();
        // let tmp = Box::into_raw(buffer) as *mut usize as usize;
        let buf_ptr: *mut u8 = UserImageUtils::get_heap_paddr(raw_buffer_ptr as usize) as *mut _;
        let buf = BufPtr::new(NonNull::new(buf_ptr).unwrap(), len);
        let mut tbuf = self.dma.tx_submit_with_translate(buf, UserImageUtils::get_heap_paddr).unwrap().wait().unwrap();
        res
    }
}


impl Device for AxiNet {
    type RxToken<'a> = RxTokenWrapper;
    type TxToken<'a> = Self;

    fn receive(
        &mut self,
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut local_eth = self.eth.lock();
        if local_eth.can_receive() {
            // let mtu = self.capabilities().max_transmission_unit;
            // let buffer = vec![0u8; mtu].into_boxed_slice();
            let mut buffer_handler = DMABufferHandler::new();
            let len = buffer_handler.len();
            let buf_ptr: *mut u8 = UserImageUtils::get_heap_paddr(buffer_handler.get_mut_buffer() as usize) as *mut _;
            // debug_println!("tmp: {:#x}, {:#x}", tmp, UserImageUtils::get_heap_paddr(tmp));
            let buf = BufPtr::new(NonNull::new(buf_ptr).unwrap(), len);
            let _ = self.dma
                                                .rx_submit_with_translate(buf, UserImageUtils::get_heap_paddr)
                                                .unwrap()
                                                .wait()
                                                .unwrap();
            Some((RxTokenWrapper(self.clone(), buffer_handler), self.clone()))
            // Some((self.clone(), self.clone()))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        // debug_println!("AxiNet transmit");
        if self.dma.tx_channel.as_ref().unwrap().has_free_bd() {
            Some(self.clone())
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = MTU;
        caps.max_burst_size = None;
        caps
    }
}


fn init_mmio(boot_info: &BootInfo) {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let (mut net_untyped, mut net_untyped_bits) = (BootInfo::init_cspace_local_cptr::<Untyped>(0), 0);
    for (i, desc) in boot_info.untyped_list().iter().enumerate() {
        if desc.is_device() && desc.paddr() <= DMA_ADDRESS && desc.paddr() + (1 << desc.size_bits()) > ETH_ADDRESS {
            debug_println!(
                "net untyped: paddr: {:x}, size_bits: {}, is_device: {}",
                desc.paddr(),
                desc.size_bits(),
                desc.is_device()
            );
            net_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(boot_info.untyped().start + i);
            net_untyped_bits = desc.size_bits();
            break;
        }
    }
    let net_untyped_slot = obj_allocator.lock().get_empty_slot();
    let retype_bits = net_untyped_bits - FrameSize::MEGA_BITS - 7;
    let retype_num = (1 << retype_bits);
    let bluprint = ObjectBlueprint::Untyped {
        size_bits: FrameSize::MEGA_BITS + 7
    };

    let cnode = BootInfo::init_thread_cnode();

    net_untyped.untyped_retype(
        &bluprint,
        &cnode.relative_self(),
        net_untyped_slot,
        retype_num
    ).unwrap();

    for _ in 0..retype_num - 1 {
        let _ = obj_allocator.lock().get_empty_slot();
    }
    debug_println!("retype num: {}", retype_num);


    let net_frame_slot = obj_allocator.lock().get_empty_slot();

    for i in 0..retype_num {
        let bluprint = ObjectBlueprint::Arch(ObjectBlueprintArch::MegaPage);
        let net_frame_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(net_untyped_slot + i);
        net_frame_untyped.untyped_retype(
            &bluprint,
            &cnode.relative_self(),
            net_frame_slot + i,
            1
        ).unwrap();
        let _ = obj_allocator.lock().get_empty_slot();
        let net_frame = BootInfo::init_cspace_local_cptr::<MegaPage>(net_frame_slot + i);
        let paddr = net_frame.frame_get_address().unwrap();
        debug_println!("paddr: {:#x}", paddr);
        if paddr <= DMA_ADDRESS && paddr + (1 << FrameSize::MEGA_BITS) > ETH_ADDRESS {
            debug_println!("net_frame paddr: {:#x}", paddr);
            let vaddr = paddr;
            let l2_page_table = obj_allocator.lock().alloc_page_table().unwrap();
            l2_page_table.page_table_map(BootInfo::init_thread_vspace(), vaddr, VMAttributes::DEFAULT).unwrap();
            net_frame.frame_map(
                BootInfo::init_thread_vspace(),
                vaddr,
                CapRights::read_write(),
                VMAttributes::DEFAULT,
            ).unwrap();
            break;
        }

    }
}

pub fn recv_test() {
    debug_println!("start recv test");
    const MTU: usize = 1500;
    loop {
        if !AXI_ETH.lock().can_receive() {
            continue;
        }
        let buffer = vec![2u8; MTU].into_boxed_slice();
        let len = buffer.len();
        let tmp = Box::into_raw(buffer) as *mut usize as usize;
        let buf_ptr: *mut u8 = UserImageUtils::get_heap_paddr(tmp) as *mut _;
        debug_println!("vaddr: {:#x}, paddr: {:#x}", tmp, buf_ptr as usize);
        let buf = BufPtr::new(NonNull::new(buf_ptr).unwrap(), len);
        let mut rbuf = AXI_DMA
            .rx_submit_with_translate(buf.clone(), UserImageUtils::get_heap_paddr)
            .unwrap()
            .wait()
            .unwrap();
        let buf_ptr = UserImageUtils::get_heap_vaddr(rbuf.as_mut_ptr() as usize) as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(buf_ptr, buf.len()) };
        let box_buf = unsafe { Box::from_raw(slice) };
        // debug_println!("single receive ok: {:?}", box_buf);
        drop(box_buf);
    }
}

pub fn transmit_test() {
    debug_println!("start transmit test");
    const MTU: usize = 1500;
    let _tx_channel = AXI_DMA.tx_channel.as_ref().unwrap();
    let mut buffer = vec![1u8; MTU].into_boxed_slice();
    let len = buffer.len();
    buffer[..6].copy_from_slice(&[0x00, 0x0A, 0x35, 0x01, 0x05, 0x06]);
    buffer[6..12].copy_from_slice(&[0x00, 0x0A, 0x35, 0x01, 0x02, 0x03]);
    buffer[12..14].copy_from_slice(&((MTU - 14) as u16).to_be_bytes());
    let tmp = Box::into_raw(buffer) as *mut usize as usize;
    let buf_ptr: *mut u8 = UserImageUtils::get_heap_paddr(tmp) as *mut _;
    debug_println!("tmp: {:#x}, {:#x}", tmp, UserImageUtils::get_heap_paddr(tmp));
    // let buf_ptr = Box::into_raw(buffer) as *mut _;
    let buf = BufPtr::new(NonNull::new(buf_ptr).unwrap(), len);
    for i in 0..36 {
        let _buff = AXI_DMA
            .tx_submit_with_translate(buf.clone(), UserImageUtils::get_heap_paddr)
            .unwrap()
            .wait()
            .unwrap();
    }
    debug_println!("tx finished");
    while !AXI_ETH.lock().is_tx_cmplt() {}
    debug_println!("transmit test pass!");
}