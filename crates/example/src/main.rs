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

mod device;
mod tcp_test;
mod net;

use alloc::alloc::alloc_zeroed;
use core::alloc::Layout;
use core::arch::asm;

use sel4::{IPCBuffer, with_ipc_buffer};
use sel4_logging::LevelFilter;
use sel4_root_task::{debug_print, debug_println};
use sel4_root_task::root_task;
use sel4_logging::{LoggerBuilder, Logger};
use crate::ipc_test::{async_ipc_test, sync_ipc_test};
// use crate::syscall_test::async_syscall_test;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
use crate::tcp_test::net_stack_test;

const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .write(|s| debug_print!("{}", s))
    .build();

#[root_task(stack_size = 4096 * 128)]
fn main(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    debug_println!("Hello, World!");
    LOGGER.set().unwrap();

    heap::init_heap();

    image_utils::UserImageUtils.init(bootinfo);
    GLOBAL_OBJ_ALLOCATOR.lock().init(bootinfo);

    async_ipc_test(bootinfo)?;
    // net_stack_test(bootinfo)?;
    // sync_ipc_test(bootinfo)?;
    // async_syscall_test(bootinfo)?;
    debug_println!("TEST_PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}
