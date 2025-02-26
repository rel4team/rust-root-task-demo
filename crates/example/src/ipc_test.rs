use riscv::register::utvec;
use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::{format, string::String};
use spin::Mutex;
use core::alloc::Layout;
use core::mem::{self, size_of};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::SeqCst;
use async_runtime::{coroutine_delay_wake, coroutine_get_current, coroutine_is_empty, coroutine_run_until_blocked, coroutine_run_until_complete, coroutine_spawn, coroutine_spawn_with_prio, get_executor_ptr, runtime_init, CoroutineId, Executor, IPCItem, NewBuffer};
use sel4::{IPCBuffer, LocalCPtr, MessageInfo};
use sel4::cap_type::{Endpoint, TCB};
use sel4_root_task::debug_println;
use sel4::get_clock;
use sel4::r#yield;
// use uintr::{register_receiver, register_sender, uipi_send};
use crate::device::taic::interface::{re_register, alloc_receiver, register_sender, register_usoft_handler, register_receiver};
use crate::async_lib::{recv_reply_coroutine, register_recv_cid, register_sender_buffer, register_sender_buffer2, seL4_Call, seL4_Call_with_item, uintr_handler, wake_recv_coroutine, yield_now, AsyncArgs, SenderID, UINT_TRIGGER};
use crate::matrix::matrix_test;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;

static SEND_NUM: usize = 4096;
static mut MUTE_SEND_NUM: usize = SEND_NUM;
static COROUTINE_NUM: usize = 16;
const MATRIX_SIZE: usize = 4;

pub fn mutex_print(s: String) {
    static PRINT_LOCK: Mutex<()> = Mutex::new(());
    let _lock = PRINT_LOCK.lock();
    debug_println!("{}", s);
}

pub fn async_helper_thread(arg: usize, ipc_buffer_addr: usize) {
    let ipc_buffer = ipc_buffer_addr as *mut sel4::sys::seL4_IPCBuffer;
    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer)
    };
    sel4::set_ipc_buffer(ipcbuf);
    runtime_init();
    debug_println!("async_helper_thread start2");
    let async_args = AsyncArgs::from_ptr(arg);
    while true {
        let _lock = async_args.lock.lock();
        if async_args.child_tcb.is_some() && async_args.server_process_id.is_some() && async_args.ipc_new_buffer.is_some() {
            break;
        }
        drop(_lock);
        r#yield();
    }
    // while async_args.child_tcb.is_none() || async_args.req_ntfn.is_none() || async_args.ipc_new_buffer.is_none() {
    //     // debug_println!("{} {} {}", async_args.child_tcb.is_none(), async_args.req_ntfn.is_none(), async_args.ipc_new_buffer.is_none());
    // }
    debug_println!("[client] exec_ptr: {:#x}", get_executor_ptr());
    let tcb = LocalCPtr::<TCB>::from_bits(async_args.child_tcb.unwrap());
    let reply_ntfn = GLOBAL_OBJ_ALLOCATOR.lock().alloc_ntfn().unwrap();

    tcb.tcb_bind_notification(reply_ntfn).unwrap();
    let client_process_id = alloc_receiver(tcb, reply_ntfn, 0).unwrap();
    let server_process_id = async_args.server_process_id.unwrap();

    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    let cid = Box::new(
        coroutine_spawn_with_prio(Box::pin(recv_reply_coroutine(arg, SEND_NUM)), 0)
    );

    // register_usoft_handler(Box::new(move || {
    //     coroutine_delay_wake(*cid);
    //     // re_register(server_process_id);
    // }));

    register_sender_buffer2(server_process_id, new_buffer);
    register_receiver(server_process_id, cid.0 as usize);
    register_sender(server_process_id);

    let _lock = async_args.lock.lock();
    async_args.client_process_id = Some(client_process_id);
    async_args.reply_ntfn = Some(reply_ntfn.bits());
    drop(_lock);
    while true {
        let _lock = async_args.lock.lock();
        if async_args.server_ready {
            break;
        }
        drop(_lock);
        r#yield();
    }
    let base = 100;
    for i in 0..COROUTINE_NUM {
        coroutine_spawn(Box::pin(client_call_test(server_process_id as i64, (base + i) as u64)));
    }
    
    debug_println!("test start");
    let start = get_clock();
    while !coroutine_is_empty() {
        // let start_inner = get_clock();
        coroutine_run_until_blocked();
        // debug_println!("coroutine_run_until_blocked: {}", get_clock() - start_inner);
        r#yield();
    }
    // coroutine_run_until_complete();
    let end = get_clock();
    let uintr_trigger_info = format!("client uintr trigger cnt: {}",
        unsafe { UINT_TRIGGER});
    mutex_print(uintr_trigger_info);
    let async_test_res_info = format!("async client passed: cost: {}", end - start);

    mutex_print(async_test_res_info);

    tcb.tcb_suspend().unwrap();
}


async fn client_call_test(sender_id: SenderID, msg: u64) {
    unsafe {
        while MUTE_SEND_NUM > 0 {
            MUTE_SEND_NUM -= 1;
            let item = IPCItem::from(coroutine_get_current(), msg as u32);;
            if let Ok(_reply) = seL4_Call_with_item(&sender_id, &item).await {

            } else {
                panic!("client test fail!")
            }
        }
    }
}


