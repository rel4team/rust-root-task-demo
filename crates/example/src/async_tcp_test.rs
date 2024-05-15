use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::alloc::Layout;
use core::mem::{forget, size_of};
use async_runtime::{coroutine_is_empty, coroutine_run_until_blocked, coroutine_run_until_complete, coroutine_spawn_with_prio, runtime_init, NewBuffer};
use sel4::{BootInfo, CPtr, IPCBuffer, LocalCPtr};
use sel4::cap_type::{Endpoint, Notification, TCB};
use sel4_root_task::{debug_println, debug_print};
use sel4::{get_clock, r#yield};
use uintr::{register_receiver, register_sender};
use crate::async_lib::{recv_reply_coroutine, register_recv_cid, register_sender_buffer, uintr_handler, AsyncArgs, SenderID, UINT_TRIGGER};
use crate::image_utils::UserImageUtils;
use crate::net::{listen, nw_recv_req_coroutine, recv, send, sync_listen, TcpBuffer};
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;

pub fn net_stack_test(boot_info: &BootInfo) -> sel4::Result<!> {
    crate::device::init(boot_info);
    let (ntfn, _) = crate::net::init();
    // BootInfo::init_thread_tcb().tcb_suspend()?;
    create_c_s_ipc_channel(ntfn);
    // coroutine_run_until_complete();
    while !coroutine_is_empty() {
        coroutine_run_until_blocked();
        r#yield();
    }
    unreachable!()
}


fn create_c_s_ipc_channel(ntfn: LocalCPtr<Notification>) {
    let new_buffer_layout = Layout::from_size_align(size_of::<NewBuffer>(), 4096).expect("Failed to create layout for page aligned memory allocation");
    let new_buffer_ref = unsafe {
        let ptr = alloc_zeroed(new_buffer_layout);
        if ptr.is_null() {
            panic!("Failed to allocate page aligned memory");
        }
        &mut *(ptr as *mut NewBuffer)
    };
    // let new_buffer_ref = unsafe {
    //     let new_buffer = Arc::new(NewBuffer::new());
    //     let res = &mut *(new_buffer.as_ref() as *const NewBuffer as usize as *mut NewBuffer);
    //     forget(new_buffer);
    //     res
    // };
    let new_buffer_cap = CPtr::from_bits(UserImageUtils.get_user_image_frame_slot(new_buffer_ref.get_ptr()) as u64);
    ntfn.register_async_syscall(new_buffer_cap).unwrap();
    let async_args = {
        let ref_args = Arc::new(AsyncArgs::new());
        let leaky_ref = unsafe { &mut *(ref_args.as_ref() as *const AsyncArgs as usize as *mut AsyncArgs) };
        forget(ref_args);
        leaky_ref
    };
    async_args.ipc_new_buffer = Some(new_buffer_ref);

    let badged_notification = BootInfo::init_cspace_local_cptr::<Notification>(
        GLOBAL_OBJ_ALLOCATOR.lock().get_empty_slot(),
    );

    let cid = coroutine_spawn_with_prio(Box::pin(nw_recv_req_coroutine(async_args.get_ptr())), 1);
    let badge = register_recv_cid(&cid).unwrap() as u64;
    assert_eq!(badge, 0);
    let cnode = BootInfo::init_thread_cnode();
    cnode.relative(badged_notification).mint(
        &cnode.relative(ntfn),
        sel4::CapRights::write_only(),
        badge,
    ).unwrap();
    async_args.req_ntfn = Some(badged_notification.cptr().bits());
    async_args.child_tcb = Some(GLOBAL_OBJ_ALLOCATOR.lock().create_thread(tcp_server_thread, async_args.get_ptr(), 255, 0, true).unwrap().cptr().bits());
    while async_args.reply_ntfn.is_none() {}
    let res_send_reply_id = register_sender(LocalCPtr::from_bits(async_args.reply_ntfn.unwrap()));
    if res_send_reply_id.is_err() {
        panic!("fail to register_sender!")
    }
    let reply_id = res_send_reply_id.unwrap();
    async_args.server_sender_id = Some(reply_id as SenderID);
    async_args.server_ready = true;

}

fn tcp_server_thread(arg: usize, ipc_buffer_addr: usize) {
    let ipc_buffer = ipc_buffer_addr as *mut sel4::sys::seL4_IPCBuffer;
    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer)
    };
    sel4::set_ipc_buffer(ipcbuf);
    runtime_init();
    let async_args = AsyncArgs::from_ptr(arg);
    while async_args.child_tcb.is_none() || async_args.req_ntfn.is_none() || async_args.ipc_new_buffer.is_none() {}
    let cid = coroutine_spawn_with_prio(Box::pin(recv_reply_coroutine(arg, usize::MAX)), 0);
    let badge = register_recv_cid(&cid).unwrap() as u64;
    let tcb = LocalCPtr::<TCB>::from_bits(async_args.child_tcb.unwrap());
    let reply_ntfn = GLOBAL_OBJ_ALLOCATOR.lock().alloc_ntfn().unwrap();
    let badged_reply_notification = BootInfo::init_cspace_local_cptr::<Notification>(
        GLOBAL_OBJ_ALLOCATOR.lock().get_empty_slot(),
    );

    let cnode = BootInfo::init_thread_cnode();
    cnode.relative(badged_reply_notification).mint(
        &cnode.relative(reply_ntfn),
        sel4::CapRights::write_only(),
        badge,
    ).unwrap();

    tcb.tcb_bind_notification(reply_ntfn).unwrap();
    register_receiver(tcb, reply_ntfn, uintr_handler as usize).unwrap();
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    let res_sender_id = register_sender_buffer(LocalCPtr::from_bits(async_args.req_ntfn.unwrap()), new_buffer);
    if res_sender_id.is_err() {
        panic!("fail to register_sender")
    }

    let sender_id = res_sender_id.unwrap();
    async_args.client_sender_id = Some(sender_id);
    async_args.reply_ntfn = Some(badged_reply_notification.bits());
    while !async_args.server_ready {}

    for _ in 0..32 {
        coroutine_spawn_with_prio(Box::pin(tcp_server(sender_id)), 1);
    }

    // coroutine_run_until_complete();
    while !coroutine_is_empty() {
        coroutine_run_until_blocked();
        r#yield();
    }
    debug_println!("server test end");
    loop {

    }
}


