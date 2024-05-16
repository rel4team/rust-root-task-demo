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
#![feature(new_uninit)]
#![allow(dead_code, unused_imports)]
extern crate alloc;
mod heap;
mod object_allocator;
mod async_lib;
mod image_utils;
mod ipc_test;
mod syscall_test;
mod async_syscall;

mod device;
mod async_tcp_test;
mod sync_tcp_test;
mod net;
mod matrix;
mod memory_allocator;

use alloc::alloc::alloc_zeroed;
use core::alloc::Layout;
use core::arch::asm;

use sel4::{IPCBuffer, with_ipc_buffer};
use sel4_logging::LevelFilter;
use sel4_root_task::{debug_print, debug_println};
use sel4_root_task::root_task;
use sel4_logging::{LoggerBuilder, Logger};
use crate::ipc_test::{async_ipc_test, sync_ipc_test};
use crate::syscall_test::async_syscall_test;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
// use crate::sync_tcp_test::net_stack_test;
use crate::async_tcp_test::net_stack_test;

const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .write(|s| debug_print!("{}", s))
    .build();

fn expand_tls() {
    const PAGE_SIZE: usize = 4096;
    const TLS_SIZE: usize = 128;
    let layout = Layout::from_size_align(TLS_SIZE * PAGE_SIZE, PAGE_SIZE)
        .expect("Failed to create layout for page aligned memory allocation");
    let vptr = unsafe {
        let ptr = alloc_zeroed(layout);
        if ptr.is_null() {
            panic!("Failed to allocate page aligned memory");
        }
        ptr as usize
    };

    let ipc_buffer_ptr = with_ipc_buffer(|buffer| {
        buffer.ptr() as *mut sel4::sys::seL4_IPCBuffer
    });

    unsafe {
        asm!("mv tp, {}", in(reg) vptr);
    }

    let ipcbuf = unsafe {
        IPCBuffer::from_ptr(ipc_buffer_ptr)
    };
    sel4::set_ipc_buffer(ipcbuf);
}

#[root_task(stack_size = 4096 * 128)]
fn main(bootinfo: &sel4::BootInfo) -> sel4::Result<!> {
    debug_println!("Hello, World!");
    LOGGER.set().unwrap();
    heap::init_heap();
    expand_tls();
    let recv_tcb = sel4::BootInfo::init_thread_tcb();
    recv_tcb.tcb_set_affinity(1);
    image_utils::UserImageUtils.init(bootinfo);
    GLOBAL_OBJ_ALLOCATOR.lock().init(bootinfo);
    // async_ipc_test(bootinfo)?;
    // net_stack_test(bootinfo)?;
    // sync_ipc_test(bootinfo)?;
    async_syscall_test(bootinfo)?;
    debug_println!("TEST_PASS");

    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}
