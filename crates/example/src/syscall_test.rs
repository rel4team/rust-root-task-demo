// use alloc::boxed::Box;
// use async_runtime::{coroutine_run_until_blocked, coroutine_spawn, NewBuffer};
// use sel4::ObjectBlueprint;
// use sel4::CPtr;
// use sel4_root_task::debug_println;
// use crate::async_lib::{AsyncArgs, recv_reply_coroutine, register_async_syscall_buffer, register_recv_cid, register_sender_buffer, seL4_Untyped_Retype};
// use crate::image_utils::get_user_image_frame_slot;
// use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
//
// static mut NEW_BUFFER: NewBuffer = NewBuffer::new();
//
// pub fn async_syscall_test(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
//     let obj_allocator = unsafe {
//         &GLOBAL_OBJ_ALLOCATOR
//     };
//     let mut async_args = AsyncArgs::new();
//     async_args.ipc_new_buffer = unsafe { Some(&mut NEW_BUFFER) };
//     let unbadged_reply_ntfn = obj_allocator.lock().alloc_ntfn().unwrap();
//     let badged_reply_ntfn = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
//         obj_allocator.lock().get_empty_slot(),
//     );
//
//     let cid = coroutine_spawn(Box::pin(recv_reply_coroutine(async_args.get_ptr(), 1)));
//     let badge = register_recv_cid(&cid).unwrap() as u64;
//     let cnode = sel4::BootInfo::init_thread_cnode();
//     cnode.relative(badged_reply_ntfn).mint(
//         &cnode.relative(unbadged_reply_ntfn),
//         sel4::CapRights::write_only(),
//         badge,
//     ).unwrap();
//     let new_buffer_ptr = unsafe { &NEW_BUFFER as *const NewBuffer as usize };
//     register_async_syscall_buffer(unsafe { &mut NEW_BUFFER });
//     let new_buffer_cap = CPtr::from_bits(get_user_image_frame_slot(bootinfo, new_buffer_ptr) as u64);
//     debug_println!("new_buffer_cap: {}, new_buffer_ptr: {:#x}", new_buffer_cap.bits(), new_buffer_ptr);
//     badged_reply_ntfn.register_async_syscall(new_buffer_cap)?;
//     let blueprint = sel4::ObjectBlueprint::Notification;
//     coroutine_spawn(Box::pin(
//         syscall_test(CPtr::from_bits(0), blueprint, 0, CPtr::from_bits(0), 0, 0, 0, 0)
//     ));
//
//     coroutine_run_until_blocked();
//
//     debug_println!("TEST_PASS");
//
//     sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
//     unreachable!()
// }
//
// async fn syscall_test(service: CPtr,
//                       r#type: ObjectBlueprint,
//                       size_bits: usize,
//                       root: CPtr,
//                       node_index: usize,
//                       node_depth: usize,
//                       node_offset: usize,
//                       num_objects: usize
// ) {
//     seL4_Untyped_Retype(service, r#type, size_bits, root, node_index, node_depth, node_offset, num_objects).await;
// }