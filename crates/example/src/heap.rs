use lazy_static::*;
use spin::mutex::Mutex;

use core::{
    alloc::{GlobalAlloc, Layout}, borrow::Borrow, cell::RefCell, ptr::NonNull
};

use buddy_system_allocator::Heap;
use super::utils::IndexAllocator;

const HEAP_SIZE: usize = 1 << 20;
const MAX_THREAD_SIZE: usize = 16;

#[thread_local]
static mut HEAP_ID: usize = 0;

lazy_static! {
    static ref HEAP_ALLOCATOR: Mutex<IndexAllocator<MAX_THREAD_SIZE>> = Mutex::new(IndexAllocator::new());
    static ref HEAP_POOL: [u64; core::mem::size_of::<Heap>() * MAX_THREAD_SIZE / 8] = [0u64; core::mem::size_of::<Heap>() * MAX_THREAD_SIZE / 8];
}


static mut HEAP_MEM: [u64; HEAP_SIZE * MAX_THREAD_SIZE / 8] = [0u64; HEAP_SIZE * MAX_THREAD_SIZE / 8];

fn get_heap_mut_ref() -> &'static mut Heap {
    unsafe {
        &mut *((HEAP_POOL.as_ptr() as usize + HEAP_ID * core::mem::size_of::<Heap>()) as *mut Heap)
    }
}

pub fn init_heap() {
    if let Some(heap_id) = HEAP_ALLOCATOR.lock().allocate() {
        unsafe {
            HEAP_ID = heap_id;
            get_heap_mut_ref().init(HEAP_MEM.as_ptr() as usize + heap_id * HEAP_SIZE, HEAP_SIZE);
        }
    } else {
        panic!("fail to alloc heap space");
    }
}

pub fn get_heap() {
    unsafe {
        sel4_root_task::debug_println!("heap id: {}", HEAP_ID);
    }
}

struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        return get_heap_mut_ref().alloc(layout).ok()
        .map_or(0 as *mut u8, |allocation| allocation.as_ptr());
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        get_heap_mut_ref().dealloc(NonNull::new_unchecked(ptr), layout);
        return;
    }
}