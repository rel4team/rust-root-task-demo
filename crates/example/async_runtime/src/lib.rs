#![no_std]
#![feature(thread_local)]
#![feature(generic_const_exprs)]
#![feature(core_intrinsics)]
extern crate alloc;

mod executor;
mod coroutine;
mod new_buffer;
pub mod utils;

use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use core::alloc::Layout;
use core::cell::{RefCell, RefMut};
use core::future::Future;
use core::mem;
use core::mem::{forget, size_of};
use core::pin::Pin;
use spin::Lazy;
pub use executor::*;
pub use new_buffer::*;
pub use coroutine::*;

#[thread_local]
static mut EXECUTOR: usize = 0;

#[inline]
pub fn get_executor() -> &'static mut Executor {
    unsafe {
        &mut *(EXECUTOR as *mut Executor)
    }
}

#[inline]
pub fn runtime_init() {
    let rt_layout = Layout::from_size_align(size_of::<Executor>(), 4096).expect("Failed to create layout for page aligned memory allocation");
    unsafe {
        EXECUTOR = {
            let ptr = alloc_zeroed(rt_layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr as usize
        }
    }
    get_executor().init();
}

#[inline]
pub fn coroutine_spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>) -> CoroutineId {
    unsafe {
        get_executor().spawn(future, 1)
    }
}

#[inline]
pub fn coroutine_spawn_with_prio(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize) -> CoroutineId {
    unsafe {
        get_executor().spawn(future, prio)
    }
}

#[inline]
pub fn coroutine_delay_wake(cid: &CoroutineId) {
    // sel4::debug_println!("Hello, coroutine_delay_wake!: {}", cid.0);
    get_executor().delay_wake(cid);
}

#[inline]
pub fn coroutine_wake_with_value(cid: &CoroutineId, value: u64) {
    // sel4::debug_println!("coroutine_wake_with_value: {}", cid.0);
    unsafe {
        let exec = get_executor();
        exec.immediate_value[cid.0 as usize] = Some(value);
        exec.wake(cid);
    }
}

#[inline]
pub fn coroutine_get_immediate_value(cid: &CoroutineId) -> Option<u64> {
    unsafe {
        let exec = get_executor();
        let ans = exec.immediate_value[cid.0 as usize];
        exec.immediate_value[cid.0 as usize] = None;
        ans
    }
}

#[inline]
pub fn coroutine_get_current() -> CoroutineId {
    unsafe {
        get_executor().current.unwrap()
    }
}

#[inline]
pub fn get_executor_ptr() -> usize {
    unsafe {
        EXECUTOR
    }
}

#[inline]
pub fn coroutine_run_until_blocked() {
    unsafe {
        get_executor().run_until_blocked()
    }
}

#[inline]
pub fn coroutine_is_empty() -> bool {
    unsafe {
        get_executor().is_empty()
    }
}

#[inline]
pub fn coroutine_run_until_complete() {
    unsafe {
        get_executor().run_until_complete()
    }
}
