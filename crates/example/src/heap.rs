use lazy_static::*;
use spin::mutex::Mutex;

use core::{
    alloc::{GlobalAlloc, Layout}, borrow::Borrow, ptr::NonNull
};

use sel4::get_clock;

use buddy_system_allocator::Heap;

const HEAP_SIZE: usize = 1 << 24;
const MAX_THREAD_SIZE: usize = 1;

// #[thread_local]
// static mut HEAP_ID: usize = 0;

lazy_static! {
    // static ref HEAP_ALLOCATOR: Mutex<IndexAllocator<MAX_THREAD_SIZE>> = Mutex::new(IndexAllocator::new());
    static ref HEAP_POOL: [u64; core::mem::size_of::<Heap>() * MAX_THREAD_SIZE / 8] = [0u64; core::mem::size_of::<Heap>() * MAX_THREAD_SIZE / 8];
}


static mut HEAP_MEM: [u64; HEAP_SIZE * MAX_THREAD_SIZE / 8] = [0u64; HEAP_SIZE * MAX_THREAD_SIZE / 8];
pub static mut HEAP: spin::Mutex<Heap> = Mutex::new(Heap::empty());
// fn get_heap_mut_ref() -> &'static mut Heap {
//     unsafe {
//         &mut *((HEAP_POOL.as_ptr() as usize + HEAP_ID * core::mem::size_of::<Heap>()) as *mut Heap)
//     }
// }

pub fn init_heap() {
    // sel4_root_task::debug_println!("HEAP_ALLOCATOR ptr: {:#x}", &HEAP_ALLOCATOR as *const Mutex<IndexAllocator<MAX_THREAD_SIZE>> as usize);
    // if let Some(heap_id) = HEAP_ALLOCATOR.lock().allocate() {
    //     unsafe {
    //         HEAP_ID = heap_id;
    //         get_heap_mut_ref().init(HEAP_MEM.as_ptr() as usize + heap_id * HEAP_SIZE, HEAP_SIZE);
    //     }
    // } else {
    //     panic!("fail to alloc heap space");
    // }
    unsafe {
        HEAP.lock().init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
    }
}

// pub fn get_heap() {
//     unsafe {
//         sel4_root_task::debug_println!("heap id: {}", HEAP_ID);
//     }
// }

struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // let start = get_clock();
        let c = HEAP.lock().alloc(layout).ok()
        .map_or(0 as *mut u8, |allocation| allocation.as_ptr());
        // let end = get_clock();
        // debug_println!("alloc: {}", end - start);
        c
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // let start = get_clock();
        HEAP.lock().dealloc(NonNull::new_unchecked(ptr), layout);
        // let end = get_clock();
        // debug_println!("dealloc: {}", end - start);
        return;
    }
}