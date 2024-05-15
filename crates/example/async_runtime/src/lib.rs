#![no_std]
#![feature(thread_local)]
#![feature(generic_const_exprs)]
#![feature(core_intrinsics)]
extern crate alloc;

mod executor;
mod coroutine;
mod new_buffer;
mod message_info;
pub mod utils;

use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use core::alloc::Layout;
use core::future::Future;
use core::mem::size_of;
use core::pin::Pin;
pub use executor::*;
pub use new_buffer::*;
pub use coroutine::*;
pub use message_info::*;

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
    get_executor().spawn(future, 1)
}

#[inline]
pub fn coroutine_spawn_with_prio(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize) -> CoroutineId {
    get_executor().spawn(future, prio)
}

#[inline]
pub fn coroutine_possible_switch() -> bool {
    get_executor().switch_possible()
}

#[inline]
pub fn coroutine_delay_wake(cid: &CoroutineId) {
    // sel4::debug_println!("Hello, coroutine_delay_wake!: {}", cid.0);
    get_executor().delay_wake(cid);
}

#[inline]
pub fn coroutine_wake(cid: &CoroutineId) {
    get_executor().wake(cid);
}


#[inline]
pub fn coroutine_get_current() -> CoroutineId {
    get_executor().current.unwrap()
}

#[inline]
pub fn get_executor_ptr() -> usize {
    unsafe {
        EXECUTOR
    }
}

#[inline]
pub fn get_ready_num() -> usize {
    get_executor().get_ready_num()
}

#[inline]
pub fn coroutine_run_until_blocked() {
    get_executor().run_until_blocked()
}

#[inline]
pub fn coroutine_is_empty() -> bool {
    get_executor().is_empty()
}

#[inline]
pub fn coroutine_run_until_complete() {
    get_executor().run_until_complete()
}
