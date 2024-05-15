use alloc::{boxed::Box, vec::Vec};
use async_runtime::{coroutine_run_until_blocked, coroutine_spawn};
use sel4::{cap_type::_4KPage, debug_println, get_clock, CapRights, LocalCPtr, ObjectBlueprintArch, VMAttributes};
use super::async_syscall::*;
use crate::object_allocator::GLOBAL_OBJ_ALLOCATOR;



const START_ADDR: usize = 0x200_0000;
const END_ADDR: usize = 0x201_0000;
const PAGE_SIZE: usize = 0x1000;
const MAX_PAGE_NUM: usize = (END_ADDR- START_ADDR) / PAGE_SIZE;


struct TestData {
    data:  usize
}

impl TestData {
    pub fn change_data(&mut self) {
        self.data = 10000000;
        debug_println!("TestData: data change to {:?}", self.data);
    }
}


pub struct AsyncMemoryAllocator {
    current: usize,
    recycled: Vec<usize>,
    frames: [LocalCPtr<_4KPage>; MAX_PAGE_NUM],
    mapped_vaddrs: [usize; MAX_PAGE_NUM]
}

impl AsyncMemoryAllocator {

    pub fn new() -> Self {
        let mut allocator = AsyncMemoryAllocator{
            current: 0,
            recycled: Vec::<usize>::new(),
            frames: [LocalCPtr::from_bits(0); MAX_PAGE_NUM],
            mapped_vaddrs: [0; MAX_PAGE_NUM]
        };
        allocator
    }

    fn alloc_slot(&mut self) -> Option<usize> {
        if let Some(slot) = self.recycled.pop() {
            Some(slot)
        } else {
            if self.current < MAX_PAGE_NUM {
                self.current += 1;
                Some(self.current - 1)
            } else {
                None
            }
        }
    }

    pub async fn run_async_memory_alloc_test(&mut self) {
        debug_println!("\nAsyncMemoryAllocator: Begin Test!");
        let start_time = get_clock() as usize;
        self.init().await;
        // 测试 
        let mut vaddr = START_ADDR;
        for i in 0..MAX_PAGE_NUM {
            // coroutine_spawn(Box::pin(single_test(self as *mut Self as usize, vaddr)));
            vaddr = vaddr + PAGE_SIZE;
        }
        let end_time = get_clock() as usize;
        let time = end_time - start_time;
        debug_println!("\nAsyncMemoryAllocator: Test Finish!\nTime Sum: {:?}, Average: {:?}", time, time / MAX_PAGE_NUM);
    }

    pub async fn init(&mut self) {
        let obj_allocator = unsafe {
            &GLOBAL_OBJ_ALLOCATOR
        };
        let cnode = sel4::BootInfo::init_thread_cnode();
        let dst = cnode.relative_self();
        // 分配页框
        for i in 0..MAX_PAGE_NUM {
            let blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::_4KPage);
            let untyped = obj_allocator.lock().get_the_first_untyped_slot(&blueprint);
            let slot = obj_allocator.lock().get_empty_slot();
            syscall_untyped_retype(
                untyped.cptr(),
                blueprint, 
                blueprint.api_size_bits().unwrap_or(0).try_into().unwrap(), 
                dst.root().cptr(), 
                dst.path().bits() as usize, 
                dst.path().depth().try_into().unwrap(), 
                slot, 
                1).await;
            let frame = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::_4KPage>(
                slot,
            );
            self.frames[i] = frame;
        }
        // 申请页表
        let pt_blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::PageTable);
        let pt_untyped = obj_allocator.lock().get_the_first_untyped_slot(&pt_blueprint);
        let pt_slot = obj_allocator.lock().get_empty_slot();
        syscall_untyped_retype(
            pt_untyped.cptr(),
            pt_blueprint, 
            pt_blueprint.api_size_bits().unwrap_or(0).try_into().unwrap(), 
            dst.root().cptr(), 
            dst.path().bits() as usize, 
            dst.path().depth().try_into().unwrap(), 
            pt_slot, 
            1).await;
        let page_table = sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::_4KPage>(
            pt_slot
        );
        let vspace = sel4::BootInfo::init_thread_vspace();
        let vaddr = 0x200_0000;
        syscall_riscv_pagetable_map(
            page_table.cptr(),
            vspace.cptr(),
            vaddr,
            VMAttributes::default().into_inner() as usize
        ).await;
    }

    pub async fn single_test(&mut self, vaddr: usize) {
        self.map_vaddr(vaddr).await;
        self.unmap_vaddr(vaddr).await;        
    }

    pub async fn map_vaddr(&mut self, vaddr: usize){
        // 检查vaddr
        if vaddr >= START_ADDR && vaddr < END_ADDR && vaddr % 0x1000 == 0  {
            // 分配slot并映射
            if let Some(slot) = self.alloc_slot() {
                let frame = self.frames[slot];
                let vspace = sel4::BootInfo::init_thread_vspace();
                syscall_riscv_page_map(
                    frame.cptr(),
                    vspace.cptr(),
                    vaddr,
                    CapRights::read_write().into_inner().0.inner()[0] as usize,
                    VMAttributes::default().into_inner() as usize
                ).await;
                self.mapped_vaddrs[slot] = vaddr;
            } else {
                debug_println!("AsyncMemoryAllocator: no available slot!");
            }
        } else {
            debug_println!("AsyncMemoryAllocator: invalid vaddr {:#x}!", vaddr);
        }
    }

    pub async fn unmap_vaddr(&mut self, vaddr: usize) {
        // 检查vaddr是否被映射
        for (index, va) in self.mapped_vaddrs.into_iter().enumerate() {
            // 如果被映射了则解除映射
            if vaddr == va {
                let frame = self.frames[index];
                syscall_riscv_page_unmap(frame.cptr()).await;
                self.mapped_vaddrs[index] = 0;
                // 回收slot
                self.recycled.push(index);
                return;
            }
        }
        debug_println!("AsyncMemoryAllocator: unmapped vaddr!");
    }
}

