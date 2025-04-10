use super::BlockDevice;

use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{transport, BufferDirection, Hal};
use virtio_drivers::device::blk::VirtIOBlk;
// use virtio_drivers::transport::mmio::MmioTransport;


use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::vec;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::ptr::NonNull;
use spin::{Lazy, Mutex};

// use virtio_drivers::device::net::{RxBuffer, TxBuffer, VirtIONet};
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use sel4::BootInfo;
use sel4::cap_type::{Untyped, MegaPage};
use sel4::{FrameSize, ObjectBlueprint, ObjectBlueprintArch, VMAttributes, CapRights};
use sel4_logging::log::debug;
use sel4_root_task::debug_println;
use crate::image_utils::UserImageUtils;
use crate::net::snoop_tcp_packet;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
use crate::sync::UPSafeCell;


#[allow(unused)]
const VIRTIO0: usize = 0x5000_2000;



pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<VirtioHal,MmioTransport>>);


impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_blocks(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_blocks(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            let header = NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap();
            let transport = MmioTransport::new(header).unwrap();
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtioHal,MmioTransport>::new(transport).unwrap(),
            ))
        }
    }
}

pub struct VirtioHal;

unsafe impl Hal for VirtioHal {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (usize, NonNull<u8>) {
        const PAGE_SIZE:usize = 4096;
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE)
            .expect("Failed to create layout for page aligned memory allocation");
        let vptr = unsafe {
            let ptr = alloc_zeroed(layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr as usize
        };
        let paddr = UserImageUtils.get_user_image_frame_paddr(vptr);
        // debug_println!("[dma_alloc] pages: {}, paddr: {:#x}, vaddr: {:#x}",pages, paddr, vptr);

        // debug!("[dma_alloc]paddr: {:#x}, vaddr: {:#x}", paddr, paddr + PPTR_BASE_OFFSET);
        (paddr, NonNull::new(vptr as _).unwrap())
    }

    unsafe fn dma_dealloc(_paddr: usize, _vaddr: NonNull<u8>, _pages: usize) -> i32 {
        // debug_println!("dma_dealloc");
        // trace!("dealloc DMA: paddr={:#x}, pages={}", paddr, pages);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: usize, _size: usize) -> NonNull<u8> {
        // debug_println!("mmio_phys_to_virt");
        NonNull::new(paddr as _).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> usize {
        // debug_println!("share");
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        // let len = buffer.len();
        // debug_println!("[share] vaddr: {:#x}, data: {}", vaddr, buffer.len());
        // Nothing to do, as the host already has access to all memory.
        UserImageUtils.get_user_image_frame_paddr(vaddr)
    }

    unsafe fn unshare(_paddr: usize, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // debug_println!("unshare");
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}