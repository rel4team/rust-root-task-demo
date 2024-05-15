use smoltcp::iface::SocketHandle;
use async_runtime::{CoroutineId, IPCItem};
use crate::net::tcp_buffer::TcpBuffer;

pub struct MessageBuilder;
pub struct MessageDecoder;

#[derive(PartialOrd, PartialEq, Debug)]
pub enum MessageType {
    Reserve = 0,
    NetPollReq,
    Listen,
    ListenReply,
    Send,
    SendReply,
    Recv,
    RecvReply,
}

const INVALID_TYPE: u32 = MessageType::RecvReply as u32 + 1;

impl MessageBuilder {
    #[inline]
    pub fn listen(cid: CoroutineId, port: usize) -> IPCItem {
        let mut item = IPCItem::default();
        item.cid = cid;
        item.msg_info = MessageType::Listen as u32;
        item.extend_msg[0] = port as u16;
        item
    }

    #[inline]
    pub fn listen_reply(cid: CoroutineId, handler: SocketHandle) -> IPCItem {
        let mut item = IPCItem::default();
        item.cid = cid;
        item.msg_info = MessageType::ListenReply as u32;
        item.extend_msg[0] = unsafe { core::mem::transmute::<SocketHandle, usize>(handler) as u16 };
        item
    }

    #[inline]
    pub fn send(cid: CoroutineId, handler: SocketHandle, buffer: &TcpBuffer, len: usize) -> IPCItem {
        let mut item = IPCItem::default();
        item.cid = cid;
        item.msg_info = MessageType::Send as u32;
        item.extend_msg[0] = unsafe { core::mem::transmute::<SocketHandle, usize>(handler) as u16 };
        item.extend_msg[1] = len as u16;
        let ptr = unsafe {
            &mut *(item.extend_msg.as_ptr().add(4) as usize as *mut usize)
        };
        *ptr = buffer as *const TcpBuffer as usize;
        item
    }

    #[inline]
    pub fn send_reply(cid: CoroutineId, send_size: usize) -> IPCItem {
        let mut item = IPCItem::default();
        item.cid = cid;
        item.msg_info = MessageType::SendReply as u32;
        item.extend_msg[1] = send_size as u16;
        item
    }

    #[inline]
    pub fn recv(cid: CoroutineId, handler: SocketHandle, buffer: &mut TcpBuffer, len: usize) -> IPCItem {
        let mut item = IPCItem::default();
        item.cid = cid;
        item.msg_info = MessageType::Recv as u32;
        item.extend_msg[0] = unsafe { core::mem::transmute::<SocketHandle, usize>(handler) as u16 };
        item.extend_msg[1] = len as u16;
        let ptr = unsafe {
            &mut *(item.extend_msg.as_ptr().add(4) as usize as *mut usize)
        };
        *ptr = buffer as *mut TcpBuffer as usize;
        item
    }

    #[inline]
    pub fn recv_reply(cid: CoroutineId, read_size: usize) -> IPCItem {
        let mut item = IPCItem::default();
        item.cid = cid;
        item.msg_info = MessageType::RecvReply as u32;
        item.extend_msg[1] = read_size as u16;
        item
    }
}

impl MessageDecoder {
    #[inline]
    pub fn get_cid(item: &IPCItem) -> CoroutineId {
        item.cid
    }

    #[inline]
    pub fn get_port(item: &IPCItem) -> usize {
        item.extend_msg[0] as usize
    }

    #[inline]
    pub fn get_type(item: &IPCItem) -> MessageType {
        unsafe {
            assert!(item.msg_info < INVALID_TYPE);
            core::mem::transmute::<u8, MessageType>(item.msg_info as u8)
        }
    }

    #[inline]
    pub fn get_socket_handler(item: &IPCItem) -> SocketHandle {
        unsafe {
            core::mem::transmute::<usize, SocketHandle>(item.extend_msg[0] as usize)
        }
    }

    #[inline]
    pub fn get_len(item: &IPCItem) -> usize {
        item.extend_msg[1] as usize
    }

    #[inline]
    pub fn get_buffer(item: &IPCItem) -> &'static mut TcpBuffer {
        unsafe {
            let ptr = *(item.extend_msg.as_ptr().add(4) as usize as *mut usize);
            &mut *(ptr as *mut TcpBuffer)
        }
    }
}