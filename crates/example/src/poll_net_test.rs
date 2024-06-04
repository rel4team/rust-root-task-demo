use core::ptr::NonNull;
use sel4::get_clock;
use alloc::{boxed::Box, sync::Arc, vec};
use axi_dma::{AxiDma, AxiDmaConfig, BufPtr};
use axi_ethernet::{AxiEthernet, LinkStatus, XAE_BROADCAST_OPTION, XAE_JUMBO_OPTION, XAE_MAX_JUMBO_FRAME_SIZE};
use sel4_root_task::debug_println;
use smoltcp::{iface::{Config, Interface, SocketSet}, phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken}, socket::tcp::{Socket, SocketBuffer}, time::Instant, wire::HardwareAddress};
use spin::{lazy::Lazy, Mutex};

use crate::{image_utils::UserImageUtils, matrix};

const ETH_ADDRESS: usize = 0x6014_0000;
const DMA_ADDRESS: usize = 0x6010_0000;
const MAC_ADDRESS: [u8; 6] = [0x00, 0x0A, 0x35, 0x01, 0x02, 0x03];
const MTU: usize = 256;

const AXI_DMA_CONFIG: AxiDmaConfig = AxiDmaConfig {
    base_address: DMA_ADDRESS,
    rx_channel_offset: 0x30,
    tx_channel_offset: 0,
    has_sts_cntrl_strm: false,
    is_micro_dma: false,
    has_mm2s: true,
    has_mm2s_dre: false,
    mm2s_data_width: 32,
    mm2s_burst_size: 16,
    has_s2mm: true,
    has_s2mm_dre: false,
    s2mm_data_width: 32,
    s2mm_burst_size: 16,
    has_sg: true,
    sg_length_width: 16,
    addr_width: 32,
};

struct AxiNetConfig {
    pub tx_bd_cnt: usize,
    pub rx_bd_cnt: usize,
    pub eth_baseaddr: usize,
    pub dma_baseaddr: usize,
    pub mac_addr: [u8; 6],
    pub mtu: usize
}

const AXI_NET_CONFIG: AxiNetConfig = AxiNetConfig {
    tx_bd_cnt: 1024,
    rx_bd_cnt: 1024,
    eth_baseaddr: ETH_ADDRESS,
    dma_baseaddr: DMA_ADDRESS,
    mac_addr: MAC_ADDRESS,
    mtu: XAE_MAX_JUMBO_FRAME_SIZE,
};

static AXI_DMA: Lazy<Arc<AxiDma>> = Lazy::new(|| Arc::new(AxiDma::new(AXI_DMA_CONFIG)));

static AXI_ETH: Lazy<Arc<Mutex<AxiEthernet>>> = Lazy::new(||  Arc::new(Mutex::new(AxiEthernet::new(
    AXI_NET_CONFIG.eth_baseaddr, AXI_NET_CONFIG.dma_baseaddr
))));

fn dma_init() {
    AXI_DMA.reset().unwrap();
    // enable cyclic mode
    AXI_DMA.cyclic_enable();

    // init cyclic block descriptor
    let _ = AXI_DMA.tx_channel_create_with_translate(AXI_NET_CONFIG.tx_bd_cnt, UserImageUtils::get_heap_paddr).unwrap();
    let _ = AXI_DMA.rx_channel_create_with_translate(AXI_NET_CONFIG.rx_bd_cnt, UserImageUtils::get_heap_paddr).unwrap();
    AXI_DMA.intr_enable();
}

fn eth_init() {
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
    eth.clear_rx_cmplt();
    eth.clear_tx_cmplt();

    eth.start();
}

#[derive(Clone)]
struct AxiNet {
    pub dma: Arc<AxiDma>,
    pub eth: Arc<Mutex<AxiEthernet>>,
}

impl AxiNet {
    pub const fn new(dma: Arc<AxiDma>, eth: Arc<Mutex<AxiEthernet>>) -> Self {
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
        AxiNet::new(AXI_DMA.clone(), AXI_ETH.clone())
    }
}

impl TxToken for AxiNet {
    fn consume<R, F>(self, len: usize, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len].into_boxed_slice();
        let res = f(&mut buffer);
        let len = buffer.len();
        let tmp = Box::into_raw(buffer) as *mut usize as usize;
        let buf_ptr: *mut u8 = UserImageUtils::get_heap_paddr(tmp) as *mut _;
        // debug_println!("tmp: {:#x}, {:#x}", tmp, buf_ptr as usize);

