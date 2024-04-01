mod tcp;
mod message;
mod tcp_buffer;
mod lose_net_stack;
mod port_table;
mod socket;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use ::lose_net_stack::packets::tcp::TCPPacket;
use ::lose_net_stack::{MacAddress, TcpFlags};
use core::sync::atomic::Ordering::SeqCst;
use spin::{Lazy, Mutex};
use async_runtime::{coroutine_get_current, coroutine_spawn_with_prio, coroutine_wake, get_ready_num, runtime_init, CoroutineId, IPCItem};
use sel4::cap_type::Notification;
use sel4::LocalCPtr;

use sel4_root_task::debug_println;
use uintr::{register_receiver, uipi_send};
use crate::async_lib::{register_recv_cid, uintr_handler, wake_with_value, yield_now, AsyncArgs, SenderID, possible_switch};
use crate::device::{init_net_interrupt_handler, NET_DEVICE};

use sel4::get_clock;
pub use tcp::*;
pub use message::*;
pub use tcp_buffer::*;

use self::lose_net_stack::{net_interrupt_handler, LOSE_NET_STACK};
use self::port_table::listen_block;
use self::socket::{get_mutex_socket, get_s_a_by_index};

type SocketHandle = usize;

const TCP_TX_BUF_LEN: usize = 4096;
const TCP_RX_BUF_LEN: usize = 4096;

#[thread_local]
static mut NET_STACK_MAP: BTreeMap<SocketHandle, SenderID> = BTreeMap::new();

pub static SOCKET_2_CID: Lazy<Arc<Mutex<BTreeMap<SocketHandle, CoroutineId>>>> =
    Lazy::new(|| Arc::new(Mutex::new(BTreeMap::new())));


pub fn init() -> LocalCPtr<Notification> {
    runtime_init();
    let (_net_handler, net_ntfn) = init_net_interrupt_handler();
    let tcb = sel4::BootInfo::init_thread_tcb();
    tcb.tcb_bind_notification(net_ntfn).unwrap();
    register_receiver(tcb, net_ntfn, uintr_handler as usize).unwrap();

    let cid = coroutine_spawn_with_prio(Box::pin(net_poll()), 0);
    // let _ = coroutine_spawn_with_prio(Box::pin(poll_timer(get_clock())), 2);
    // debug_println!("init cid: {:?}", cid);
    let badge = register_recv_cid(&cid).unwrap() as u64;
    assert_eq!(badge, 0);
    return net_ntfn;
}


static mut NET_POLL_CNT: usize = 0;
static mut NET_POLL_COST: u64 = 0;
async fn net_poll() {
    // debug_println!("net poll cid: {:?}", coroutine_get_current());
    loop {
        // let start = get_clock();
        net_interrupt_handler();
        unsafe {
            // NET_POLL_COST += get_clock() - start;
            // NET_POLL_CNT += 1;
            // debug_println!("net poll: {}", NET_POLL_CNT);
            // debug_println!("iface_poll cost: {}", NET_POLL_COST);
        }
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
            if cnt >= 100 {
                possible_switch().await;
                cnt = 0;
            }
        } else {
            // debug_println!("nw recv cnt: {}", cnt);
            cnt = 0;
            new_buffer.recv_req_status.store(false, SeqCst);
            yield_now().await;
        }
    }
}

async fn process_req(item: &IPCItem, arg: usize) -> Option<IPCItem> {
    let async_args= AsyncArgs::from_ptr(arg);
    match MessageDecoder::get_type(&item) {
        MessageType::Listen => {
            let port = MessageDecoder::get_port(&item);
            coroutine_spawn_with_prio(Box::pin(tcp_accept_coroutine(item.cid, port as u16, async_args)), 2);
        }
        MessageType::Send => {
            let cid = MessageDecoder::get_cid(&item);
            let handler = MessageDecoder::get_socket_handler(&item);
            let tcp_buffer = MessageDecoder::get_buffer(&item);
            let len = MessageDecoder::get_len(&item);
            let mutex_sock = get_mutex_socket(handler).unwrap();
            let socket = mutex_sock.lock();
            let s_port = socket.lport;
            let dest_addr = socket.raddr;
            let d_port = socket.rport;
            drop(socket);
            // debug_println!("hello: {:?}, {:?}, {:?}", s_port, dest_addr, d_port);
            let net_stack = &LOSE_NET_STACK.0;
            let (seq, ack) = get_s_a_by_index(handler).map_or((0, 0), |x| x);
            let tcp_packet = TCPPacket {
                source_ip: net_stack.ip,
                source_mac: net_stack.mac,
                source_port: s_port,
                dest_ip: dest_addr,
                dest_mac: MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
                dest_port: d_port,
                data_len: len,
                seq,
                ack,
                flags: TcpFlags::A,
                win: 65535,
                urg: 0,
                data: tcp_buffer.data[..len].as_ref(),
            };
            NET_DEVICE.transmit(&tcp_packet.build_data());
            drop(net_stack);
            let reply = MessageBuilder::send_reply(cid, len);
            return Some(reply);

        }
        MessageType::Recv => {
            let cid = MessageDecoder::get_cid(&item);
            let handler: SocketHandle = MessageDecoder::get_socket_handler(&item);
            let tcp_buffer = MessageDecoder::get_buffer(&item);
            wake_with_value(SOCKET_2_CID.lock().get(&handler).unwrap(), item);
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
        // let tcp_buffer = MessageDecoder::get_buffer(&item_inner);
        loop {
            let socket = get_mutex_socket(handler).unwrap();
            let mut mutex_socket = socket.lock();
            if mutex_socket.read_ready_len > 0 {
                let min_len = mutex_socket.read_ready_len.min(TCP_TX_BUF_LEN);
                let reply = MessageBuilder::recv_reply(cid, min_len,
                    mutex_socket.buffers.as_ref().unwrap().get_ptr());
                mutex_socket.read_ready_len = 0;
                new_buffer.res_items.write_free_item(&reply).unwrap();
                if new_buffer.recv_reply_status.load(SeqCst) == false {
                    new_buffer.recv_reply_status.store(true, SeqCst);
                    unsafe { uipi_send(async_args.server_sender_id.unwrap() as u64); }
                }
                break;
            } else {
                mutex_socket.block_task.replace(coroutine_get_current());
                drop(mutex_socket);
                // coroutine_wake(&coroutine_get_current());
                yield_now().await;
            }
        }
    }
}

async fn tcp_accept_coroutine(cid: CoroutineId, port: u16, async_args: &mut AsyncArgs) {

    let current_cid = coroutine_get_current();
    listen_block(port, current_cid);

    
    let tmp_ipc_item = yield_now().await.unwrap();
    let handler = tmp_ipc_item.extend_msg[0] as usize;
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    let reply = MessageBuilder::listen_reply(cid, handler);
    new_buffer.res_items.write_free_item(&reply).unwrap();
    if new_buffer.recv_reply_status.load(SeqCst) == false {
        new_buffer.recv_reply_status.store(true, SeqCst);
        unsafe { uipi_send(async_args.server_sender_id.unwrap() as u64); }
    }
    SOCKET_2_CID.lock().insert(handler, coroutine_get_current());    
    
    tcp_recv_coroutine(None, async_args).await;

}