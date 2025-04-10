use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
use sel4::cap_type::{MegaPage, Untyped};
use sel4::BootInfo;
use sel4::{CapRights, FrameSize, ObjectBlueprint, ObjectBlueprintArch, VMAttributes};
use sel4_root_task::debug_println;

const VIRTIO_BASE_ADDRE: usize = 0x10001000;
const VIRTIO_LEN: usize = 0x1000;
// const V_OFFSET:usize = 0x5000_0000 - VIRTIO_BASE_ADDRE;

pub fn fuse_test(boot_info: &sel4::BootInfo) -> sel4::Result<!> {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;

    // 翻表，找untyped area
    let (mut virtio_untyped, mut virtio_untyped_bits) =
        (BootInfo::init_cspace_local_cptr::<Untyped>(0), 0);

    for (i, desc) in boot_info.untyped_list().iter().enumerate() {
        // debug_println!(
        //     "untyped: paddr: {:x}, size_bits: {}, is_device: {}",
        //     desc.paddr(),
        //     desc.size_bits(),
        //     desc.is_device()
        // );
        if desc.is_device() && desc.paddr() <= VIRTIO_BASE_ADDRE && desc.paddr() + (1 << desc.size_bits()) > VIRTIO_BASE_ADDRE {
            debug_println!(
                "virtio untyped: paddr: {:x}, size_bits: {}, is_device: {}",
                desc.paddr(),
                desc.size_bits(),
                desc.is_device()
            );
            virtio_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(boot_info.untyped().start + i);
            virtio_untyped_bits = desc.size_bits();
            break;
        }
    }

    let virtio_untyped_slot = obj_allocator.lock().get_empty_slot();
    let retype_bits = virtio_untyped_bits - FrameSize::MEGA_BITS;
    let retype_num = (1 << retype_bits)/4 ;
    let bluprint = ObjectBlueprint::Untyped {
        size_bits: FrameSize::MEGA_BITS
    };

    let cnode = BootInfo::init_thread_cnode();
    //retype
    virtio_untyped.untyped_retype(
        &bluprint,
        &cnode.relative_self(),
        virtio_untyped_slot,
        retype_num
    ).unwrap();

    for _ in 0..retype_num - 1 {
        let _ = obj_allocator.lock().get_empty_slot();
    }
    debug_println!("retype num: {}", retype_num);

    let net_frame_slot = obj_allocator.lock().get_empty_slot();

    for i in 0..retype_num {
        let bluprint = ObjectBlueprint::Arch(ObjectBlueprintArch::MegaPage);
        let virtio_frame_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(virtio_untyped_slot + i);
        virtio_frame_untyped.untyped_retype(
            &bluprint,
            &cnode.relative_self(),
            net_frame_slot + i,
            1
        ).unwrap();
        let _ = obj_allocator.lock().get_empty_slot();
        let virtio_frame = BootInfo::init_cspace_local_cptr::<MegaPage>(net_frame_slot + i);
        let paddr = virtio_frame.frame_get_address().unwrap();
        debug_println!("paddr: {:#x}, {:#x}", paddr, 1 << FrameSize::MEGA_BITS);

        if paddr <= VIRTIO_BASE_ADDRE && paddr + (1 << FrameSize::MEGA_BITS) > VIRTIO_BASE_ADDRE {
            let vaddr = 0x50000000;
            debug_println!("virtio_frame paddr: {:#x}, vaddr: {:#x}", paddr, vaddr);

            let l2_page_table = obj_allocator.lock().alloc_page_table().unwrap();
            l2_page_table.page_table_map(BootInfo::init_thread_vspace(), vaddr, VMAttributes::DEFAULT).unwrap();
            virtio_frame.frame_map(
                BootInfo::init_thread_vspace(),
                vaddr,
                CapRights::read_write(),
                VMAttributes::DEFAULT,
            ).unwrap();
            break;
        }
    }



    list_apps();


    sel4::BootInfo::init_thread_tcb().tcb_suspend()?;
    unreachable!()
}


use crate::device::block::BLOCK_DEVICE;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::*;
/// A wrapper around a filesystem inode
/// to implement File trait atop
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}
/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

impl OSInode {
    /// Construct an OS inode from a inode
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }
    /// Read all data inside a inode into vector
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}
/// List all files in the filesystems
pub fn list_apps() {
    debug_println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
      debug_println!("{}", app);
    }
    debug_println!("**************/");
}

bitflags! {
    ///Open file flags
    pub struct OpenFlags: u32 {
        ///Read only
        const RDONLY = 0;
        ///Write only
        const WRONLY = 1 << 0;
        ///Read & Write
        const RDWR = 1 << 1;
        ///Allow create
        const CREATE = 1 << 9;
        ///Clear file and return an empty one
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}
///Open file with flags
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create file
            ROOT_INODE
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

// impl File for OSInode {
//     fn readable(&self) -> bool {
//         self.readable
//     }
//     fn writable(&self) -> bool {
//         self.writable
//     }
//     fn read(&self, mut buf: UserBuffer) -> usize {
//         let mut inner = self.inner.exclusive_access();
//         let mut total_read_size = 0usize;
//         for slice in buf.buffers.iter_mut() {
//             let read_size = inner.inode.read_at(inner.offset, *slice);
//             if read_size == 0 {
//                 break;
//             }
//             inner.offset += read_size;
//             total_read_size += read_size;
//         }
//         total_read_size
//     }
//     fn write(&self, buf: UserBuffer) -> usize {
//         let mut inner = self.inner.exclusive_access();
//         let mut total_write_size = 0usize;
//         for slice in buf.buffers.iter() {
//             let write_size = inner.inode.write_at(inner.offset, *slice);
//             assert_eq!(write_size, slice.len());
//             inner.offset += write_size;
//             total_write_size += write_size;
//         }
//         total_write_size
//     }
// }
