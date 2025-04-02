use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::{format, string::String};
use async_runtime::{
    coroutine_delay_wake, coroutine_get_current, coroutine_is_empty, coroutine_run_until_blocked,
    coroutine_run_until_complete, coroutine_spawn, coroutine_spawn_with_prio, get_executor_ptr,
    runtime_init, CoroutineId, Executor, IPCItem, NewBuffer,
};
use core::alloc::Layout;
use core::mem::{self, size_of};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::SeqCst;
use riscv::register::utvec;
use sel4::cap_type::{Endpoint, TCB};
use sel4::get_clock;
use sel4::r#yield;
use sel4::{IPCBuffer, LocalCPtr, MessageInfo};
use sel4_root_task::debug_println;
use spin::Mutex;
// use uintr::{register_receiver, register_sender, uipi_send};
use crate::async_lib::{
    recv_reply_coroutine, register_recv_cid, register_sender_buffer, register_sender_buffer2, seL4_Call, sel4_call_with_item, uintr_handler, wake_recv_coroutine, yield_now, AsyncArgs, SenderID, TEST_CLOCK, TEST_TAIC_SEND_SIGNAL, UINT_TRIGGER
};
use crate::device::taic::interface::{
    alloc_receiver, alloc_vec, free_vec, register_receiver, register_sender,
    register_usoft_handler, send_signal,
};
use crate::matrix::matrix_test;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;

static SEND_NUM: usize = 4096;
static COROUTINE_NUM: usize = 16;
static mut MUTE_SEND_NUM: usize = SEND_NUM;
const MATRIX_SIZE: usize = 4;
static mut SUCESS_NUM: usize = 0;

pub fn mutex_print(s: String) {
    static PRINT_LOCK: Mutex<()> = Mutex::new(());
    let _lock = PRINT_LOCK.lock();
    debug_println!("{}", s);
}

///客户端子线程 (发送ipc)
pub fn async_helper_thread(arg: usize, ipc_buffer_addr: usize) {
    //注册 ipc_buffer
    let ipc_buffer = ipc_buffer_addr as *mut sel4::sys::seL4_IPCBuffer;
    let ipcbuf = unsafe { IPCBuffer::from_ptr(ipc_buffer) };
    sel4::set_ipc_buffer(ipcbuf);
    //子线程中的运行时初始化
    runtime_init();
    debug_println!("async_helper_thread start2");
    let async_args = AsyncArgs::from_ptr(arg);
    loop {
        let _lock = async_args.lock.lock();
        if async_args.child_tcb.is_some()
            && async_args.server_process_id.is_some()
            && async_args.ipc_new_buffer.is_some()
        {
            break;
        }
        drop(_lock);
        r#yield();
    }

    debug_println!("[client] exec_ptr: {:#x}", get_executor_ptr());
    let tcb = LocalCPtr::<TCB>::from_bits(async_args.child_tcb.unwrap());
    let reply_ntfn = GLOBAL_OBJ_ALLOCATOR.lock().alloc_ntfn().unwrap();
    tcb.tcb_bind_notification(reply_ntfn).unwrap();
    let client_process_id = alloc_receiver(tcb, reply_ntfn, 0).unwrap();
    let server_process_id = async_args.server_process_id.unwrap();

    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    let cid = Box::new(coroutine_spawn_with_prio(
        Box::pin(recv_reply_coroutine(arg, SEND_NUM)),
        0,
    ));

    register_sender_buffer2(server_process_id, new_buffer);

    register_receiver(server_process_id, 0, cid.0 as usize, true, true);

    register_sender(server_process_id, 0);

    let _lock = async_args.lock.lock();
    async_args.client_process_id = Some(client_process_id);
    async_args.reply_ntfn = Some(reply_ntfn.bits());
    drop(_lock);

    loop {
        let _lock = async_args.lock.lock();
        if async_args.server_ready {
            break;
        }
        drop(_lock);
        r#yield();
    }
    let base = 100;
    for i in 0..COROUTINE_NUM {
        coroutine_spawn(Box::pin(client_call_test(
            server_process_id as i64,
            (base + i) as u64,
        )));
    }

    debug_println!("test start");
    let start = get_clock();
    while !coroutine_is_empty() {
        // let start_inner = get_clock();
        coroutine_run_until_blocked();
        // debug_println!("coroutine_run_until_blocked: {}", get_clock() - start_inner);
        r#yield();
    }
    // debug_println!("cannot get here??");
    // coroutine_run_until_complete();
    let end = get_clock();
    let uintr_trigger_info = format!("client uintr trigger cnt: {}", unsafe { TEST_TAIC_SEND_SIGNAL });
    mutex_print(uintr_trigger_info);
    let async_test_res_info = format!("async client passed: cost: {}", end - start);

    mutex_print(async_test_res_info);

    tcb.tcb_suspend().unwrap();
}

