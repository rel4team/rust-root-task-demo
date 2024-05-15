use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address};
use sel4::BootInfo;


mod net;

pub use net::{init_net_interrupt_handler, interrupt_handler};
pub use net::{transmit_test, recv_test};

pub use net::{INTERFACE, NET_DEVICE};
use sel4_root_task::debug_println;

// pub fn init(boot_info: &BootInfo) {
//     // #[cfg(feature = "board_qemu")]
//     net::init(boot_info);
//     INTERFACE.lock().update_ip_addrs(|ip_addrs| {
//         ip_addrs
//             .push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24))
//             .unwrap()
//     });
//     INTERFACE.lock().routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2)).unwrap();
//     let interface = INTERFACE.lock();
//     debug_println!("device init, mac addr: {:?}, ip_addr: {:?}", interface.hardware_addr(), interface.ip_addrs());
// }

pub fn init(boot_info: &BootInfo) {
    // #[cfg(feature = "board_qemu")]
    net::init(boot_info);
    INTERFACE.lock().update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::v4(172, 16, 1, 2), 30))
            .unwrap()
    });
    let interface = INTERFACE.lock();
    debug_println!("device init, mac addr: {:?}, ip_addr: {:?}", interface.hardware_addr(), interface.ip_addrs());
}