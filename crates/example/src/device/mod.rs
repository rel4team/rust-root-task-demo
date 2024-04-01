use sel4::BootInfo;

// #[cfg(feature = "board_qemu")]
mod config;
mod net;

pub use net::init_net_interrupt_handler;

pub use net::NET_DEVICE;
use sel4_root_task::debug_println;

pub fn init(boot_info: &BootInfo) {
    // #[cfg(feature = "board_qemu")]
    net::init(boot_info);
}
