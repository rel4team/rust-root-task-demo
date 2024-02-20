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

use sel4_logging::{LevelFilter, Logger, LoggerBuilder};
use sel4_root_task::{debug_print, debug_println};
use sel4_root_task::root_task;
use crate::heap::get_heap;
use crate::object_allocator::ObjectAllocator;

const LOG_LEVEL: LevelFilter = LevelFilter::Trace;

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .write(|s| debug_print!("{}", s))
    .build();


pub fn new_thread() {
    heap::init_heap();
    debug_println!("hello new thread");
    get_heap();

    // panic!("new thread exit");
    loop {

    }
}

#[root_task]
fn main(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    sel4::debug_println!("Hello, World!");
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
    badged_notification.signal();
    sel4::debug_println!("after signal! {:?}", unbadged_notification.cptr());
    let (_, observed_badge) = unbadged_notification.wait();
    sel4::debug_println!("after wait! observed_badge: {:?}", observed_badge);
    let _  = obj_allocator.create_thread(new_thread, 255);


    sel4::debug_println!("TEST_PASS");
    get_heap();

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}
