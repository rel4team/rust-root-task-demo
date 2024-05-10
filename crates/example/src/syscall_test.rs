use alloc::boxed::Box;
use alloc::sync::Arc;
use async_runtime::{coroutine_run_until_complete, runtime_init};
use spin::Mutex;
use core::alloc::Layout;
use core::mem::size_of;
use alloc::alloc::alloc_zeroed;
use async_runtime::{coroutine_run_until_blocked, coroutine_spawn, NewBuffer};
use sel4::{CNode, CapRights, LocalCPtr, ObjectBlueprint, ObjectBlueprintArch, VMAttributes, TCB};
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
    let new_buffer_ptr: usize = new_buffer_ref as *const NewBuffer as usize;
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
    let cid = coroutine_spawn(Box::pin(recv_reply_coroutine_async_syscall(new_buffer_ptr, 4)));
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
    
    // 选择测试用例
    // coroutine_spawn(Box::pin(test_async_output_section(new_buffer_ptr)));
    // coroutine_spawn(Box::pin(test_async_notification_section(obj_allocator)));
    // show_error_async_riscv_page_map();
    coroutine_spawn(Box::pin(test_async_riscv_page_section(obj_allocator)));
    // coroutine_spawn(Box::pin(test_async_riscv_page_unmap(obj_allocator)));
    coroutine_run_until_complete();

    debug_println!("TEST PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}


async fn test_async_output_section(vaddr: usize) {
    debug_println!("\nBegin Async PutChar Syscall Test");
    syscall_putchar('X' as u16).await;
    debug_println!("\nBegin Async PutString Syscall Test");
    // syscall_putstring(&test_data).await;
    debug_println!("\nBegin Async RISCV Page Get Address Syscall Test");
    let paddr = UserImageUtils.get_user_image_frame_paddr(vaddr);
    syscall_riscv_page_get_address(vaddr).await;
    debug_println!("test_async_riscvpage_get_address: sync RISCVPageGetAddress get paddr: {:#x}", paddr);
}

fn test_helper_thread(arg: usize, ipc_buffer_addr: usize) {
    loop {

    }
}

static test_data: [u16; 20] = ['1' as u16; 20];

async fn test_async_notification_section(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("\nBegin Async Retype Syscall Test");
    // 生成tcb
    let cnode = sel4::BootInfo::init_thread_cnode();
    let mut async_args = AsyncArgs::new();
    let target_tcb_bits = obj_allocator.lock().create_thread(test_helper_thread, async_args.get_ptr(), 255, 1, true).unwrap().cptr().bits();
    let target_tcb: TCB = LocalCPtr::from_bits(target_tcb_bits);
    // 生成Notification
    let blueprint = sel4::ObjectBlueprint::Notification;
    let untyped = obj_allocator.lock().get_the_first_untyped_slot(&blueprint);
    let slot = obj_allocator.lock().get_empty_slot();
    let dst = cnode.relative_self();
    syscall_untyped_retype(
        untyped.cptr(),
        blueprint, 
        blueprint.api_size_bits().unwrap_or(0).try_into().unwrap(), 
        dst.root().cptr(), 
        dst.path().bits() as usize, 
        dst.path().depth().try_into().unwrap(), 
        slot, 
        1).await;
    let notification  = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        slot
    );
    // 绑定Notification
    debug_println!("\nBegin Async TCB Bind Notification Syscall Test");
    syscall_tcb_bind_notification(target_tcb, notification).await;
    debug_println!("\nBegin Async TCB Unbind Notification Syscall Test");
    // 解绑Notification
    syscall_tcb_unbind_notification(target_tcb).await;  
}

struct TestData {
    data:  usize
}

impl TestData {
    pub fn change_data(&mut self) {
        self.data = 10000000;
    }
}

