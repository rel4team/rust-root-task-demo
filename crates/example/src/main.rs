//
// Copyright 2023, Colias Group, LLC
//
// SPDX-License-Identifier: BSD-2-Clause
//

#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(thread_local)]
#![feature(generic_const_exprs)]
extern crate alloc;
mod heap;
mod utils;
mod object_allocator;

use sel4::{CPtr, IPCBuffer, LocalCPtr, with_ipc_buffer};

use sel4_logging::{LevelFilter, Logger, LoggerBuilder};
use sel4_root_task::{debug_print, debug_println};
use sel4_root_task::root_task;
use crate::heap::get_heap;
use uintr::*;
use crate::object_allocator::ObjectAllocator;

const LOG_LEVEL: LevelFilter = LevelFilter::Trace;

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .write(|s| debug_print!("{}", s))
    .build();

static mut CHILD_TCB_PTR: u64 = 0;
static mut UINT_NTFN_PTR: u64 = 0;

static mut UINT_FLAG: u32 = 0;
pub fn new_thread(arg: usize) {
    debug_println!("hello new thread0: {}", arg);
    heap::init_heap();
    debug_println!("hello new thread: {}", arg);
    get_heap();
    let ipc_buffer = (0x200_0000 as *mut sel4::sys::seL4_IPCBuffer);
    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer)
    };
    sel4::set_ipc_buffer(ipcbuf);
    with_ipc_buffer(|buffer| {
        debug_println!("buffer ptr: {:#x}", buffer.ptr() as usize);
    });
    get_heap();
    unsafe {
        while CHILD_TCB_PTR == 0 || UINT_NTFN_PTR == 0 {}
        if let Ok(sender_id) = register_sender(LocalCPtr::from_bits(UINT_NTFN_PTR)) {
            debug_println!("sender_id: {}", sender_id);
            uipi_send(sender_id as usize);
            debug_println!("sender_id after: {}", sender_id);
        } else {
            debug_println!("fail to register_sender!");
        }
    }

    loop {

    }
}

pub fn uintr_handler(frame: *mut uintr_frame, irqs: usize) -> usize {
    debug_println!("Hello, uintr_handler!: {}", irqs);
    unsafe {
        UINT_FLAG = 1;
    }
    return 0;
}


#[root_task]
fn main(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    debug_println!("Hello, World!");
    // with_ipc_buffer(|buffer| {
    //     sel4::debug_println!("buffer ptr: {:#x}", buffer as *const IPCBuffer as usize);
    // });
    LOGGER.set().unwrap();

    // let blueprint = sel4::ObjectBlueprint::TCB;
    heap::init_heap();
    get_heap();
    let mut obj_allocator = ObjectAllocator::new(bootinfo);

    let unbadged_notification = obj_allocator.alloc_ntfn().unwrap();
    let badged_notification = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        obj_allocator.get_empty_slot(),
    );
    let cnode = sel4::BootInfo::init_thread_cnode();
    let badge = 0x1337;
    cnode.relative(badged_notification).mint(
        &cnode.relative(unbadged_notification),
        sel4::CapRights::write_only(),
        badge,
    )?;
    // badged_notification.signal();
    debug_println!("after signal! {:?}", unbadged_notification.cptr());
    // let (_, observed_badge) = unbadged_notification.wait();
    let recv_tcb = sel4::BootInfo::init_thread_tcb();
    recv_tcb.tcb_bind_notification(unbadged_notification)?;
    register_receiver(recv_tcb, badged_notification, uintr_handler as usize)?;
    // let child_tcb = obj_allocator.create_thread(new_thread, 126, 255)?;

    // sel4::debug_println!("after wait! observed_badge: {:?}", observed_badge);
    unsafe {
        CHILD_TCB_PTR = obj_allocator.create_thread(new_thread, 126, 255)?.cptr().bits();
        UINT_NTFN_PTR = badged_notification.cptr().bits();
    }
    // sel4::debug_println!("start wait");
    while unsafe { UINT_FLAG == 0} {};



    debug_println!("TEST_PASS");
    with_ipc_buffer(|buffer| {
        debug_println!("buffer ptr: {:#x}", buffer.ptr() as usize);
    });
    get_heap();


    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}
