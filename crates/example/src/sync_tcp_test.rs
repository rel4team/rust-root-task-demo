use core::cmp::min;
use core::sync::atomic::AtomicUsize;
use core::{mem::forget, usize};

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{sync::Arc, vec};
use sel4::cap_type::{IRQHandler, TCB};
use sel4::{with_ipc_buffer_mut, MessageInfo};
use sel4::{cap_type::Endpoint, with_ipc_buffer, BootInfo, CPtr, IPCBuffer, LocalCPtr, r#yield, get_clock};
use sel4_root_task::{debug_print, debug_println};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::tcp::{Socket, SocketBuffer};
use smoltcp::wire::IpListenEndpoint;
use spin::Mutex;
use crate::device::{recv_test, transmit_test};
use crate::net::{iface_poll, TcpBuffer, LISTEN_TABLE, POLL_EPS, SOCKET_SET};
use crate::{
    net::{
        sync_recv, sync_listen, sync_send, MessageType, TCP_RX_BUF_LEN, TCP_TX_BUF_LEN
    }, 
    object_allocator::GLOBAL_OBJ_ALLOCATOR
};

struct RecvBlockedTask {
    pub ep: LocalCPtr<Endpoint>,
    pub tcp_buffer: &'static mut TcpBuffer,
    pub handler: SocketHandle,
    pub complete: bool,
}

lazy_static::lazy_static! {
    static ref RECV_BLOCKED_TASKS: Mutex<Vec<RecvBlockedTask>> = Mutex::new(Vec::new());
}

struct SyncArgs {
    ep: CPtr,
    tcb: Option<LocalCPtr<TCB>>,
}

impl SyncArgs {
    pub fn new(ep: CPtr) -> Self {
        Self {
            ep,
            tcb: None
        }
    }

    pub fn from_ptr(ptr: usize) -> &'static mut Self {
        unsafe {
            &mut *(ptr as *mut Self)
        }
    }

    pub fn get_ptr(&self) -> usize {
        self as *const Self as usize
    }
}

static THREDA_NUM_BITS: usize = 5;
static THREAD_NUM: usize = 1 << THREDA_NUM_BITS;
static mut COMPLETE_CNT: u8 = 0u8;

#[inline]
fn net_interrupt_handler(handler: LocalCPtr<IRQHandler>) {
    iface_poll(true);
    crate::device::interrupt_handler();
    handler.irq_handler_ack();
}

pub fn net_stack_test(boot_info: &BootInfo) -> sel4::Result<!> {
    crate::device::init(boot_info);
    // transmit_test();
    // recv_test();
    let (ntfn, handler) = crate::net::init();
    // BootInfo::init_thread_tcb().tcb_suspend()?;
    let eps = create_c_s_ipc_channel(THREDA_NUM_BITS);
    let thread_num = 1 << THREDA_NUM_BITS;
    loop {
        let mut listen_cnt = 0;
        for ep in eps.iter() {
            let (msg, badge) = ep.nb_recv(());
            if badge != 0 {
                if badge == 1 {
                    net_interrupt_handler(handler);
                } else {
                    listen_cnt += 1;
                    process_req(ep.clone());
                }
            }
        }
        if listen_cnt == THREAD_NUM {
            break;
        }
    }

    loop {
        let (msg, badge) = ntfn.poll();
        if badge == 1 {
            net_interrupt_handler(handler);
        }
        let eps = POLL_EPS.lock();
        for ep in eps.iter() {
            let (msg, badge) = ep.nb_recv(());
            if badge != 0 {
                if badge == 1 {
                    net_interrupt_handler(handler);
                } else {
                    process_req(ep.clone());
                }
            }
        }
        let mut recv_blocked_tasks = RECV_BLOCKED_TASKS.lock();
        for task in recv_blocked_tasks.iter_mut() {
            process_blocked_task(task);
        }
        recv_blocked_tasks.retain(|task| task.complete == false);
    }

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}

