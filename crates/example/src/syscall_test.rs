use alloc::boxed::Box;
use alloc::sync::Arc;
use async_runtime::{coroutine_run_until_complete, coroutine_spawn_with_prio, runtime_init};
use sel4::cap_type::_4KPage;
use spin::Mutex;
use core::alloc::{Layout};
use core::mem::size_of;
use alloc::alloc::alloc_zeroed;
use async_runtime::{coroutine_run_until_blocked, coroutine_spawn, NewBuffer};
use sel4::{get_clock, CNode, CapRights, LocalCPtr, ObjectBlueprint, ObjectBlueprintArch, VMAttributes, TCB};
use sel4::{CPtr, Notification};
use sel4_root_task::debug_println;
use crate::async_lib::{AsyncArgs, SUBMIT_SYSCALL_CNT, UINT_TRIGGER};
use crate::async_lib::recv_reply_coroutine_async_syscall;

use crate::async_lib::{recv_reply_coroutine, register_async_syscall_buffer, register_recv_cid, uintr_handler};
use crate::image_utils::UserImageUtils;
use crate::memory_allocator::{self, AsyncMemoryAllocator, SyncMemoryAllocator};
use crate::object_allocator::{self, ObjectAllocator, GLOBAL_OBJ_ALLOCATOR};
use uintr::register_receiver;
use super::async_syscall::*;
//static mut NEW_BUFFER: NewBuffer = NewBuffer::new();

const REPLY_NUM: usize = TEST_REPLY_NUM;
const OUTPUT_REPLY_NUM: usize = 3;
const NTFN_REPLY_NUM: usize = 3;
const MAP_REPLY_NUM: usize = 4;
const UNMAP_REPLY_NUM: usize = 1;
const TEST_REPLY_NUM: usize = 2 * MAX_PAGE_NUM * EPOCH;

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
    // debug_println!("async_syscall_test: new_buffer_ptr vaddr: {:#x}", new_buffer_ptr);
    // debug_println!("async_syscall_test: new_buffer_ptr paddr: {:#x}", UserImageUtils.get_user_image_frame_paddr(new_buffer_ptr));
    let obj_allocator = unsafe {
        &GLOBAL_OBJ_ALLOCATOR
    };
    let unbadged_reply_ntfn = obj_allocator.lock().alloc_ntfn().unwrap();
    let badged_reply_ntfn = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        obj_allocator.lock().get_empty_slot(),
    );
    debug_println!("async_syscall_test: spawn recv_reply_coroutine");
    let cid = coroutine_spawn(Box::pin(recv_reply_coroutine_async_syscall(new_buffer_ptr, REPLY_NUM)));
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
    // debug_println!("async_syscall_test: new_buffer_cap: {}, new_buffer_ptr: {:#x}", new_buffer_cap.bits(), new_buffer_ptr);
    badged_reply_ntfn.register_async_syscall(new_buffer_cap)?;
    
    // 输出类系统调用演示
    // coroutine_spawn(Box::pin(test_async_output_section(new_buffer_ptr)));
    
    // Notification机制类类系统调用演示
    // coroutine_spawn(Box::pin(test_async_notification_section(obj_allocator)));
    
    // 内存映射类系统调用演示
    // show_error_async_riscv_page_map();
    // test_sync_riscv_page_map(obj_allocator);
    // coroutine_spawn(Box::pin(test_async_riscv_page_section(obj_allocator)));
    // 内存映射类Unmap系统调用演示
    // test_sync_riscv_page_unmap(obj_allocator);
    // coroutine_spawn(Box::pin(test_async_riscv_page_unmap(obj_allocator)));
    
    // 功能测试请将此行解除注释
    // coroutine_run_until_complete();
    
    // 传入参数表示is_sync，输入true测试同步系统调用，输入false测试异步系统调用
    // run_performance_test(false);
    run_performance_test_all();

    debug_println!("TEST PASS");
    debug_println!("Uintr: {:?}, submit syscall cnt: {:?}", unsafe {
        UINT_TRIGGER
    }, unsafe { SUBMIT_SYSCALL_CNT });
    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}


