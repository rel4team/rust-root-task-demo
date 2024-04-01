use super::TCP_TX_BUF_LEN;

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct TcpBuffer {
    pub data: [u8; TCP_TX_BUF_LEN],
}

impl TcpBuffer {
    pub const fn new() -> Self {
        Self {
            data: [0u8; TCP_TX_BUF_LEN]
        }
    }

    pub fn get_ptr(&self) -> usize {
        self as *const Self as usize
    }
}

