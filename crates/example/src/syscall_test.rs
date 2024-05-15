use alloc::boxed::Box;
use alloc::sync::Arc;
use async_runtime::{coroutine_run_until_complete, runtime_init};
use spin::Mutex;
use core::alloc::Layout;
use core::mem::size_of;
use alloc::alloc::alloc_zeroed;
use async_runtime::{coroutine_run_until_blocked, coroutine_spawn, NewBuffer};
use sel4::{CapRights, LocalCPtr, ObjectBlueprint, TCB};
use sel4::{CPtr, Notification};
use sel4_root_task::debug_println;
use crate::async_lib::AsyncArgs;
use crate::async_lib::recv_reply_coroutine_async_syscall;

use crate::async_lib::{recv_reply_coroutine, register_async_syscall_buffer, register_recv_cid, uintr_handler};
use crate::image_utils::UserImageUtils;
use crate::object_allocator::{self, ObjectAllocator, GLOBAL_OBJ_ALLOCATOR};
use uintr::register_receiver;
use super::async_syscall::*;
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
    
    // 选择测试用例
    // test_async_tcb_unbind_notification(obj_allocator);
    test_async_tcb_bind_notification(obj_allocator);
    // 选择测试用例
    coroutine_run_until_complete();

    debug_println!("TEST PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}

fn test_async_putchar() {
    debug_println!("Begin Async TCB Bind Notification Syscall Test");
    coroutine_spawn(Box::pin(
        syscall_putchar('X' as u16)
    ));
}

fn test_async_putstring() {
    debug_println!("Begin Async TCB Bind Notification Syscall Test");
    coroutine_spawn(Box::pin(
        syscall_putstring(&test_data)
    ));
}

fn test_async_riscvpage_get_address(vaddr: usize) {
    debug_println!("Begin Async TCB Bind Notification Syscall Test");
    coroutine_spawn(Box::pin(
        syscall_riscvpage_get_address(vaddr)
    ));
}

fn test_async_tcb_bind_notification(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("Begin Async TCB Bind Notification Syscall Test");
    // 生成tcb
    let mut async_args = AsyncArgs::new();
    let target_tcb_bits = obj_allocator.lock().create_thread(test_helper_thread, async_args.get_ptr(), 255, 1, true).unwrap().cptr().bits();
    let target_tcb: TCB = LocalCPtr::from_bits(target_tcb_bits);
    // 生成Notification
    let notification = obj_allocator.lock().alloc_ntfn().unwrap();
    coroutine_spawn(Box::pin(
        syscall_tcb_bind_notification(target_tcb, notification)
    ));
}

fn test_async_tcb_unbind_notification(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("Begin Async TCB Unbind Notification Syscall Test");
    // 生成tcb
    let mut async_args = AsyncArgs::new();
    let target_tcb_bits = obj_allocator.lock().create_thread(test_helper_thread, async_args.get_ptr(), 255, 1, true).unwrap().cptr().bits();
    let target_tcb: TCB = LocalCPtr::from_bits(target_tcb_bits);
    // 绑定Notification
    let notification = obj_allocator.lock().alloc_ntfn().unwrap();
    target_tcb.tcb_bind_notification(notification);
    // 解绑Notification
    coroutine_spawn(Box::pin(
        syscall_tcb_unbind_notification(target_tcb)
    ));
}

fn test_helper_thread(arg: usize, ipc_buffer_addr: usize) {
    loop {

    }
}


static test_data: [u16; 20] = ['1' as u16; 20];