pub static mut IPC_SEND_TIME: usize = 0;

async fn client_call_test(sender_id: SenderID, msg: u64) {
    unsafe {
        let cid = coroutine_get_current();
        let vec = if let Some(res) = alloc_vec() {
            register_receiver(sender_id as usize, res, cid.0 as usize, true, true);
            res
        } else {
            0
        };
        // debug_println!("[client] get vec {:?}",vec);
        for i in 0..SEND_NUM / COROUTINE_NUM{
        // while MUTE_SEND_NUM > 0 {
            // register_receiver(sender_id as usize, vec, cid.0 as usize, false, false);
            // MUTE_SEND_NUM -= 1;
            // debug_println!("[client{:?}] mute send{:?}", cid.0,MUTE_SEND_NUM);
            TEST_CLOCK.start();
            let item = IPCItem::from(vec as u32, cid, msg as u32);
            // debug_println!("get here???7");
            if let Ok(_reply) = sel4_call_with_item(&sender_id, 0, &item).await {
                // debug_println!("[client] register receiver vec:{:?} handler:{:?}",vec, cid.0);
                TEST_CLOCK.stop();
                SUCESS_NUM = SUCESS_NUM + 1;
                if SUCESS_NUM % 256 == 0 {
                    debug_println!("[client{:?}] success num:{:?}", cid.0, SUCESS_NUM);
                }
                // debug_println!("[client] async ipc success cid:{:?} vec:{:?}",cid.0 ,vec);
            } else {
                // debug_println!("[client] error cid:{:?} vec:{:?}",cid.0 ,vec);
                panic!("client test fail!")
            }
        }
        free_vec(vec);
    }
}

/// 接收请求的协程 （服务端）
async fn recv_req_coroutine(arg: usize) {
    debug_println!("hello recv_req_coroutine");
    //初始化，解构参数
    static mut REQ_NUM: usize = 0;
    let current_cid = coroutine_get_current().0 as usize;
    let async_args = AsyncArgs::from_ptr(arg);
    let client_process_id = async_args.client_process_id.unwrap() as usize;
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();

    //注册 sender
    for vec in 0..32 {
        register_sender(client_process_id, vec);
    }
    //注册receiver
    register_receiver(client_process_id, 0, current_cid, true, true);

    // server_ready 置 true
    let _lock = async_args.lock.lock();
    async_args.server_ready = true;
    drop(_lock);

    //循环处理请求
    loop {
        //缓冲区有请求
        if let Some(idx) = new_buffer.req_items.get_first_idx() {
            let req_item = new_buffer.data[idx];
            //回复item写入buffer
            new_buffer.data[idx] = req_item;
            //检测是否是0号vec,如果不是0号,直接唤醒;如果是0号,检测dispatcher的状态
            if req_item.vec != 0 {
                send_signal(client_process_id, req_item.vec as usize);
                // debug_println!("[server] wake coroutine recv:{:?}, vec:{:?}",client_process_id,item.vec);
            } else if new_buffer.recv_reply_status.load(SeqCst) == false {
                // wake dispatcher
                new_buffer.res_items.write_free_idx(idx);
                new_buffer.recv_reply_status.store(true, SeqCst);
                send_signal(client_process_id, 0);
                // debug_println!("[server] wake dispatcher recv:{:?}, vec:{:?}",client_process_id,item.vec);
            }
            unsafe {
                REQ_NUM += 1;
                if REQ_NUM == SEND_NUM {
                    break;
                }
            }
        } else {
            //如果buffer空了的话,阻塞自己，等待唤醒
            new_buffer.recv_req_status.store(false, SeqCst);
            //阻塞
            yield_now().await;
        }
    }
}

