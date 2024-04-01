use crate::{device::NET_DEVICE, net::{port_table::check_accept, socket::{get_s_a_by_index, get_socket, push_data}}};
use sel4_root_task::debug_println;
use spin::Mutex;
use lazy_static;
use alloc::sync::Arc;
use lose_net_stack::{results::Packet, LoseStack, MacAddress, TcpFlags, IPv4};
pub struct NetStack(pub LoseStack);

impl NetStack {
    pub fn new() -> Self {
        unsafe {
            NetStack(LoseStack::new(
                IPv4::new(10, 0, 2, 15),
                MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            ))
        }
    }
}

lazy_static::lazy_static! {
    pub static ref LOSE_NET_STACK: Arc<NetStack> = Arc::new(NetStack::new());
}

pub fn net_interrupt_handler() {
    match NET_DEVICE.receive() {
        Some(buf) => {
            let packet = LOSE_NET_STACK.0.analysis(buf.packet());
            match packet {
                Packet::ARP(arp_packet) => {
                    let lose_stack = &LOSE_NET_STACK.0;
                    let reply_packet = arp_packet
                        .reply_packet(lose_stack.ip, lose_stack.mac)
                        .expect("can't build reply");
                    let reply_data = reply_packet.build_data();
                    NET_DEVICE.transmit(&reply_data)
                }

                Packet::TCP(tcp_packet) => {
                    let target = tcp_packet.source_ip;
                    let lport = tcp_packet.dest_port;
                    let rport = tcp_packet.source_port;
                    let flags = tcp_packet.flags;
                    // debug_println!("[TCP] target: {}, lport: {}, rport: {}", target, lport, rport);
                    if flags.contains(TcpFlags::S) {
                        if let Ok(()) = check_accept(lport, &tcp_packet) {
                            let mut reply_packet = tcp_packet.ack();
                            reply_packet.flags = TcpFlags::S | TcpFlags::A;
                            NET_DEVICE.transmit(&reply_packet.build_data());
                        } else {
                            debug_println!("check accpet fail");
                        }
                        NET_DEVICE.recycle_rx_buffer(buf);
                        return;
                    } else if tcp_packet.flags.contains(TcpFlags::F) {
                        let reply_packet = tcp_packet.ack();
                        NET_DEVICE.transmit(&reply_packet.build_data());
                        let mut end_packet = reply_packet.ack();
                        end_packet.flags |= TcpFlags::F;
                        NET_DEVICE.transmit(&end_packet.build_data());
                    } else if tcp_packet.flags.contains(TcpFlags::A) && tcp_packet.data_len == 0 {
                        let reply_packet = tcp_packet.ack();
                        NET_DEVICE.transmit(&reply_packet.build_data());
                        NET_DEVICE.recycle_rx_buffer(buf);
                        return;
                    } else {
                        let reply_packet = tcp_packet.ack();
                        NET_DEVICE.transmit(&reply_packet.build_data());
                    }
                    if let Some(socket_index) = get_socket(target, lport, rport) {
                        let packet_seq = tcp_packet.seq;
                        if let Some((seq, ack)) = get_s_a_by_index(socket_index) {
                            // debug_println!("packet_seq: {}, ack: {}", packet_seq, ack);
                            if ack == packet_seq && tcp_packet.data_len > 0 {
                                push_data(socket_index, &tcp_packet);
                            }
                        }
                    }
                }
                _ => {}
            }
            NET_DEVICE.recycle_rx_buffer(buf);
        },
        None => {
            // debug_println!("do nothing");
        }
    }
}