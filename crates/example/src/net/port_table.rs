use async_runtime::{coroutine_wake, CoroutineId, IPCItem};
use lose_net_stack::packets::tcp::TCPPacket;
use alloc::{collections::VecDeque, sync::Arc, vec::{self, Vec}};
use spin::Mutex;
use lazy_static::lazy_static;
use sel4_root_task::debug_println;

use crate::async_lib::wake_with_value;

use super::socket::add_socket;
pub struct Port {
    pub port: u16,
    pub receivable: bool,
    pub schedule: VecDeque<CoroutineId>,
}

lazy_static! {
    static ref LISTEN_TABLE: Mutex<Vec<Option<Port>>> =
        unsafe { Mutex::new(Vec::new()) };
}


pub fn check_accept(port: u16, tcp_packet: &TCPPacket) -> Result<(), ()> {
    let mut listen_table = LISTEN_TABLE.lock();
    let mut listen_ports: Vec<&mut Option<Port>> = listen_table
            .iter_mut()
            .filter(|x| match x {
                Some(t) => t.port == port && t.receivable == true,
                None => false,
            })
            .collect();
    if listen_ports.len() == 0 {
        debug_println!("no listen");
        Err(())
    } else {
        let listen_port = listen_ports[0].as_mut().unwrap();
        if let Some(cid) = listen_port.schedule.pop_back() {
            if let Some(socket_idx) = add_socket(tcp_packet.source_ip, tcp_packet.dest_port,
                tcp_packet.source_port, 0, tcp_packet.seq + 1) {
                    let mut ipc_item = IPCItem::default();
                    ipc_item.extend_msg[0] = socket_idx as u16;
                    wake_with_value(&cid, &ipc_item);
                }
            
        }
        Ok(())
    }
}

pub fn listen_block(port: u16, cid: CoroutineId) {
    let mut listen_table = LISTEN_TABLE.lock();
    let mut listen_ports: Vec<&mut Option<Port>> = listen_table
            .iter_mut()
            .filter(|x| match x {
                Some(t) => t.port == port,
                None => false,
            })
            .collect();
    if listen_ports.len() == 0 {
        debug_println!("create new listen port");
        let mut listen_port = Port {
            port,
            receivable: true,
            schedule: VecDeque::new(),
        };
        listen_port.schedule.push_front(cid);
        listen_table.push(Some(listen_port));
    } else {
        let listen_port = listen_ports[0].as_mut().unwrap();
        listen_port.receivable = true;
        listen_port.schedule.push_front(cid);
    }
}