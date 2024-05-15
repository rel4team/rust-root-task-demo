mod tcp;
mod message;
mod tcp_buffer;
mod listen_table;
mod sync_tcp;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use smoltcp::wire::IpEndpoint;
use core::cmp::min;
use core::sync::atomic::Ordering::SeqCst;
use smoltcp::iface::{SocketHandle, SocketSet};
use smoltcp::socket::tcp::{Socket, SocketBuffer};
use smoltcp::time::Instant;
use spin::{Lazy, Mutex};
use async_runtime::{coroutine_get_current, coroutine_spawn_with_prio, coroutine_wake, get_ready_num, runtime_init, CoroutineId, IPCItem};
use sel4::cap_type::{Endpoint, IRQHandler, Notification};
use sel4::LocalCPtr;

use sel4_root_task::debug_println;
use uintr::{register_receiver, uipi_send};
use crate::async_lib::{register_recv_cid, uintr_handler, wake_with_value, yield_now, AsyncArgs, SenderID, possible_switch};
use crate::device::{init_net_interrupt_handler, interrupt_handler, INTERFACE, NET_DEVICE};

use sel4::get_clock;
pub use tcp::*;
pub use sync_tcp::*;
pub use message::*;
pub use listen_table::{snoop_tcp_packet, LISTEN_TABLE, POLL_EPS};
pub use tcp_buffer::*;
use smoltcp::wire::IpListenEndpoint;

pub const TCP_TX_BUF_LEN: usize = 8 * 4096;
pub const TCP_RX_BUF_LEN: usize = 8 * 4096;

#[thread_local]
static mut NET_STACK_MAP: BTreeMap<SocketHandle, SenderID> = BTreeMap::new();

#[thread_local]
static mut NET_STACK_MAP2: BTreeMap<SocketHandle, LocalCPtr<Endpoint>> = BTreeMap::new();

pub static SOCKET_SET: Lazy<Arc<Mutex<SocketSet>>> =
    Lazy::new(|| Arc::new(Mutex::new(SocketSet::new(vec![]))));


pub static SOCKET_2_CID: Lazy<Arc<Mutex<BTreeMap<SocketHandle, CoroutineId>>>> =
    Lazy::new(|| Arc::new(Mutex::new(BTreeMap::new())));

pub static ADDR_2_CID: Lazy<Arc<Mutex<BTreeMap<IpEndpoint, CoroutineId>>>> =
    Lazy::new(|| Arc::new(Mutex::new(BTreeMap::new())));

pub fn init() -> (LocalCPtr<Notification>, LocalCPtr<IRQHandler>){
    runtime_init();
    let (net_handler, net_ntfn) = init_net_interrupt_handler();
    let tcb = sel4::BootInfo::init_thread_tcb();
    tcb.tcb_bind_notification(net_ntfn).unwrap();
    register_receiver(tcb, net_ntfn, uintr_handler as usize).unwrap();

    let cid = coroutine_spawn_with_prio(Box::pin(net_poll(net_handler.clone())), 0);
    unsafe {
        NET_DEVICE_POLLER_CID = cid;
    }
    // let _ = coroutine_spawn_with_prio(Box::pin(poll_timer(get_clock())), 2);
    // debug_println!("init cid: {:?}", cid);
    // let badge = register_recv_cid(&cid).unwrap() as u64;
    // assert_eq!(badge, 0);
    return (net_ntfn, net_handler);
}

fn wake_net_device_poller() {
    unsafe {
        coroutine_wake(&NET_DEVICE_POLLER_CID);
    }
}

#[thread_local]
static mut NET_DEVICE_POLLER_CID: CoroutineId = CoroutineId::from_val(65535);

pub fn iface_poll(urgent: bool) -> bool {
    // let start = get_clock();
    static THRESHOLD: usize = 10;
    static mut POLL_CNT: usize = 0;
    unsafe {
        POLL_CNT += 1;
        if urgent || POLL_CNT >= THRESHOLD {
            // let start = get_clock();
            let ans = INTERFACE.lock().poll(
                Instant::ZERO,
                &mut *NET_DEVICE.as_mut_ptr(),
                &mut SOCKET_SET.lock(),
            );
            NET_POLL_CNT += 1;
            // NET_POLL_COST += get_clock() - start;
            // debug_println!("{} {}", NET_POLL_COST, NET_POLL_CNT);
            POLL_CNT = 0;
            return ans;
        }
        return false;
    }
    // unsafe {
    //     NET_POLL_CNT += 1;
    //     NET_POLL_COST += get_clock() - start;
    //     debug_println!("{}", NET_POLL_COST);
    // }
}