pub struct SyncMemoryAllocator {
    current: usize,
    recycled: Vec<usize>,
    frames: [LocalCPtr<_4KPage>; MAX_PAGE_NUM],
    mapped_vaddrs: [usize; MAX_PAGE_NUM]
}

impl SyncMemoryAllocator {

    pub fn new() -> Self {
        let mut allocator = SyncMemoryAllocator{
            current: 0,
            recycled: Vec::<usize>::new(),
            frames: [LocalCPtr::from_bits(0); MAX_PAGE_NUM],
            mapped_vaddrs: [0; MAX_PAGE_NUM]
        };
        allocator
    }

    fn alloc_slot(&mut self) -> Option<usize> {
        if let Some(slot) = self.recycled.pop() {
            Some(slot)
        } else {
            if self.current < MAX_PAGE_NUM {
                self.current += 1;
                Some(self.current - 1)
            } else {
                None
            }
        }
    }

    pub fn run_sync_memory_alloc_test(&mut self) {
        debug_println!("\nSyncMemoryAllocator: Begin Test!");
        let start_time = get_clock() as usize;
        self.init();
        // 测试 
        let mut vaddr = START_ADDR;
        for i in 0..MAX_PAGE_NUM {
            self.map_vaddr(vaddr);
            // let data = unsafe {
            //     &mut *(vaddr as *mut TestData)
            // };    
            // data.change_data();
            self.unmap_vaddr(vaddr);
            vaddr = vaddr + PAGE_SIZE;
        }
        let end_time = get_clock() as usize;
        let time = end_time - start_time;
        debug_println!("\nSyncMemoryAllocator: Test Finish!\nTime Sum: {:?}, Average: {:?}", time, time / MAX_PAGE_NUM);
    }

    pub fn init(&mut self) {
        let obj_allocator = unsafe {
            &GLOBAL_OBJ_ALLOCATOR
        };
        // 准备页框
        for i in 0..MAX_PAGE_NUM {
            let frame = obj_allocator.lock().alloc_frame().unwrap();
            self.frames[i] = frame;
        }
        // 申请页表
        let page_table = obj_allocator.lock().alloc_page_table().unwrap();
        let vspace = sel4::BootInfo::init_thread_vspace();
        let vaddr = 0x200_0000;
        page_table.page_table_map(vspace, vaddr, VMAttributes::default());
    }

    pub fn map_vaddr(&mut self, vaddr: usize){
        // 检查vaddr
        if vaddr >= START_ADDR && vaddr < END_ADDR && vaddr % 0x1000 == 0  {
            // 分配slot并映射
            if let Some(slot) = self.alloc_slot() {
                let frame = self.frames[slot];
                let vspace = sel4::BootInfo::init_thread_vspace();
                frame.frame_map(vspace, vaddr, CapRights::read_write(), VMAttributes::default());
                self.mapped_vaddrs[slot] = vaddr;
            } else {
                debug_println!("SyncMemoryAllocator: no available slot!");
            }
        } else {
            debug_println!("SyncMemoryAllocator: invalid vaddr {:#x}!", vaddr);
        }
    }

    pub async fn unmap_vaddr(&mut self, vaddr: usize) {
        // 检查vaddr是否被映射
        for (index, va) in self.mapped_vaddrs.into_iter().enumerate() {
            // 如果被映射了则解除映射
            if vaddr == va {
                let frame = self.frames[index];
                frame.frame_unmap();
                self.mapped_vaddrs[index] = 0;
                // 回收slot
                self.recycled.push(index);
                return;
            }
        }
        debug_println!("SyncMemoryAllocator: unmapped vaddr!");
    }
}