async fn recv_req_coroutine(arg: usize) {
    debug_println!("hello recv_req_coroutine");
    static mut REQ_NUM: usize = 0;
    let async_args= AsyncArgs::from_ptr(arg);
    let client_process_id = async_args.client_process_id.unwrap() as usize;
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    loop {
        if let Some(mut item) = new_buffer.req_items.get_first_item() {
            // item.msg_info += 1;
            // debug_println!("hello get item");
            let _res = matrix_test::<MATRIX_SIZE>();
            new_buffer.res_items.write_free_item(&item).unwrap();
            if new_buffer.recv_reply_status.load(SeqCst) == false {
                new_buffer.recv_reply_status.store(true, SeqCst);
                unsafe {
                    crate::device::taic::interface::send_signal(client_process_id);
                }
            }
            unsafe {
                REQ_NUM += 1;
                if REQ_NUM == SEND_NUM {
                    break;
                }
            }
            
        } else {
            register_receiver(client_process_id, coroutine_get_current().0 as usize);
            new_buffer.recv_req_status.store(false, SeqCst);
            yield_now().await;
        }
    }
}

pub fn async_ipc_test(_bootinfo: &sel4::BootInfo) -> sel4::Result<!>  {
    runtime_init();
    crate::device::taic::taic_init(_bootinfo);
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    debug_println!("exec size: {}", size_of::<Executor>());
    let mut async_args = AsyncArgs::new();
    let badged_notification = obj_allocator.lock().alloc_ntfn().unwrap();

    let recv_tcb = sel4::BootInfo::init_thread_tcb();
    recv_tcb.tcb_bind_notification(badged_notification)?;
    let server_process_id = alloc_receiver(recv_tcb, badged_notification, 0)?;

    let _lock = async_args.lock.lock();
    async_args.server_process_id = Some(server_process_id);
    // debug_println!("NEW BUFFER ptr: {:#x}", unsafe { NEW_BUFFER.as_mut_ptr() as usize});
    let ipc_new_buffer = unsafe {
        obj_allocator.lock().alloc_new_buffer_without_free()
    };
    async_args.ipc_new_buffer = unsafe { Some(ipc_new_buffer) };

    drop(_lock);
    let child_tcb = Some(obj_allocator.lock().create_thread(async_helper_thread, async_args.get_ptr(), 255, 0, true)?.cptr().bits());

    let _lock = async_args.lock.lock();
    async_args.child_tcb = child_tcb;
    drop(_lock);

    loop {
        let _lock = async_args.lock.lock();
        if async_args.reply_ntfn.is_some() {
            break;
        }
        drop(_lock);
        r#yield();
    }
    let cid = Box::new(coroutine_spawn_with_prio(Box::pin(recv_req_coroutine(async_args.get_ptr())), 1));
    let client_process_id = async_args.client_process_id.unwrap();
    debug_println!("[server] cid: {}", cid.0);
    register_receiver(client_process_id, cid.0 as usize);
    register_sender(client_process_id);


    // register_usoft_handler(Box::new(move || {
    //     coroutine_delay_wake(*cid);
    //     // re_register(client_process_id);
    // }));

    let _lock = async_args.lock.lock();
    async_args.server_ready = true;
    drop(_lock);
    // coroutine_run_until_complete();
    while !coroutine_is_empty() {
        coroutine_run_until_blocked();
        r#yield();
    }
    debug_println!("TEST_PASS");
    let uintr_trigger_info = format!("server uintr cnt: {}",
        unsafe { UINT_TRIGGER });
    mutex_print(uintr_trigger_info);

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}


fn sync_helper_thread(ep_bits: usize, ipc_buffer_addr: usize) {
    debug_println!("hello sync_helper_thread");
    let ipc_buffer = ipc_buffer_addr as *mut sel4::sys::seL4_IPCBuffer;
    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer)
    };
    sel4::set_ipc_buffer(ipcbuf);
    let ep = LocalCPtr::<Endpoint>::from_bits(ep_bits as u64);
    let msg = MessageInfo::new(1, 0, 0, 1);
    debug_println!("hello sync_helper_thread2");
    let reply = ep.call(msg);
    debug_println!("get reply: {:?}", reply);
    let base = 100;
    let mut msg_info = MessageInfo::new(0, 0,0, 1);
    let start = get_clock();
    for i in 0..SEND_NUM {
        // let mut msg_info = MessageInfo::new(0, 0,0, 1);
        // msg_info.inner_mut().0.inner_mut()[0] = ((base + i) as u64) % 3;
        let _reply = ep.call(msg_info.clone());
        // debug_println!("get reply: {:?}", reply);
    }
    let end = get_clock();
    debug_println!("sync client passed: {}", end - start);
    loop {

    }
}

pub fn sync_ipc_test(_bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let endpoint = obj_allocator.lock().alloc_ep()?;
    let _ = obj_allocator.lock().create_thread(sync_helper_thread, endpoint.bits() as usize, 255, 0, true)?;
    // let reply_msg = MessageInfo::new(2, 0, 0, 1);
    let (recv, sender) = endpoint.recv(());
    debug_println!("recv : {:?}, sender: {}",recv, sender);
    let mut recv = MessageInfo::new(0, 0, 0, 0);
    loop {
        let (new_recv, _) = endpoint.reply_recv(recv.clone(), ());
        // matrix_test::<MATRIX_SIZE>();
        recv = new_recv;
    }
    // sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    // unreachable!()
}