        let buf = BufPtr::new(NonNull::new(buf_ptr).unwrap(), len);
        // let start = get_clock();
        let mut tbuf = self.dma.tx_submit_with_translate(buf, UserImageUtils::get_heap_paddr).unwrap().wait().unwrap();
        // let end = get_clock();
        // debug_println!("send cost: {}", end - start);
        // let vptr = (tbuf.as_mut_ptr() as usize + KERNEL_ELF_BASE_OFFSET) as *mut u8;
        let vptr = UserImageUtils::get_heap_vaddr(tbuf.as_mut_ptr() as usize) as *mut u8;
        let buf = unsafe { core::slice::from_raw_parts_mut(vptr, tbuf.len()) };
        let _box_buf = unsafe { Box::from_raw(buf) };
        // debug_println!("recv box: {:?}", _box_buf);
        drop(_box_buf);
        res
    }
}

impl RxToken for AxiNet {
    fn consume<R, F>(self, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
    {
        // let start = get_clock();
        let mtu = self.capabilities().max_transmission_unit;
        let buffer = vec![0u8; mtu].into_boxed_slice();
        let len = buffer.len();
        let tmp = Box::into_raw(buffer) as *mut usize as usize;
        let buf_ptr: *mut u8 = UserImageUtils::get_heap_paddr(tmp) as *mut _;
        let buf = BufPtr::new(NonNull::new(buf_ptr).unwrap(), len);
        let mut rbuf = self.dma.rx_submit_with_translate(buf, UserImageUtils::get_heap_paddr).unwrap().wait().unwrap();
        let vptr = UserImageUtils::get_heap_vaddr(rbuf.as_mut_ptr() as usize) as *mut u8;
        let buf = unsafe { core::slice::from_raw_parts_mut(vptr, rbuf.len()) };
        let mut box_buf = unsafe { Box::from_raw(buf) };
        // let end = get_clock();
        // debug_println!("recv cost: {}", end - start);
        f(&mut box_buf)
    }
}

impl Device for AxiNet {
    type RxToken<'a> = Self;
    type TxToken<'a> = Self;

    fn receive(
        &mut self,
        _timestamp: Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if self.eth.lock().can_receive() {
            Some((self.clone(), self.clone()))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        if self.dma.tx_channel.as_ref().unwrap().has_free_bd() {
            Some(self.clone())
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 256;
        caps.max_burst_size = None;
        caps
    }
}


static AXI_NET: Lazy<AxiNet> = Lazy::new(|| AxiNet::default());
static SOCKET_SET: Lazy<Arc<Mutex<SocketSet>>> =
    Lazy::new(|| Arc::new(Mutex::new(SocketSet::new(vec![]))));
static INTERFACE: Lazy<Arc<Mutex<Interface>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Interface::new(
        Config::new(AXI_NET.mac()),
        unsafe { &mut *AXI_NET.as_mut_ptr() },
        Instant::ZERO,
    )))
});

fn set_up() {
    INTERFACE.lock().update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(smoltcp::wire::IpCidr::new(smoltcp::wire::IpAddress::v4(172, 16, 1, 2), 30))
            .unwrap()
    });
}

fn iface_poll() {
    INTERFACE.lock().poll(
        Instant::ZERO,
        unsafe { &mut *AXI_NET.as_mut_ptr() },
        &mut SOCKET_SET.lock(),
    );
}


pub fn smoltcp_poll_test(bootinfo: &sel4::BootInfo) {
    crate::device::init(bootinfo);
    dma_init();
    eth_init();
    set_up();
    let rx_buffer = SocketBuffer::new(vec![0u8; 4096]);
    let tx_buffer = SocketBuffer::new(vec![0u8; 4096]);
    
    let mut tcp_socket = Socket::new(rx_buffer, tx_buffer);
    if !tcp_socket.is_open() {
        if tcp_socket.listen(80).is_err() {
            debug_println!("tcp listen error");
            return;
        }
    }
    let socket_handle = SOCKET_SET.lock().add(tcp_socket);
    debug_println!("start test");
    loop {
        // let start = get_clock();
        iface_poll();
        // let end = get_clock();
        // debug_println!("recv cost: {}", end - start);
        let mut socket_sets = SOCKET_SET.lock();
        let tcp_socket = socket_sets.get_mut::<Socket>(socket_handle);
        if tcp_socket.can_recv() {
            if let Ok(_data) = tcp_socket.recv(|data| {
                // debug_println!("data: {:?}", data);
                (data.len(), data)
            }) {
                // matrix_test::<16>();
                let _ = tcp_socket.send_slice(b"connect ok");
            }
        }
        drop(socket_sets);
        // let start = get_clock();
        iface_poll();
        // let end = get_clock();
        // debug_println!("send cost: {}", end - start);
    }
}