fn process_blocked_task(task: &mut RecvBlockedTask) {
    let handler = task.handler;
    let tcp_buffer = &mut task.tcp_buffer;
    let ep = task.ep.clone();
    let mut bindings = SOCKET_SET.lock();
    let socket: &mut Socket = bindings.get_mut(handler);
    if socket.can_recv() {
        if let Ok(read_size) = socket.recv_slice(&mut tcp_buffer.data) {
            let reply = MessageInfo::new(0, 0, 0, 2);
            with_ipc_buffer_mut(
                |ipc_buf| {
                    ipc_buf.msg_regs_mut()[0] = MessageType::RecvReply as u64;
                    ipc_buf.msg_regs_mut()[1] = read_size as u64;
                }
            );
            ep.send(reply);
            task.complete = true;
        }
    }
}

fn process_req(ep: LocalCPtr<Endpoint>) {
    let msg_type = with_ipc_buffer(
        |ipc_buffer| {
            unsafe {
                core::mem::transmute::<u8, MessageType>(ipc_buffer.msg_regs()[0] as u8)
            }
        }
    );
    match msg_type {
        MessageType::Listen => {
            let port = with_ipc_buffer(
                |ipc_buf| ipc_buf.msg_regs()[1] as u16
            );
            let tcp_rx_buffer = SocketBuffer::new(vec![0; TCP_RX_BUF_LEN]);
            let tcp_tx_buffer = SocketBuffer::new(vec![0; TCP_TX_BUF_LEN]);
            let mut tcp_socket = Socket::new(tcp_rx_buffer, tcp_tx_buffer);
            tcp_socket.set_ack_delay(None);
            tcp_socket.set_nagle_enabled(false);
            // debug_println!("port: {}", port);

            tcp_socket.listen(port).unwrap();
            let mut endpoint = IpListenEndpoint::default();
            endpoint.port = port;
            let handler = SOCKET_SET.lock().add(tcp_socket);
            unsafe {
                LISTEN_TABLE.listen_with_ep(endpoint, handler, ep)
            }
        }

        MessageType::Recv => {
            let (handler, tcp_buffer, len) = unsafe {
                with_ipc_buffer(
                    |ipc_buf| {
                        (
                            core::mem::transmute::<usize, SocketHandle>(ipc_buf.msg_regs()[1] as usize),
                            &mut *(ipc_buf.msg_regs()[2] as usize as *mut TcpBuffer),
                            ipc_buf.msg_regs()[3] as usize
                        )
                    }
                )
            };
            let mut bindings = SOCKET_SET.lock();
            let socket: &mut Socket = bindings.get_mut(handler);
            if socket.can_recv() {
                let min_len = min(tcp_buffer.data.len(), len);
                if let Ok(read_size) = socket.recv_slice(&mut tcp_buffer.data[..min_len]) {
                    let reply = MessageInfo::new(0, 0, 0, 2);
                    with_ipc_buffer_mut(
                        |ipc_buf| {
                            ipc_buf.msg_regs_mut()[0] = MessageType::RecvReply as u64;
                            ipc_buf.msg_regs_mut()[1] = read_size as u64;
                        }
                    );
                    ep.send(reply);
                }
            } else {
                RECV_BLOCKED_TASKS.lock().push(
                    RecvBlockedTask {
                        ep: ep.clone(),
                        tcp_buffer,
                        handler,
                        complete: false,
                    }
                );
            }
        }

        MessageType::Send => {
            let (handler, tcp_buffer, len) = unsafe {
                with_ipc_buffer(
                    |ipc_buf| {
                        (
                            core::mem::transmute::<usize, SocketHandle>(ipc_buf.msg_regs()[1] as usize),
                            &mut *(ipc_buf.msg_regs()[2] as usize as *mut TcpBuffer),
                            ipc_buf.msg_regs()[3] as usize
                        )
                    }
                )
            };
            let mut bindings = SOCKET_SET.lock();
            let socket: &mut Socket = bindings.get_mut(handler);
            if socket.can_send() {
                let send_data = &mut tcp_buffer.data[0..len];
                if let Ok(send_size) = socket.send_slice(&send_data) {
                    drop(bindings);
                    iface_poll(true);
                    let reply = MessageInfo::new(0, 0, 0, 2);
                    with_ipc_buffer_mut(
                        |ipc_buf| {
                            ipc_buf.msg_regs_mut()[0] = MessageType::SendReply as u64;
                            ipc_buf.msg_regs_mut()[1] = send_size as u64;
                        }
                    );
                    ep.send(reply);
                }
            } else {
                // 假设所有数据大小都不大于socket buffer
                // panic!("fail to send");
                drop(bindings);
            }

        }
        _ => {

        }
    }
}

