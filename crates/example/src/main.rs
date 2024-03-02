//
// Copyright 2023, Colias Group, LLC
//
// SPDX-License-Identifier: BSD-2-Clause
//

#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(thread_local)]
#![feature(int_roundings)]
#![feature(slice_index_methods)]
#![feature(build_hasher_simple_hash_one)]
extern crate alloc;
mod heap;
mod object_allocator;
mod async_lib;
mod image_utils;
mod ipc_test;
mod syscall_test;

use sel4_root_task::debug_println;
use sel4_root_task::root_task;

use crate::ipc_test::{async_ipc_test, sync_ipc_test};
// use crate::syscall_test::async_syscall_test;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;


#[root_task]
fn main(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    debug_println!("Hello, World!");

    heap::init_heap();

    GLOBAL_OBJ_ALLOCATOR.lock().init(bootinfo);
    async_ipc_test(bootinfo)?;
    // sync_ipc_test(bootinfo)?;
    // async_syscall_test(bootinfo)?;
    debug_println!("TEST_PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}
