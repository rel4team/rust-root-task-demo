pub struct NetConfig {
    pub mac_addr: [u8; 6],
    pub mtu: usize,
}

pub const NET_CONFIG: NetConfig = NetConfig {
    mac_addr: [0, 0, 0, 0, 0, 0],
    mtu: 5000,
};