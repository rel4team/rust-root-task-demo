#![no_std]
#![feature(thread_local)]
#![feature(generic_const_exprs)]
extern crate alloc;

mod executor;
mod coroutine;
mod new_buffer;
pub mod utils;

use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use lazy_static::lazy_static;
pub use executor::*;
pub use new_buffer::*;
pub use coroutine::*;

#[thread_local]
static mut EXECUTOR: Executor = Executor::new();


#[inline]
pub fn coroutine_spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>) -> CoroutineId {
    unsafe {
        EXECUTOR.spawn(future)
    }
}

#[inline]
pub fn coroutine_wake(cid: &CoroutineId) {
    // sel4::debug_println!("coroutine_wake: {}, {:#x}", cid.0, unsafe { &EXECUTOR as *const Executor as usize });
    unsafe {
        EXECUTOR.wake(cid);
    }
}

#[inline]
pub fn coroutine_wake_with_value(cid: &CoroutineId, value: u64) {
    // sel4::debug_println!("coroutine_wake_with_value");
    unsafe {
        EXECUTOR.immediate_value[cid.0 as usize] = Some(value);
        EXECUTOR.wake(cid);
    }
}

#[inline]
pub fn coroutine_get_immediate_value(cid: &CoroutineId) -> Option<u64> {
    unsafe {
        let ans = EXECUTOR.immediate_value[cid.0 as usize];
        EXECUTOR.immediate_value[cid.0 as usize] = None;
        ans
    }
}

#[inline]
pub fn coroutine_get_current() -> CoroutineId {
    unsafe {
        EXECUTOR.current.unwrap()
    }
}

#[inline]
pub fn get_executor_ptr() -> usize {
    unsafe {
        &EXECUTOR as *const Executor as usize
    }
}

#[inline]
pub fn coroutine_run_until_blocked() {
    unsafe {
        EXECUTOR.run_until_blocked()
    }
}

#[inline]
pub fn coroutine_is_empty() -> bool {
    unsafe {
        EXECUTOR.is_empty()
    }
}

#[inline]
pub fn coroutine_run_until_complete() {
    unsafe {
        EXECUTOR.run_until_complete()
    }
}
