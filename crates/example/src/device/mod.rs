use alloc::sync::Arc;
use alloc::vec;
use smoltcp::iface::SocketSet;
use spin::{Lazy, Mutex};
use sel4::BootInfo;

// #[cfg(feature = "board_qemu")]
mod config;
mod net;

pub use net::init_net_interrupt_handler;

pub use net::{INTERFACE, NET_DEVICE};

pub static SOCKET_SET: Lazy<Arc<Mutex<SocketSet>>> =
    Lazy::new(|| Arc::new(Mutex::new(SocketSet::new(vec![]))));

pub fn init(boot_info: &BootInfo) {
    // #[cfg(feature = "board_qemu")]
    net::init(boot_info);
}