static mut POLL_TIMER_CNT:usize = 0;

async fn poll_timer(mut timeout: u64) {
    static TIME_INTERVAL: u64 = 10000;
    let cid = coroutine_get_current();
    loop {
        // debug_println!("prio 2 task num: {}", get_ready_num());
        let cur = get_clock();
        if cur > timeout {
            // debug_println!("timer timeout");
            iface_poll(false);
            timeout = cur + TIME_INTERVAL;
        }
        coroutine_wake(&cid);
        yield_now().await;
    }
}

static mut NET_POLL_CNT: usize = 0;
static mut NET_POLL_COST: u64 = 0;
async fn net_poll(handler: LocalCPtr<IRQHandler>) {
    // debug_println!("net poll cid: {:?}", coroutine_get_current());
    loop {
        // debug_println!("hello net poll");
        // let start = get_clock();
        // while iface_poll(true) {};
        iface_poll(true);
        interrupt_handler();
        handler.irq_handler_ack();
        // debug_println!("poll end");
        unsafe {
            // NET_POLL_COST += get_clock() - start;
            // NET_POLL_CNT += 1;
            // debug_println!("iface_poll cost: {}", NET_POLL_COST);
        }

        // for (handler, socket) in SOCKET_SET.lock().iter() {
        //     debug_println!("get socket, handle: {}, socket: {:?}", handler, socket);
        // }
        let _ = yield_now().await;
    }
}


pub async fn nw_recv_req_coroutine(arg: usize) {
    debug_println!("hello recv_req_coroutine");
    static mut REQ_NUM: usize = 0;
    let async_args= AsyncArgs::from_ptr(arg);
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    let mut cnt = 0;
    loop {
        if let Some(item) = new_buffer.req_items.get_first_item() {
            cnt += 1;
            if let Some(item) = process_req(&item, arg).await {
                new_buffer.res_items.write_free_item(&item).unwrap();
                if new_buffer.recv_reply_status.load(SeqCst) == false {
                    new_buffer.recv_reply_status.store(true, SeqCst);
                    unsafe { uipi_send(async_args.server_sender_id.unwrap() as u64); }
                }
            }
            if cnt >= 20 {
                possible_switch().await;
                cnt = 0;
            }
        } else {
            new_buffer.recv_req_status.store(false, SeqCst);
            cnt = 0;
            yield_now().await;
            // debug_println!("nw recv cnt: {}", cnt);
        }
    }
}

async fn process_req(item: &IPCItem, arg: usize) -> Option<IPCItem> {
    let async_args= AsyncArgs::from_ptr(arg);
    match MessageDecoder::get_type(&item) {
        MessageType::NetPollReq => {
            wake_net_device_poller();
            return None;
        }
        MessageType::Listen => {
            let port = MessageDecoder::get_port(&item);
            coroutine_spawn_with_prio(Box::pin(tcp_accept_coroutine(item.cid, port as u16, async_args)), 2);
        }
        MessageType::Send => {
            let cid = MessageDecoder::get_cid(&item);
            let handler = MessageDecoder::get_socket_handler(&item);
            let tcp_buffer = MessageDecoder::get_buffer(&item);
            let len = MessageDecoder::get_len(&item);
            // let start = get_clock();
            // iface_poll();
            // debug_println!("empty poll cost: {}", get_clock() - start);
            let mut bindings = SOCKET_SET.lock();
            let socket: &mut Socket = bindings.get_mut(handler);
            if socket.can_send() {
                let send_data = &mut tcp_buffer.data[0..len];
                if let Ok(send_size) = socket.send_slice(&send_data) {
                    drop(bindings);
                    let reply = MessageBuilder::send_reply(cid, send_size);
                    iface_poll(true);
                    return Some(reply);
                }
            } else {
                // 假设所有数据大小都不大于socket buffer
                // panic!("fail to send");
                drop(bindings);
            }
        }
        MessageType::Recv => {
            let cid = MessageDecoder::get_cid(&item);
            let handler: SocketHandle = MessageDecoder::get_socket_handler(&item);
            let tcp_buffer = MessageDecoder::get_buffer(&item);
            let len = MessageDecoder::get_len(&item);
            let min_len = min(tcp_buffer.data.len(), len);
            let mut bindings = SOCKET_SET.lock();
            let socket: &mut Socket = bindings.get_mut(handler);
            if socket.can_recv() {
                if let Ok(read_size) = socket.recv_slice(&mut tcp_buffer.data[..min_len]) {
                    drop(bindings);
                    let reply = MessageBuilder::recv_reply(cid, read_size);
                    return Some(reply);
                }
            } else {
                drop(bindings);
                // coroutine_spawn_with_prio(Box::pin(tcp_recv_coroutine2(cid, handler, tcp_buffer, async_args)), 1);
                wake_with_value(SOCKET_2_CID.lock().get(&handler).unwrap(), item);
            }
        }
        _ => {
            panic!("wrong Request format")
        }
    }
    None
}


