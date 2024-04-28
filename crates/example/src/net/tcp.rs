use smoltcp::iface::SocketHandle;
use async_runtime::coroutine_get_current;
use crate::async_lib::{seL4_Call_with_item, SenderID};
use crate::net::message::{MessageBuilder, MessageDecoder, MessageType};
use crate::net::NET_STACK_MAP;
use crate::net::tcp_buffer::TcpBuffer;


pub async fn listen(port: usize, nw_sender_id: &SenderID) -> Result<SocketHandle, ()> {
    let message = MessageBuilder::listen(coroutine_get_current(), port);
    // debug_println!("[listen] message: {:?}", message);
    if let Ok(reply) = seL4_Call_with_item(nw_sender_id, &message).await {
        assert_eq!(MessageDecoder::get_type(&reply), MessageType::ListenReply);
        let handler = MessageDecoder::get_socket_handler(&reply);
        unsafe { NET_STACK_MAP.insert(handler, *nw_sender_id); }
        return Ok(handler);
    }
    return Err(());
}


pub async fn send(handler: SocketHandle, buffer: &TcpBuffer, len: usize) -> Result<usize, ()> {
    let nw_sender_id = unsafe { NET_STACK_MAP.get(&handler).unwrap() };
    let message = MessageBuilder::send(coroutine_get_current(), handler, buffer, len);
    if let Ok(reply) = seL4_Call_with_item(nw_sender_id, &message).await {
        assert_eq!(MessageDecoder::get_type(&reply), MessageType::SendReply);
        return Ok(MessageDecoder::get_len(&reply));
    }
    return Err(());
}

pub async fn recv(handler: SocketHandle, buffer: &mut TcpBuffer, len: usize) -> Result<usize, ()> {
    let nw_sender_id = unsafe { NET_STACK_MAP.get(&handler).unwrap() };
    let message = MessageBuilder::recv(coroutine_get_current(), handler, buffer, len);
    if let Ok(reply) = seL4_Call_with_item(nw_sender_id, &message).await {
        assert_eq!(MessageDecoder::get_type(&reply), MessageType::RecvReply);
        return Ok(MessageDecoder::get_len(&reply));
    }
    return Err(());
}