pub fn async_ipc_test(_bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    //异步运行时初始化
    runtime_init();
    crate::device::taic::taic_init(_bootinfo);
    // root task分发capability等对象的全局实例
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let mut async_args = AsyncArgs::new();
    // 注册通知
    let badged_notification = obj_allocator.lock().alloc_ntfn().unwrap();
    // 初始化服务端线程
    // 拿自己tcb的capability. 这个线程就是服务端
    let recv_tcb = sel4::BootInfo::init_thread_tcb();
    // 绑定通知。将receiver线程设置为通知的接受者。
    // 允许该线程通过接收通知来响应异步事件
    // 系统会进行权限检查
    recv_tcb.tcb_bind_notification(badged_notification)?;
    //初始化buffer,taic根据id绑定一个local queue
    //服务端线程id,这个id也是lq的id
    let server_process_id = alloc_receiver(recv_tcb, badged_notification, 0)?;

    // 把服务端线程id和buffer填进去
    let _lock = async_args.lock.lock();
    async_args.server_process_id = Some(server_process_id);
    let ipc_new_buffer = unsafe { obj_allocator.lock().alloc_new_buffer_without_free() };
    async_args.ipc_new_buffer = unsafe { Some(ipc_new_buffer) };
    drop(_lock);

    //初始化客户端的线程
    let child_tcb = Some(
        obj_allocator
            .lock()
            .create_thread(async_helper_thread, async_args.get_ptr(), 255, 0, true)?
            .cptr()
            .bits(),
    );

    // 把子线程tcb(客户端)填进去
    let _lock = async_args.lock.lock();
    async_args.child_tcb = child_tcb;
    drop(_lock);

    //子线程生成之后会注册用于回复的通知(reply_ntfn)
    //用与在ipc结束之后，接收服务端的回复
    // wait,等reply_ntfn初始化完毕
    loop {
        let _lock = async_args.lock.lock();
        if async_args.reply_ntfn.is_some() {
            break;
        }
        drop(_lock);
        r#yield();
    }
    //生成服务端的协程  cid相当于句柄
    let cid = Box::new(coroutine_spawn_with_prio(
        Box::pin(recv_req_coroutine(async_args.get_ptr())),
        1,
    ));

    // 这个时候server_ready锁没开，客户端和服务端的协程不会开始收/发
    // taic 注册接收方和接收方
    // 拿客户端的线程id
    // let client_process_id = async_args.client_process_id.unwrap();

    //把server_ready 置 true
    // coroutine_run_until_complete();

    //等所有协程运行完
    while !coroutine_is_empty() {
        coroutine_run_until_blocked();
        r#yield();
    }
    unsafe {
        debug_println!("success ipc num {:?},time:{:?}", SUCESS_NUM,TEST_CLOCK.get_duration());
    }
    debug_println!("TEST_PASS");
    let uintr_trigger_info = format!("client uintr cnt: {}", unsafe { TEST_TAIC_SEND_SIGNAL });
    mutex_print(uintr_trigger_info);

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}

///同步ipc
fn sync_helper_thread(ep_bits: usize, ipc_buffer_addr: usize) {
    debug_println!("hello sync_helper_thread");
    let ipc_buffer = ipc_buffer_addr as *mut sel4::sys::seL4_IPCBuffer;
    let ipcbuf = unsafe { IPCBuffer::from_ptr(ipc_buffer) };
    sel4::set_ipc_buffer(ipcbuf);
    let ep = LocalCPtr::<Endpoint>::from_bits(ep_bits as u64);
    let msg = MessageInfo::new(1, 0, 0, 1);
    debug_println!("hello sync_helper_thread2");
    let reply = ep.call(msg);
    debug_println!("get reply: {:?}", reply);
    let base = 100;
    let mut msg_info = MessageInfo::new(0, 0, 0, 1);
    let start = get_clock();
    for i in 0..SEND_NUM {
        // let mut msg_info = MessageInfo::new(0, 0,0, 1);
        // msg_info.inner_mut().0.inner_mut()[0] = ((base + i) as u64) % 3;
        let _reply = ep.call(msg_info.clone());
        // debug_println!("get reply: {:?}", reply);
    }
    let end = get_clock();
    debug_println!("sync client passed: {}", end - start);
    loop {}
}

pub fn sync_ipc_test(_bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let endpoint = obj_allocator.lock().alloc_ep()?;
    let _ = obj_allocator.lock().create_thread(
        sync_helper_thread,
        endpoint.bits() as usize,
        255,
        0,
        true,
    )?;
    // let reply_msg = MessageInfo::new(2, 0, 0, 1);
    let (recv, sender) = endpoint.recv(());
    debug_println!("recv : {:?}, sender: {}", recv, sender);
    let mut recv = MessageInfo::new(0, 0, 0, 0);
    loop {
        let (new_recv, _) = endpoint.reply_recv(recv.clone(), ());
        // matrix_test::<MATRIX_SIZE>();
        recv = new_recv;
    }
    // sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    // unreachable!()
}
