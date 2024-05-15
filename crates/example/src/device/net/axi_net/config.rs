use axi_dma::AxiDmaConfig;
use axi_ethernet::XAE_MAX_JUMBO_FRAME_SIZE;

pub const ETH_ADDRESS: usize = 0x6014_0000;
pub const DMA_ADDRESS: usize = 0x6010_0000;
pub const MAC_ADDRESS: [u8; 6] = [0x00, 0x0A, 0x35, 0x01, 0x02, 0x03];
pub const MTU: usize = 256;
pub const AXI_DMA_CONFIG: AxiDmaConfig = AxiDmaConfig {
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

pub struct AxiNetConfig {
    pub tx_bd_cnt: usize,
    pub rx_bd_cnt: usize,
    pub eth_baseaddr: usize,
    pub dma_baseaddr: usize,
    pub mac_addr: [u8; 6],
    pub mtu: usize
}

pub const AXI_NET_CONFIG: AxiNetConfig = AxiNetConfig {
    tx_bd_cnt: 1024,
    rx_bd_cnt: 1024,
    eth_baseaddr: ETH_ADDRESS,
    dma_baseaddr: DMA_ADDRESS,
    mac_addr: MAC_ADDRESS,
    mtu: XAE_MAX_JUMBO_FRAME_SIZE,
};
