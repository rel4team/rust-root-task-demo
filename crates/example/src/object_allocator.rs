use alloc::vec::Vec;
use core::ops::Range;
use sel4::{CNodeCapData, InitCSpaceSlot, LocalCPtr, UntypedDesc};
use sel4::cap_type::Untyped;
use sel4::ObjectBlueprintArch;
use sel4::UserContext;
use sel4_root_task::debug_println;

pub struct ObjectAllocator {
    untyped_list: Vec<UntypedDesc>,
    untyped_start: InitCSpaceSlot,
    empty: Range<InitCSpaceSlot>,
}

static NEW_STACK: [u8; 4096] = [0u8; 4096];

impl ObjectAllocator {
    pub fn new(bootinfo: &sel4::BootInfo) -> Self {
        Self {
            untyped_list: bootinfo.untyped_list().to_vec(),
            untyped_start: bootinfo.untyped().start,
            empty: bootinfo.empty()
        }
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

    pub fn create_thread(&mut self, func: fn(), prio: usize) -> sel4::Result<LocalCPtr<sel4::cap_type::TCB>>
    {
        let tcb = self.alloc_tcb()?;
        let ipc_buffer = self.alloc_frame()?;
        let ipc_buffer_addr = ipc_buffer.frame_get_address().unwrap();
        debug_println!("hello ipc_buffer addr: {:#x}", ipc_buffer_addr);
        let ep = self.alloc_ep()?;
        let cnode = sel4::BootInfo::init_thread_cnode();
        let vspace = sel4::BootInfo::init_thread_vspace();
        tcb.tcb_configure(ep.cptr(), cnode, CNodeCapData::new(0, 0), vspace, ipc_buffer_addr as u64, ipc_buffer)?;
        tcb.tcb_set_sched_params(sel4::BootInfo::init_thread_tcb(), prio as u64, prio as u64)?;
        let mut user_context = tcb.tcb_read_registers(false, (core::mem::size_of::<UserContext>() / sel4::WORD_SIZE) as u64)?;
        *(user_context.pc_mut()) = unsafe { core::mem::transmute(func) };

        *(user_context.sp_mut()) = unsafe {
            NEW_STACK.as_ptr().add(4096) as u64
        };
        tcb.tcb_write_all_registers(false, &mut user_context)?;
        tcb.tcb_resume()?;
        Ok(tcb)
    }
}