use sel4::BootInfo;

// #[cfg(feature = "board_qemu")]
mod virt_net;

static mut NET_DEVICE: usize = 0;
pub struct NetDevice;

pub fn init(boot_info: &BootInfo) {
    // #[cfg(feature = "board_qemu")]
    virt_net::init(boot_info);
}