fn show_error_async_riscv_page_map() {
    debug_println!("\nBegin Async RISCV PageTable Map Test");
    let vaddr = 0x200_0000;
    let data = unsafe {
        &mut *(vaddr as *mut TestData)
    };
    debug_println!("Write and Read Data to show the map result:");
    debug_println!("test_async_riscv_page_map: data value: {:?}", data.data);
    debug_println!("test_async_riscv_page_map: call func change_data");
    data.change_data();
    debug_println!("test_async_riscv_page_map: data value: {:?}", data.data);
}

async fn test_async_riscv_page_section(obj_allocator: &Mutex<ObjectAllocator>) {
    let cnode = sel4::BootInfo::init_thread_cnode();
    let dst = cnode.relative_self();    
    debug_println!("\nBegin Async Untyped to PageTable Test");
    let pt_blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::PageTable);
    let pt_untyped = obj_allocator.lock().get_the_first_untyped_slot(&pt_blueprint);
    let pt_slot = obj_allocator.lock().get_empty_slot();
    syscall_untyped_retype(
        pt_untyped.cptr(),
        pt_blueprint, 
        pt_blueprint.api_size_bits().unwrap_or(0).try_into().unwrap(), 
        dst.root().cptr(), 
        dst.path().bits() as usize, 
        dst.path().depth().try_into().unwrap(), 
        pt_slot, 
        1).await;
    let page_table = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::_4KPage>(
        pt_slot
    );

    debug_println!("\nBegin Async RISCV PageTable Map Test");
    let vspace = sel4::BootInfo::init_thread_vspace();
    let vaddr = 0x200_0000;
    syscall_riscv_pagetable_map(
        page_table.cptr(),
        vspace.cptr(),
        vaddr,
        VMAttributes::default().into_inner() as usize
    ).await;

    debug_println!("\nBegin Async Untyped to Frame Test");
    let frame_blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::_4KPage);
    let frame_untyped = obj_allocator.lock().get_the_first_untyped_slot(&frame_blueprint);
    let frame_slot = obj_allocator.lock().get_empty_slot();
    syscall_untyped_retype(
        frame_untyped.cptr(),
        frame_blueprint, 
        frame_blueprint.api_size_bits().unwrap_or(0).try_into().unwrap(), 
        dst.root().cptr(), 
        dst.path().bits() as usize, 
        dst.path().depth().try_into().unwrap(), 
        frame_slot, 
        1).await;
    let frame = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::_4KPage>(
        frame_slot
    );
    debug_println!("\nBegin Async RISCV Page Map Test");
    // let frame = obj_allocator.lock().alloc_frame().unwrap();
    syscall_riscv_page_map(
        frame.cptr(), 
        vspace.cptr(), 
        vaddr, 
        CapRights::read_write().into_inner().0.inner()[0] as usize, 
        VMAttributes::default().into_inner() as usize
    ).await;

    debug_println!("\nWrite and Read Data to show the map result:");
    let data = unsafe {
        &mut *(vaddr as *mut TestData)
    };    
    debug_println!("test_async_riscv_page_map: data value: {:?}", data.data);
    debug_println!("test_async_riscv_page_map: call func change_data");
    data.change_data();
    debug_println!("test_async_riscv_page_map: data value: {:?}", data.data);

}

async fn test_async_riscv_page_unmap(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("\nBegin Async RISCV PageTable Unmap Test");
    let l2_pagetable = obj_allocator.lock().alloc_page_table().unwrap();
    let vspace = sel4::BootInfo::init_thread_vspace();
    let vaddr = 0x200_0000;
    l2_pagetable.page_table_map(vspace, vaddr, VMAttributes::default());
    
    let frame = obj_allocator.lock().alloc_frame().unwrap();
    frame.frame_map(vspace, vaddr, CapRights::read_write(), VMAttributes::default());
    
    // frame.frame_unmap();
    syscall_riscv_page_unmap(frame.cptr()).await;
    let data = unsafe {
        &mut *(vaddr as *mut TestData)
    };
    data.change_data();
    debug_println!("test_async_riscv_page_unmap: unmap success, data value: {:?}", data.data);
}

