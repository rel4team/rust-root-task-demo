use alloc::boxed::Box;
use async_runtime::{coroutine_run_until_complete, coroutine_spawn, get_executor_ptr, NewBuffer};
use async_runtime::utils::yield_now;
use sel4::{IPCBuffer, LocalCPtr, MessageInfo};
use sel4::cap_type::TCB;
use sel4_root_task::debug_println;
use uintr::{register_receiver, register_sender, uipi_send};
use crate::async_lib::{AsyncArgs, recv_reply_coroutine, register_recv_cid, register_sender_buffer, seL4_Call, SenderID, uintr_handler};
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;

static SEND_NUM: usize = 64;

static mut NEW_BUFFER: NewBuffer = NewBuffer::new();

pub fn thread_helper(arg: usize) {
    let ipc_buffer = (0x200_0000 as *mut sel4::sys::seL4_IPCBuffer);
    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer)
    };
    sel4::set_ipc_buffer(ipcbuf);


    let async_args = AsyncArgs::from_ptr(arg);
    while async_args.child_tcb.is_none() || async_args.req_ntfn.is_none() || async_args.ipc_new_buffer.is_none() {}
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    let cid = coroutine_spawn(Box::pin(recv_reply_coroutine(arg)));

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
    for i in 0..SEND_NUM {
        coroutine_spawn(Box::pin(client_call_test(sender_id, (base + i) as u64)));
    }
    coroutine_run_until_complete();
    tcb.tcb_suspend().unwrap();
}

async fn client_call_test(sender_id: SenderID, msg: u64) {
    let mut msg_info = MessageInfo::new(0, 0,0, 0);
    msg_info.inner_mut().0.inner_mut()[0] = msg;
    if let Ok(reply) = seL4_Call(&sender_id, msg_info).await {
        assert_eq!(msg + 1, reply.inner().0.inner()[0]);
        debug_println!("get reply: {}, client test pass!", reply.inner().0.inner()[0]);
    } else {
        panic!("client test fail!")
    }
}


async fn recv_req_coroutine(arg: usize) {
    let async_args= AsyncArgs::from_ptr(arg);
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    loop {
        if let Some(mut item) = new_buffer.req_items.get_first_item() {
            item.msg_info += 1;
            new_buffer.res_items.write_free_item(&item).unwrap();
            if new_buffer.recv_reply_status == false {
                new_buffer.recv_reply_status = true;
                unsafe { uipi_send(async_args.server_sender_id.unwrap() as u64); }
            }
        } else {
            new_buffer.recv_req_status = false;
            yield_now().await;
        }
    }
}


pub fn async_ipc_test(_bootinfo: &sel4::BootInfo) -> sel4::Result<!>  {
    let obj_allocator = unsafe {
        &GLOBAL_OBJ_ALLOCATOR
    };
    let mut async_args = AsyncArgs::new();

    let unbadged_notification = obj_allocator.lock().alloc_ntfn().unwrap();
    let badged_notification = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        obj_allocator.lock().get_empty_slot(),
    );

    let cid = coroutine_spawn(Box::pin(recv_req_coroutine(async_args.get_ptr())));
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
    async_args.ipc_new_buffer = unsafe { Some(&mut NEW_BUFFER) };
    async_args.child_tcb = Some(obj_allocator.lock().create_thread(thread_helper, async_args.get_ptr(), 255)?.cptr().bits());

    while async_args.reply_ntfn.is_none() {}
    let res_send_reply_id = register_sender(LocalCPtr::from_bits(async_args.reply_ntfn.unwrap()));
    if res_send_reply_id.is_err() {
        panic!("fail to register_sender!")
    }
    let reply_id = res_send_reply_id.unwrap();
    async_args.server_sender_id = Some(reply_id as SenderID);
    async_args.server_ready = true;

    coroutine_run_until_complete();

    debug_println!("TEST_PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}