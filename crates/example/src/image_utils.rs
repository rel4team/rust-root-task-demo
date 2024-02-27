use core::ops::Range;
use core::ptr;
use sel4::{InitCSpaceSlot, SizedFrameType};

pub fn get_user_image_frame_slot(
    bootinfo: &sel4::BootInfo,
    addr: usize,
) -> InitCSpaceSlot {
    assert_eq!(addr % GRANULE_SIZE, 0);
    let user_image_footprint = get_user_image_footprint();
    let num_user_frames = bootinfo.user_image_frames().len();
    assert_eq!(user_image_footprint.len(), num_user_frames * GRANULE_SIZE);
    let ix = (addr - user_image_footprint.start) / GRANULE_SIZE;
    bootinfo.user_image_frames().start + ix
}

fn get_user_image_footprint() -> Range<usize> {
    extern "C" {
        static __executable_start: u64;
        static _end: u64;
    }
    unsafe {
        let start = round_down(ptr::addr_of!(__executable_start) as usize, GRANULE_SIZE);
        let end = (ptr::addr_of!(_end) as usize).next_multiple_of(GRANULE_SIZE);
        start..end
    }
}

const fn round_down(n: usize, b: usize) -> usize {
    n - n % b
}

const GRANULE_SIZE: usize = sel4::cap_type::Granule::FRAME_SIZE.bytes();