fn create_c_s_ipc_channel(thread_num_bits: usize) -> Vec<LocalCPtr<Endpoint>> {
    let thread_num = 1 << thread_num_bits;
    let cnode = BootInfo::init_thread_cnode();
    let mut eps = GLOBAL_OBJ_ALLOCATOR.lock().alloc_many_ep(thread_num_bits);
    let mut args = Vec::new();
    for i in 0..thread_num {
        let ep = eps[i];
        let badge = (i + 2) as u64;
        let badge_ep = BootInfo::init_cspace_local_cptr::<Endpoint>(
            GLOBAL_OBJ_ALLOCATOR.lock().get_empty_slot()
        );

        cnode.relative(badge_ep).mint(
            &cnode.relative(ep),
            sel4::CapRights::all(),
            badge,
        ).unwrap();
        
        let sync_args = {
            let ref_args = Arc::new(SyncArgs::new(badge_ep.cptr()));
            let leaky_ref = unsafe { &mut *(ref_args.as_ref() as *const SyncArgs as usize as *mut SyncArgs) };
            forget(ref_args);
            leaky_ref
        };
        args.push(sync_args.get_ptr());
    }
    
    let tcbs = GLOBAL_OBJ_ALLOCATOR.lock().create_many_threads(thread_num_bits, tcp_server, args.clone(), 255, 0, false);
    for i in 0..thread_num {
        let sync_arg = SyncArgs::from_ptr(args[i]);
        sync_arg.tcb = Some(tcbs[i].clone());
    }

    for tcb in tcbs {
        tcb.tcb_resume().unwrap()
    }
    eps
}

fn tcp_server(args: usize, ipc_buffer_addr: usize) {
    let arg = SyncArgs::from_ptr(args);
    let ep = LocalCPtr::<Endpoint>::from_cptr(arg.ep);
    let ipc_buffer = ipc_buffer_addr as *mut sel4::sys::seL4_IPCBuffer;
    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer)
    };
    sel4::set_ipc_buffer(ipcbuf);
    let thread = arg.tcb.unwrap();
    let send = true;
    let recv = true;
    debug_println!("start listen");
    let listen_fd = sync_listen(80, ep).unwrap();
    let mut tcp_buffer = Box::new(TcpBuffer::new());
    // debug_println!("accept success!, fd: {:?}", listen_fd);
    let start = get_clock();
    loop {
        if recv {
            if let Ok(recv_size) = sync_recv(listen_fd, tcp_buffer.as_mut(), 1) {
                // debug_println!("recv success, recv_size: {}", recv_size);
                if tcp_buffer.data[0] == '.' as u8 {
                    break;
                }
            } else {
                panic!("recv fail!");
            }
        }

        if send {
            // let resp_str = String::from("connect ok!");
            let resp_str = String::from("!");
            let resp = resp_str.as_bytes();
            for i in 0..resp.len() {
                tcp_buffer.data[i] = resp[i];
            }
            // let start = get_clock();
            if let Ok(_send_size) = sync_send(listen_fd, tcp_buffer.as_ref(), resp.len()) {
                // debug_println!("send success, send_size: {}", _send_size);
            }
        }
    }
    let resp_str = String::from(".");
    let resp = resp_str.as_bytes();
    for i in 0..resp.len() {
        tcp_buffer.data[i] = resp[i];
    }
    sync_send(listen_fd, tcp_buffer.as_ref(), resp.len());
    unsafe {
        COMPLETE_CNT += 1;
    }
    // debug_println!("client cost: {}", get_clock() - start);
    thread.tcb_suspend().unwrap();
    // loop {
    //     r#yield();
    // }
}
