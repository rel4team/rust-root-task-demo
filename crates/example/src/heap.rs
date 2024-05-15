use lazy_static::*;
use spin::mutex::Mutex;

use core::{
    alloc::{GlobalAlloc, Layout}, borrow::Borrow, ptr::NonNull
};

use sel4::get_clock;

use buddy_system_allocator::Heap;

const HEAP_SIZE: usize = 1 << 24;
const MAX_THREAD_SIZE: usize = 1;


lazy_static! {
    static ref HEAP_POOL: [u64; core::mem::size_of::<Heap>() * MAX_THREAD_SIZE / 8] = [0u64; core::mem::size_of::<Heap>() * MAX_THREAD_SIZE / 8];
}


pub static mut HEAP_MEM: [u64; HEAP_SIZE * MAX_THREAD_SIZE / 8] = [0u64; HEAP_SIZE * MAX_THREAD_SIZE / 8];
pub static mut HEAP: spin::Mutex<Heap> = Mutex::new(Heap::empty());

pub fn init_heap() {

    unsafe {
        HEAP.lock().init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
    }
}


struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let c = HEAP.lock().alloc(layout).ok()
        .map_or(0 as *mut u8, |allocation| allocation.as_ptr());
        c
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.lock().dealloc(NonNull::new_unchecked(ptr), layout);
    }
}