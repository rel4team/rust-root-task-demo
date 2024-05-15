use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::{format, string::String};
use spin::Mutex;
use core::alloc::Layout;
use core::mem::{self, size_of};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::SeqCst;
use async_runtime::{coroutine_get_current, coroutine_is_empty, coroutine_run_until_blocked, coroutine_run_until_complete, coroutine_spawn, coroutine_spawn_with_prio, get_executor_ptr, runtime_init, Executor, IPCItem, NewBuffer};
use sel4::{IPCBuffer, LocalCPtr, MessageInfo};
use sel4::cap_type::{Endpoint, TCB};
use sel4_root_task::debug_println;
use sel4::get_clock;
use sel4::r#yield;
use uintr::{register_receiver, register_sender, uipi_send};
use crate::async_lib::{recv_reply_coroutine, register_recv_cid, register_sender_buffer, seL4_Call, seL4_Call_with_item, uintr_handler, yield_now, AsyncArgs, SenderID, UINT_TRIGGER};
use crate::matrix::matrix_test;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;

static SEND_NUM: usize = 20480;
static mut MUTE_SEND_NUM: usize = SEND_NUM;
static COROUTINE_NUM: usize = 1;
const MATRIX_SIZE: usize = 1;

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
    let async_args = AsyncArgs::from_ptr(arg);
    while async_args.child_tcb.is_none() || async_args.req_ntfn.is_none() || async_args.ipc_new_buffer.is_none() {}
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    debug_println!("[client] exec_ptr: {:#x}", get_executor_ptr());
    let cid = coroutine_spawn_with_prio(Box::pin(recv_reply_coroutine(arg, SEND_NUM)), 0);

    debug_println!("[client] cid: {:?}, exec_ptr: {:#x}", cid, get_executor_ptr());
    let badge = register_recv_cid(&cid).unwrap() as u64;
    debug_println!("client: badge: {}", badge);
    let tcb = LocalCPtr::<TCB>::from_bits(async_args.child_tcb.unwrap());
    let reply_ntfn = GLOBAL_OBJ_ALLOCATOR.lock().alloc_ntfn().unwrap();
    let badged_reply_notification = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        GLOBAL_OBJ_ALLOCATOR.lock().get_empty_slot(),
    );

    let cnode = sel4::BootInfo::init_thread_cnode();
    cnode.relative(badged_reply_notification).mint(
        &cnode.relative(reply_ntfn),
        sel4::CapRights::write_only(),
        badge,
    ).unwrap();

    tcb.tcb_bind_notification(reply_ntfn).unwrap();
    register_receiver(tcb, reply_ntfn, uintr_handler as usize).unwrap();

    let res_sender_id = register_sender_buffer(LocalCPtr::from_bits(async_args.req_ntfn.unwrap()), new_buffer);
    if res_sender_id.is_err() {
        panic!("fail to register_sender")
    }

    let sender_id = res_sender_id.unwrap();
    async_args.client_sender_id = Some(sender_id);
    async_args.reply_ntfn = Some(badged_reply_notification.bits());

    while !async_args.server_ready {}
    let base = 100;
    for i in 0..COROUTINE_NUM {
        coroutine_spawn(Box::pin(client_call_test(sender_id, (base + i) as u64)));
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
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    loop {
        if let Some(mut item) = new_buffer.req_items.get_first_item() {
            // item.msg_info += 1;
            // debug_println!("hello get item");
            matrix_test::<MATRIX_SIZE>();
            new_buffer.res_items.write_free_item(&item).unwrap();
            if new_buffer.recv_reply_status.load(SeqCst) == false {
                new_buffer.recv_reply_status.store(true, SeqCst);
                unsafe {
                    uipi_send(async_args.server_sender_id.unwrap() as u64);
                }
            }
            unsafe {
                REQ_NUM += 1;
                if REQ_NUM == SEND_NUM {
                    break;
                }
            }
            
        } else {
            new_buffer.recv_req_status.store(false, SeqCst);
            yield_now().await;
        }
    }
}


pub fn async_ipc_test(_bootinfo: &sel4::BootInfo) -> sel4::Result<!>  {
    runtime_init();
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    debug_println!("exec size: {}", core::mem::size_of::<Executor>());
    let mut async_args = AsyncArgs::new();
    let unbadged_notification = obj_allocator.lock().alloc_ntfn().unwrap();
    let badged_notification = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        obj_allocator.lock().get_empty_slot(),
    );

    let cid = coroutine_spawn_with_prio(Box::pin(recv_req_coroutine(async_args.get_ptr())), 1);
    debug_println!("[server] cid: {:?}, exec_ptr: {:#x}", cid, get_executor_ptr());
    let badge = register_recv_cid(&cid).unwrap() as u64;
    let cnode = sel4::BootInfo::init_thread_cnode();
    cnode.relative(badged_notification).mint(
        &cnode.relative(unbadged_notification),
        sel4::CapRights::write_only(),
        badge,
    )?;

    let recv_tcb = sel4::BootInfo::init_thread_tcb();
    recv_tcb.tcb_bind_notification(unbadged_notification)?;
    register_receiver(recv_tcb, unbadged_notification, uintr_handler as usize)?;


    async_args.req_ntfn = Some(badged_notification.cptr().bits());
    // debug_println!("NEW BUFFER ptr: {:#x}", unsafe { NEW_BUFFER.as_mut_ptr() as usize});
    let new_buffer_layout = Layout::from_size_align(size_of::<NewBuffer>(), 4096).expect("Failed to create layout for page aligned memory allocation");
    let ipc_new_buffer = unsafe {
        let ptr = alloc_zeroed(new_buffer_layout);
        if ptr.is_null() {
            panic!("Failed to allocate page aligned memory");
        }
        &mut *(ptr as *mut NewBuffer)
    };
    async_args.ipc_new_buffer = unsafe { Some(ipc_new_buffer) };
    async_args.child_tcb = Some(obj_allocator.lock().create_thread(async_helper_thread, async_args.get_ptr(), 255, 0, true)?.cptr().bits());
    while async_args.reply_ntfn.is_none() {}
    let res_send_reply_id = register_sender(LocalCPtr::from_bits(async_args.reply_ntfn.unwrap()));
    if res_send_reply_id.is_err() {
        panic!("fail to register_sender!")
    }
    let reply_id = res_send_reply_id.unwrap();
    async_args.server_sender_id = Some(reply_id as SenderID);
    async_args.server_ready = true;

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
        matrix_test::<MATRIX_SIZE>();
        recv = new_recv;
    }
    // sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    // unreachable!()
}