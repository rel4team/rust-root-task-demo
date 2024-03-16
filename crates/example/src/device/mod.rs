use sel4::BootInfo;

// #[cfg(feature = "board_qemu")]
mod config;
mod net;



pub fn init(boot_info: &BootInfo) {
    // #[cfg(feature = "board_qemu")]
    net::init(boot_info);
}
