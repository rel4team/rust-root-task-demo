use alloc::alloc::alloc_zeroed;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::arch::asm;
use core::ops::Range;
use spin::Mutex;
use sel4::{CNodeCapData, InitCSpaceSlot, LocalCPtr, UntypedDesc};
use sel4::cap_type::Untyped;
use sel4::ObjectBlueprintArch;
use sel4::UserContext;
use sel4_root_task::debug_println;
use crate::image_utils::UserImageUtils;


pub static GLOBAL_OBJ_ALLOCATOR: Mutex<ObjectAllocator> = Mutex::new(ObjectAllocator::default());


pub struct ObjectAllocator {
    untyped_list: Vec<UntypedDesc>,
    untyped_start: InitCSpaceSlot,
    empty: Range<InitCSpaceSlot>,
}

#[warn(dead_code)]
impl ObjectAllocator {
    pub fn new(bootinfo: &sel4::BootInfo) -> Self {
        Self {
            untyped_list: bootinfo.untyped_list().to_vec(),
            untyped_start: bootinfo.untyped().start,
            empty: bootinfo.empty()
        }
    }

    pub const fn default() -> Self {
        Self {
            untyped_list: Vec::new(),
            untyped_start: 0,
            empty: Range { start: 0, end: 0},
        }
    }

    pub fn init(&mut self, bootinfo: &sel4::BootInfo) {
        self.untyped_list = bootinfo.untyped_list().to_vec();
        self.untyped_start = bootinfo.untyped().start;
        self.empty = bootinfo.empty();
    }

    fn get_the_first_untyped_slot(&mut self, blueprint: &sel4::ObjectBlueprint) -> LocalCPtr<Untyped> {
        {
            let idx = self
                .untyped_list
                .iter()
                .position(|desc| {
                    !desc.is_device() && desc.size_bits() >= blueprint.physical_size_bits()
                }).unwrap();
            self.untyped_list.remove(idx);
            let slot = self.untyped_start + idx;
            sel4::BootInfo::init_cspace_local_cptr::<Untyped>(slot)
        }
    }

    #[inline]
    pub fn get_empty_slot(&mut self) -> InitCSpaceSlot {
        self.empty.next().unwrap()
    }

    pub fn alloc_ntfn(&mut self) -> sel4::Result<LocalCPtr<sel4::cap_type::Notification>> {
        let blueprint = sel4::ObjectBlueprint::Notification;
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.next().unwrap();
        let cnode = sel4::BootInfo::init_thread_cnode();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot,
            1,
        )?;
        Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
            slot,
        ))
    }

    pub fn alloc_ep(&mut self) -> sel4::Result<LocalCPtr<sel4::cap_type::Endpoint>> {
        let blueprint = sel4::ObjectBlueprint::Endpoint;
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.next().unwrap();
        let cnode = sel4::BootInfo::init_thread_cnode();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot,
            1,
        )?;
        Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Endpoint>(
            slot,
        ))
    }

    pub fn alloc_frame(&mut self) -> sel4::Result<LocalCPtr<sel4::cap_type::_4KPage>> {
        let blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::_4KPage);
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.next().unwrap();
        let cnode = sel4::BootInfo::init_thread_cnode();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot,
            1,
        )?;
        Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::_4KPage>(
            slot,
        ))
    }

    pub fn alloc_tcb(&mut self) -> sel4::Result<LocalCPtr<sel4::cap_type::TCB>> {
        let blueprint = sel4::ObjectBlueprint::TCB;
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.next().unwrap();
        let cnode = sel4::BootInfo::init_thread_cnode();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot,
            1,
        )?;
        Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::TCB>(
            slot,
        ))
    }

    pub fn alloc_page_table(&mut self) -> sel4::Result<LocalCPtr<sel4::cap_type::PageTable>> {
        let blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::PageTable);
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.next().unwrap();
        let cnode = sel4::BootInfo::init_thread_cnode();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot,
            1,
        )?;
        Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::PageTable>(
            slot,
        ))
    }

    pub fn create_thread(&mut self, func: fn(usize, usize), args: usize, prio: usize, affinity: u64) -> sel4::Result<LocalCPtr<sel4::cap_type::TCB>>
    {
        let ipc_buffer_layout = Layout::from_size_align(4096, 4096)
            .expect("Failed to create layout for page aligned memory allocation");
        let ipc_buffer_addr = unsafe {
            let ptr = alloc_zeroed(ipc_buffer_layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr as usize
        };
        let ipc_buffer_cap = UserImageUtils.get_user_image_frame_slot(ipc_buffer_addr) as u64;
        let tcb = self.alloc_tcb()?;
        let ep = self.alloc_ep()?;
        let cnode = sel4::BootInfo::init_thread_cnode();
        let vspace = sel4::BootInfo::init_thread_vspace();
        let ipc_buffer = LocalCPtr::<sel4::cap_type::_4KPage>::from_bits(ipc_buffer_cap);
        tcb.tcb_configure(ep.cptr(), cnode, CNodeCapData::new(0, 0), vspace, ipc_buffer_addr as u64, ipc_buffer)?;
        tcb.tcb_set_sched_params(sel4::BootInfo::init_thread_tcb(), prio as u64, prio as u64)?;
        let mut user_context = tcb.tcb_read_registers(false, (core::mem::size_of::<UserContext>() / sel4::WORD_SIZE) as u64)?;

        let new_stack_layout = Layout::from_size_align(4096 * 256, 4096).expect("Failed to create layout for page aligned memory allocation");
        let raw_sp = unsafe {
            let ptr = alloc_zeroed(new_stack_layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr.add(4096 * 256) as u64
        };
        let mut tp = raw_sp - 4096 * 128;
        tp = tp & (!((1 << 12) - 1));
        debug_println!("tp: {:#x}", tp);

        user_context.inner_mut().tp = tp;
        *(user_context.pc_mut()) = unsafe { core::mem::transmute(func) };
        *(user_context.sp_mut()) = tp & !(16 - 1);

        user_context.inner_mut().s0 = 0;
        user_context.inner_mut().s1 = 0;


        let gp: u64;
        unsafe {
            asm!("mv {}, gp", out(reg) gp);
        }
        user_context.inner_mut().gp = gp;
        user_context.inner_mut().a0 = args as u64;
        user_context.inner_mut().a1 = ipc_buffer_addr as u64;
        debug_println!("write register: {:?}", user_context);
        tcb.tcb_write_all_registers(false, &mut user_context)?;

        tcb.tcb_set_affinity(affinity)?;
        tcb.tcb_resume()?;
        Ok(tcb)
    }
}