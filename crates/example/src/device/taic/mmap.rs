use sel4::BootInfo;
use sel4_root_task::debug_println;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;
use sel4::{FrameSize, ObjectBlueprint, ObjectBlueprintArch, VMAttributes, CapRights};
use sel4::cap_type::{Untyped, MegaPage};

pub fn mmap_device_addr(boot_info: &BootInfo, base_addr: usize, len: usize, v_offset: usize) {
    let obj_allocator = &GLOBAL_OBJ_ALLOCATOR;
    let (mut taic_untyped, mut taic_untyped_bits) = (BootInfo::init_cspace_local_cptr::<Untyped>(0), 0);
    for (i, desc) in boot_info.untyped_list().iter().enumerate() {
        if desc.is_device() && desc.paddr() <= base_addr && desc.paddr() + (1 << desc.size_bits()) > base_addr {
            debug_println!(
                "taic untyped: paddr: {:x}, size_bits: {}, is_device: {}",
                desc.paddr(),
                desc.size_bits(),
                desc.is_device()
            );
            taic_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(boot_info.untyped().start + i);
            taic_untyped_bits = desc.size_bits();
            break;
        }
    }
    let taic_untyped_slot = obj_allocator.lock().get_empty_slot();
    let retype_bits = taic_untyped_bits - FrameSize::MEGA_BITS - 3;
    let retype_num = 1 << 3;
    let bluprint = ObjectBlueprint::Untyped {
        size_bits: FrameSize::MEGA_BITS + 3
    };

    let cnode = BootInfo::init_thread_cnode();

    taic_untyped.untyped_retype(
        &bluprint,
        &cnode.relative_self(),
        taic_untyped_slot,
        retype_num
    ).unwrap();

    for _ in 0..retype_num - 1 {
        let _ = obj_allocator.lock().get_empty_slot();
    }
    debug_println!("retype num: {}", retype_num);

    let net_frame_slot = obj_allocator.lock().get_empty_slot();

    for i in 0..retype_num {
        let bluprint = ObjectBlueprint::Arch(ObjectBlueprintArch::MegaPage);
        let taic_frame_untyped = BootInfo::init_cspace_local_cptr::<Untyped>(taic_untyped_slot + i);
        taic_frame_untyped.untyped_retype(
            &bluprint,
            &cnode.relative_self(),
            net_frame_slot + i,
            1
        ).unwrap();
        let _ = obj_allocator.lock().get_empty_slot();
        let taic_frame = BootInfo::init_cspace_local_cptr::<MegaPage>(net_frame_slot + i);
        let paddr = taic_frame.frame_get_address().unwrap();
        debug_println!("paddr: {:#x}, {:#x}", paddr, 1 << FrameSize::MEGA_BITS);
        if paddr <= base_addr && paddr + (1 << FrameSize::MEGA_BITS) > base_addr {
            let vaddr = paddr + v_offset;
            debug_println!("taic_frame paddr: {:#x}, vaddr: {:#x}", paddr, vaddr);

            let l2_page_table = obj_allocator.lock().alloc_page_table().unwrap();
            l2_page_table.page_table_map(BootInfo::init_thread_vspace(), vaddr, VMAttributes::DEFAULT).unwrap();
            taic_frame.frame_map(
                BootInfo::init_thread_vspace(),
                vaddr,
                CapRights::read_write(),
                VMAttributes::DEFAULT,
            ).unwrap();
            break;
        }
    }
}
