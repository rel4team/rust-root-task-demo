use alloc::boxed::Box;
use alloc::collections::VecDeque;


use core::ops::DerefMut;
use smoltcp::iface::{SocketHandle, SocketSet};

use smoltcp::wire::{IpAddress, IpEndpoint, IpListenEndpoint};
use spin::{Lazy, Mutex};
use async_runtime::{coroutine_wake, CoroutineId};
use sel4_logging::log::{debug, warn};
use smoltcp::socket::tcp::{Socket, State};
use crate::net::SOCKET_SET;

const LISTEN_QUEUE_SIZE: usize = 4096;
const PORT_NUM: usize = 65536;


pub static mut LISTEN_TABLE: Lazy<ListenTable> = Lazy::new(|| ListenTable::new());

struct ListenTableEntry {
    listen_endpoint: IpListenEndpoint,
    syn_queue: VecDeque<SocketHandle>,
    block_cids: VecDeque<CoroutineId>,
}

impl ListenTableEntry {
    pub fn new(listen_endpoint: IpListenEndpoint) -> Self {
        Self {
            listen_endpoint,
            syn_queue: VecDeque::with_capacity(LISTEN_QUEUE_SIZE),
            block_cids: VecDeque::with_capacity(LISTEN_QUEUE_SIZE),
        }
    }

    #[inline]
    fn can_accept(&self, dst: IpAddress) -> bool {
        match self.listen_endpoint.addr {
            Some(addr) => addr == dst,
            None => true,
        }
    }
}

impl Drop for ListenTableEntry {
    fn drop(&mut self) {
        for &handle in &self.syn_queue {
            SOCKET_SET.lock().remove(handle);
        }
    }
}

pub struct ListenTable {
    tcp: Box<[Mutex<Option<Box<ListenTableEntry>>>]>,
}


impl ListenTable {
    pub fn new() -> Self {
        let tcp = unsafe {
            let mut buf = Box::new_uninit_slice(PORT_NUM);
            for i in 0..PORT_NUM {
                buf[i].write(Mutex::new(None));
            }
            buf.assume_init()
        };
        Self { tcp }
    }

    pub fn can_listen(&self, port: u16) -> bool {
        self.tcp[port as usize].lock().is_none()
    }

    pub fn listen(&self, listen_endpoint: IpListenEndpoint, handler: SocketHandle, cid: CoroutineId) -> Result<(), ()> {
        let port = listen_endpoint.port;
        assert_ne!(port, 0);
        let mut entry = self.tcp[port as usize].lock();
        if entry.is_none() {
            *entry = Some(Box::new(ListenTableEntry::new(listen_endpoint)));

        }
        let en: &mut Box<ListenTableEntry> = entry.as_mut().unwrap();
        en.syn_queue.push_back(handler);
        en.block_cids.push_back(cid);
        // debug_println!("listen push");
        Ok(())
    }

    pub fn unlisten(&self, port: u16) {
        debug!("TCP socket unlisten on {}", port);
        *self.tcp[port as usize].lock() = None;
    }

    pub fn blocked_cid(&self, port: u16, cid: CoroutineId) {
        if let Some(entry) = self.tcp[port as usize].lock().deref_mut() {
            let blocked_cids = &mut entry.block_cids;
            blocked_cids.push_back(cid);
        }
    }

    pub fn accept(&self, port: u16) -> Result<(SocketHandle, (IpEndpoint, IpEndpoint)), ()> {
        if let Some(entry) = self.tcp[port as usize].lock().deref_mut() {
            let syn_queue = &mut entry.syn_queue;
            assert!(!syn_queue.is_empty());
            let handle = syn_queue.pop_front().unwrap();
            // debug!("[accept] handler: {}", handle);
            assert!(is_connected(handle));
            Ok((handle, get_addr_tuple(handle)))
        } else {
            Err(())
        }
    }

    pub fn incoming_tcp_packet(
        &self,
        _src: IpEndpoint,
        dst: IpEndpoint,
        _sockets: &mut SocketSet<'_>,
    ) {
        if let Some(entry) = self.tcp[dst.port as usize].lock().deref_mut() {
            // debug!("dst.port: {}", dst.port);
            if !entry.can_accept(dst.addr) {
                // not listening on this address
                return;
            }
            // debug!("incoming_tcp_packet");
            if entry.syn_queue.len() >= LISTEN_QUEUE_SIZE {
                // SYN queue is full, drop the packet
                warn!("SYN queue overflow!");
                return;
            }

            if !entry.block_cids.is_empty() {
                // debug!("wake cid");
                coroutine_wake(&entry.block_cids.pop_front().unwrap());
            }
        }
    }
}

#[inline]
fn is_connected(handle: SocketHandle) -> bool {
    let bindings = SOCKET_SET.lock();
    let sock: &Socket = bindings.get(handle);
    sock.state() == State::Established
}

#[inline]
fn get_addr_tuple(handle: SocketHandle) -> (IpEndpoint, IpEndpoint) {
    let bindings = SOCKET_SET.lock();
    let sock: &Socket = bindings.get(handle);
    (sock.local_endpoint().unwrap(), sock.remote_endpoint().unwrap())
}



pub fn snoop_tcp_packet(buf: &[u8], sockets: &mut SocketSet<'_>) -> Result<(), smoltcp::wire::Error> {
    use smoltcp::wire::{EthernetFrame, IpProtocol, Ipv4Packet, TcpPacket};

    let ether_frame = EthernetFrame::new_checked(buf)?;
    let ipv4_packet = Ipv4Packet::new_checked(ether_frame.payload())?;


    // debug!("[snoop_tcp_packet] arp_packet target_addr: {:?}, operator: {:?}, ether_frame.dst_addr: {:?}, {}, {}", arp_packet.target_protocol_addr(),
    //     arp_packet.operation(), ether_frame.dst_addr(), res1, res2);
    if ipv4_packet.next_header() == IpProtocol::Tcp {
        // debug_println!("snoop_tcp_packet");
        let tcp_packet = TcpPacket::new_checked(ipv4_packet.payload())?;
        let src_addr = (ipv4_packet.src_addr(), tcp_packet.src_port()).into();
        let dst_addr = (ipv4_packet.dst_addr(), tcp_packet.dst_port()).into();
        let is_first = tcp_packet.syn() && !tcp_packet.ack();
        if is_first {
            // debug_println!("incoming_tcp_packet");   
            // create a socket for the first incoming TCP packet, as the later accept() returns.
            unsafe {
                LISTEN_TABLE.incoming_tcp_packet(src_addr, dst_addr, sockets);
            }
        }
    }
    Ok(())
}