async fn tcp_server(nw_sender_id: SenderID) {
    debug_println!("tcp server start");
    let listen_fd = listen(80, &nw_sender_id).await.unwrap();
    // let socket_fd = accept(listen_fd).await.unwrap();
    // debug_println!("accept success!");
    let mut tcp_buffer = Box::new(TcpBuffer::new());
    let need_send = true;
    let need_recv = true;
    loop {
        if need_recv {
            if let Ok(recv_size) = recv(listen_fd, tcp_buffer.as_mut(), 1).await {
                // debug_println!("recv success, recv_size: {}", recv_size);
                if tcp_buffer.data[0] == '.' as u8 {
                    break;
                }
                // for i in 0..recv_size {
                //     debug_print!("{}", char::from(tcp_buffer.data[i]));
                // }
                // debug_println!("");
            } else {
                panic!("recv fail!");
            }
        }
        
        if need_send {
            // let resp_str = '!'.to_string().repeat(400);
            let resp_str = String::from("!");
            let resp = resp_str.as_bytes();
            for i in 0..resp.len() {
                tcp_buffer.data[i] = resp[i];
            }
            // let start = get_clock();
            if let Ok(_send_size) = send(listen_fd, tcp_buffer.as_ref(), resp.len()).await {
                // debug_println!("send success, send_size: {}", send_size);
            }
            // debug_println!("send cost: {}", get_clock() - start);
        }
        
    }

    let resp_str = String::from(".");
    let resp = resp_str.as_bytes();
    for i in 0..resp.len() {
        tcp_buffer.data[i] = resp[i];
    }
    send(listen_fd, tcp_buffer.as_ref(), resp.len()).await;
    // debug_println!("server cnt: {}", unsafe { UINT_TRIGGER });
    
}
