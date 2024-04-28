use sel4::{cap_type::Endpoint, with_ipc_buffer, with_ipc_buffer_mut, LocalCPtr, MessageInfo};
use sel4_root_task::debug_println;
use smoltcp::iface::SocketHandle;

use super::{MessageType, TcpBuffer, NET_STACK_MAP, NET_STACK_MAP2};

pub fn sync_listen(port: u16, ep: LocalCPtr<Endpoint>) -> Result<SocketHandle, ()> {
    let msg = MessageInfo::new(0, 0, 0, 2);

    with_ipc_buffer_mut(
        |ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = MessageType::Listen as u64;
            ipc_buf.msg_regs_mut()[1] = port as u64;
        }
    );
    ep.send(msg);
    // debug_println!("send end");
    ep.recv(());
    // debug_println!("recv end");
    let handler = with_ipc_buffer(
        |ipc_buf| ipc_buf.msg_regs()[1]
    );
    let res = unsafe {
        core::mem::transmute::<usize, SocketHandle>(handler as usize)
    };
    unsafe {
        NET_STACK_MAP2.insert(res, ep.clone());
    }
    Ok(res)
}

pub fn sync_send(handler: SocketHandle, buffer: &TcpBuffer, len: usize) -> Result<usize, ()> {
    let msg = MessageInfo::new(0, 0, 0, 4);
    with_ipc_buffer_mut(
        |ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = MessageType::Send as u64;
            ipc_buf.msg_regs_mut()[1] = unsafe {
                core::mem::transmute::<SocketHandle, u64>(handler)
            };
            ipc_buf.msg_regs_mut()[2] = buffer.get_ptr() as u64;
            ipc_buf.msg_regs_mut()[3] = len as u64;
        }
    );
    let ep = unsafe {
        NET_STACK_MAP2.get(&handler).unwrap()
    };
    ep.send(msg);
    ep.recv(());
    let len = with_ipc_buffer(
        |ipc_buf| ipc_buf.msg_regs()[1]
    );
    Ok(len as usize)
}

pub fn sync_recv(handler: SocketHandle, buffer: &mut TcpBuffer, len: usize) -> Result<usize, ()> {
    let ep = unsafe {
        NET_STACK_MAP2.get(&handler).unwrap()
    };
    let msg = MessageInfo::new(0, 0, 0, 4);
    with_ipc_buffer_mut(
        |ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = MessageType::Recv as u64;
            ipc_buf.msg_regs_mut()[1] = unsafe {
                core::mem::transmute::<SocketHandle, u64>(handler)
            };
            ipc_buf.msg_regs_mut()[2] = buffer.get_ptr() as u64;
            ipc_buf.msg_regs_mut()[3] = len as u64;
        }
    );
    ep.send(msg);
    ep.recv(());
    let len = with_ipc_buffer(
        |ipc_buf| ipc_buf.msg_regs()[1]
    );
    Ok(len as usize)
}