async fn tcp_recv_coroutine(mut item: Option<IPCItem>, async_args: &mut AsyncArgs) {
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    loop {
        // debug_println!("tcp_recv_coroutine");
        if item.is_none() {
            // debug_println!("tcp_recv_coroutine yield");
            item = yield_now().await;
            continue;
        }
        let item_inner = item.take().unwrap();
        let cid = MessageDecoder::get_cid(&item_inner);
        let handler: SocketHandle = MessageDecoder::get_socket_handler(&item_inner);
        let tcp_buffer = MessageDecoder::get_buffer(&item_inner);
        let len = MessageDecoder::get_len(&item_inner);
        let min_len = min(tcp_buffer.data.len(), len);
        loop {
            let mut bindings = SOCKET_SET.lock();
            let socket: &mut Socket = bindings.get_mut(handler);
            if socket.can_recv() {
                if let Ok(read_size) = socket.recv_slice(&mut tcp_buffer.data[..min_len]) {
                    drop(bindings);
                    let reply = MessageBuilder::recv_reply(cid, read_size);
                    new_buffer.res_items.write_free_item(&reply).unwrap();
                    if new_buffer.recv_reply_status.load(SeqCst) == false {
                        new_buffer.recv_reply_status.store(true, SeqCst);
                        unsafe { uipi_send(async_args.server_sender_id.unwrap() as u64); }
                    }
                }
                break;
            } else {
                drop(bindings);
                // todo: need to wakeup co in poll handler
                coroutine_wake(&coroutine_get_current());
                yield_now().await;
            }
        }
    }
}

async fn tcp_accept_coroutine(cid: CoroutineId, port: u16, async_args: &mut AsyncArgs) {
    // debug_println!("start accept_coroutine");
    let tcp_rx_buffer = SocketBuffer::new(vec![0; TCP_RX_BUF_LEN]);
    let tcp_tx_buffer = SocketBuffer::new(vec![0; TCP_TX_BUF_LEN]);
    let mut tcp_socket = Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    tcp_socket.set_ack_delay(None);
    tcp_socket.set_nagle_enabled(false);
    // debug_println!("port: {}", port);

    tcp_socket.listen(port).unwrap();
    let mut endpoint: IpListenEndpoint = IpListenEndpoint::default();
    endpoint.port = port;
    let handler = SOCKET_SET.lock().add(tcp_socket);
    debug_println!("start listen");
    unsafe {
        LISTEN_TABLE.listen(endpoint, handler, coroutine_get_current()).unwrap();
        yield_now().await;
    }
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    if let Ok((handle, (_local_ep, remote_ep))) = unsafe { LISTEN_TABLE.accept(port) } {
        let reply = MessageBuilder::listen_reply(cid, handle);
        new_buffer.res_items.write_free_item(&reply).unwrap();
        if new_buffer.recv_reply_status.load(SeqCst) == false {
            new_buffer.recv_reply_status.store(true, SeqCst);
            unsafe { uipi_send(async_args.server_sender_id.unwrap() as u64); }
        }
        SOCKET_2_CID.lock().insert(handler, coroutine_get_current());        
        // ADDR_2_CID.lock().insert(remote_ep, coroutine_get_current());
        // debug_println!("accept_addr: {:?}", ip_addr);
    } else {
        panic!("wake failed")
    }
    tcp_recv_coroutine(None, async_args).await;

}