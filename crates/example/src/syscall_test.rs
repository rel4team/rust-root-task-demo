use alloc::boxed::Box;
use alloc::sync::Arc;
use async_runtime::runtime_init;
use core::alloc::Layout;
use core::mem::size_of;
use alloc::alloc::alloc_zeroed;
use async_runtime::{coroutine_run_until_blocked, coroutine_spawn, NewBuffer};
use sel4::ObjectBlueprint;
use sel4::CPtr;
use sel4_root_task::debug_println;
use crate::async_lib::recv_reply_coroutine_async_syscall;
use crate::async_lib::{recv_reply_coroutine, register_async_syscall_buffer, register_recv_cid, register_sender_buffer, seL4_Untyped_Retype, uintr_handler};
use crate::image_utils::UserImageUtils;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
use uintr::{register_receiver};
//static mut NEW_BUFFER: NewBuffer = NewBuffer::new();

pub fn async_syscall_test(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    debug_println!("Enter Async Syscall Test");
    runtime_init();
    let new_buffer_layout = Layout::from_size_align(size_of::<NewBuffer>(), 4096).expect("Failed to create layout for page aligned memory allocation");
    let new_buffer_ref = unsafe {
        let ptr = alloc_zeroed(new_buffer_layout);
        if ptr.is_null() {
            panic!("Failed to allocate page aligned memory");
        }
        &mut *(ptr as *mut NewBuffer)
    };
    let new_buffer_ptr = new_buffer_ref as *const NewBuffer as usize;
    debug_println!("async_syscall_test: new_buffer_ptr vaddr: {:#x}", new_buffer_ptr);
    debug_println!("async_syscall_test: new_buffer_ptr paddr: {:#x}", UserImageUtils.get_user_image_frame_paddr(new_buffer_ptr));
    let obj_allocator = unsafe {
        &GLOBAL_OBJ_ALLOCATOR
    };
    let unbadged_reply_ntfn = obj_allocator.lock().alloc_ntfn().unwrap();
    let badged_reply_ntfn = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        obj_allocator.lock().get_empty_slot(),
    );
    debug_println!("async_syscall_test: spawn recv_reply_coroutine");
    let cid = coroutine_spawn(Box::pin(recv_reply_coroutine_async_syscall(new_buffer_ptr, 1)));
    debug_println!("async_syscall_test: cid: {:?}", cid);
    let badge = register_recv_cid(&cid).unwrap() as u64;
    let cnode = sel4::BootInfo::init_thread_cnode();
    cnode.relative(badged_reply_ntfn).mint(
        &cnode.relative(unbadged_reply_ntfn),
        sel4::CapRights::write_only(),
        badge,
    ).unwrap();

    let recv_tcb = sel4::BootInfo::init_thread_tcb();
    recv_tcb.tcb_bind_notification(unbadged_reply_ntfn)?;
    register_receiver(recv_tcb, unbadged_reply_ntfn, uintr_handler as usize)?;

    register_async_syscall_buffer(new_buffer_ptr);
    let new_buffer_cap = CPtr::from_bits(UserImageUtils.get_user_image_frame_slot(new_buffer_ptr) as u64);
    debug_println!("async_syscall_test: new_buffer_cap: {}, new_buffer_ptr: {:#x}", new_buffer_cap.bits(), new_buffer_ptr);
    badged_reply_ntfn.register_async_syscall(new_buffer_cap)?;
    let blueprint = sel4::ObjectBlueprint::Notification;
    coroutine_spawn(Box::pin(
        syscall_test(CPtr::from_bits(0), blueprint, 0, CPtr::from_bits(0), 0, 0, 0, 0)
    ));

    coroutine_run_until_blocked();

    debug_println!("TEST PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}

async fn syscall_test(service: CPtr,
                      r#type: ObjectBlueprint,
                      size_bits: usize,
                      root: CPtr,
                      node_index: usize,
                      node_depth: usize,
                      node_offset: usize,
                      num_objects: usize
) {
    seL4_Untyped_Retype(service, r#type, size_bits, root, node_index, node_depth, node_offset, num_objects).await;
}