async fn test_async_output_section(vaddr: usize) {
    debug_println!("\nBegin Async PutChar Syscall Test");
    syscall_putchar('X' as u16).await;
    debug_println!("\nBegin Async PutString Syscall Test");
    syscall_putstring(&test_data).await;
    debug_println!("\nBegin Async RISCV Page Get Address Syscall Test");
    let paddr = UserImageUtils.get_user_image_frame_paddr(vaddr);
    syscall_riscv_page_get_address(vaddr).await;
    debug_println!("test_async_riscvpage_get_address: sync RISCVPageGetAddress get paddr: {:#x}", paddr);
}

fn test_helper_thread(arg: usize, ipc_buffer_addr: usize) {
    loop {

    }
}

static test_data: [u16; 5] = ['1' as u16; 5];

async fn test_async_notification_section(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("\nBegin Async Untyped to Notification Syscall Test");
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

fn test_sync_riscv_page_map(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("\nBegin Sync RISCV Page Map Test");
    let l2_pagetable = obj_allocator.lock().alloc_page_table().unwrap();
    let vspace = sel4::BootInfo::init_thread_vspace();
    let vaddr = 0x200_0000;
    l2_pagetable.page_table_map(vspace, vaddr, VMAttributes::default());
    
    let frame = obj_allocator.lock().alloc_frame().unwrap();
    frame.frame_map(vspace, vaddr, CapRights::read_write(), VMAttributes::default());
    let data = unsafe {
        &mut *(vaddr as *mut TestData)
    };
    data.change_data();
    debug_println!("test_async_riscv_page_unmap: unmap success, data value: {:?}", data.data);
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

fn test_sync_riscv_page_unmap(obj_allocator: &Mutex<ObjectAllocator>) {
    debug_println!("\nBegin Sync RISCV Page Unmap Test");
    let l2_pagetable = obj_allocator.lock().alloc_page_table().unwrap();
    let vspace = sel4::BootInfo::init_thread_vspace();
    let vaddr = 0x200_0000;
    l2_pagetable.page_table_map(vspace, vaddr, VMAttributes::default());
    
    let frame = obj_allocator.lock().alloc_frame().unwrap();
    frame.frame_map(vspace, vaddr, CapRights::read_write(), VMAttributes::default());
    frame.frame_unmap();
    let data = unsafe {
        &mut *(vaddr as *mut TestData)
    };
    data.change_data();
    debug_println!("test_async_riscv_page_unmap: unmap success, data value: {:?}", data.data);
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
    debug_println!("test_async_riscv_page_unmap: call func change_data");
    data.change_data();
    // debug_println!("test_async_riscv_page_unmap: unmap success, data value: {:?}", data.data);
}

const START_ADDR: usize = 0x200_0000;
const PAGE_SIZE: usize = 0x1000;
const MAX_PAGE_NUM_BITS: usize = 9;
const MAX_PAGE_NUM: usize = 1 << MAX_PAGE_NUM_BITS;
const EPOCH: usize = 10;

static mut FRAMES: [LocalCPtr<_4KPage>; MAX_PAGE_NUM] = [LocalCPtr::from_bits(0); MAX_PAGE_NUM];

fn run_performance_test(is_sync: bool) {
    performance_test_init();
    if is_sync {
        let start = get_clock() as usize;
        sync_memory_test();
        // sync_test_address(new_buffer_ptr);
        let end = get_clock() as usize;
        let time = end - start;
        debug_println!("\nSyncMemoryAllocator: Test Finish!\nTime Sum: {:?}, Average: {:?}", time, time / MAX_PAGE_NUM / EPOCH);
    } else {
        async_memory_test();
        let start = get_clock() as usize;
        coroutine_run_until_complete();
        let end = get_clock() as usize;
        let time = end - start;
        debug_println!("\nAsyncMemoryAllocator: Test Finish!\nTime Sum: {:?}, Average: {:?}", time, time / MAX_PAGE_NUM / EPOCH);
    }
}

fn run_performance_test_all() {
    performance_test_init();
    let start = get_clock() as usize;
    sync_memory_test();
    // sync_test_address(new_buffer_ptr);
    let end = get_clock() as usize;
    let time = end - start;
    debug_println!("\nSyncMemoryAllocator: Test Finish!\nTime Sum: {:?}, Average: {:?}", time, time / MAX_PAGE_NUM / EPOCH);
    
    async_memory_test();
    let start = get_clock() as usize;
    coroutine_run_until_complete();
    let end = get_clock() as usize;
    let time = end - start;
    debug_println!("\nAsyncMemoryAllocator: Test Finish!\nTime Sum: {:?}, Average: {:?}", time, time / MAX_PAGE_NUM / EPOCH);
}


fn performance_test_init() {
    // 初始化
    let obj_allocator = unsafe {
        &GLOBAL_OBJ_ALLOCATOR
    };
    // 分配页框
    let ans1 = obj_allocator.lock().alloc_many_frame(MAX_PAGE_NUM_BITS - 1);
    let ans2 = obj_allocator.lock().alloc_many_frame(MAX_PAGE_NUM_BITS - 1);
    for i in 0..MAX_PAGE_NUM {
        if i < MAX_PAGE_NUM / 2 {
            unsafe {
                FRAMES[i] = ans1[i];
            }
        } else {
            unsafe {
                FRAMES[i] = ans2[i - MAX_PAGE_NUM / 2 ];
            }
        }
    }
    // 申请页表
    let page_table = obj_allocator.lock().alloc_page_table().unwrap();
    let vspace = sel4::BootInfo::init_thread_vspace();
    let vaddr = 0x200_0000;
    page_table.page_table_map(vspace, vaddr, VMAttributes::default());
}

fn async_memory_test() {
    // 测试 
    let mut vaddr = START_ADDR;
    for i in 0..MAX_PAGE_NUM {
        let frame = unsafe {
            FRAMES
        }[i];
        coroutine_spawn_with_prio(Box::pin(async_memery_single_test(frame, vaddr)), 0);
        vaddr = vaddr + PAGE_SIZE;
    }
}

fn async_address_test(ptr: usize) {
    // 测试 
    for i in 0..MAX_PAGE_NUM {
        coroutine_spawn_with_prio(Box::pin(async_address_single_test(ptr)), 2);
    }
}

async fn async_memery_single_test(frame: LocalCPtr<_4KPage>, vaddr: usize) {
    let vspace = sel4::BootInfo::init_thread_vspace();
    for i in 0..EPOCH {
        syscall_riscv_page_map(
            frame.cptr(),
            vspace.cptr(),
            vaddr,
            CapRights::read_write().into_inner().0.inner()[0] as usize,
            VMAttributes::default().into_inner() as usize
        ).await;
        syscall_riscv_page_unmap(frame.cptr()).await;           
    }
}

async fn async_address_single_test(vaddr: usize) {
    for i in 0..EPOCH {
        syscall_riscv_page_get_address(vaddr).await;        
        syscall_riscv_page_get_address(vaddr).await;        
    }
}

fn sync_memory_test() {
    // 测试 
    let mut vaddr = START_ADDR;
    let vspace = sel4::BootInfo::init_thread_vspace();
    for i in 0..MAX_PAGE_NUM {
        let frame = unsafe {
            FRAMES
        }[i];
        for _ in 0..EPOCH {
            frame.frame_map(vspace, vaddr, CapRights::read_write(), VMAttributes::default());
            frame.frame_unmap();   
        }
        vaddr = vaddr + PAGE_SIZE;
    }
}

fn sync_address_test(ptr: usize) {
    for i in 0..MAX_PAGE_NUM {
        for j in 0..EPOCH {
            UserImageUtils.get_user_image_frame_paddr(ptr);
            UserImageUtils.get_user_image_frame_paddr(ptr);
        }
    }
}