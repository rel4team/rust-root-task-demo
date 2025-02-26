mod mmap;
mod utrap_handler;
pub mod interface;

use sel4_root_task::debug_println;
use taic_driver::{LocalQueue, Taic};
use lazy_init::LazyInit;
use alloc::vec::Vec;
use sel4::{get_clock, BootInfo};
use riscv::register::{
    ucause, uepc, uie, uip,
    ustatus::{self, Ustatus},
    utval,
    utvec::{self, TrapMode},
};
use crate::device::taic::utrap_handler::init_utrap_handler;

const TAIC_BASE: usize = 0x100_0000;
const TAIC_OFFSET: usize = 0x4000_0000 - TAIC_BASE;
const TAIC_LEN: usize = 0x100_0000;
static LQ: LazyInit<LocalQueue> = LazyInit::new();
const LQ_NUM: usize = 8;
const TAIC: Taic = Taic::new(TAIC_BASE + TAIC_OFFSET, LQ_NUM);
const NUM: usize = 1000;


fn enq_deq_test() {
    debug_println!("Start Taic enq & deq test ...");
    let lq0 = TAIC.alloc_lq(1, 2).unwrap();
    let mut enq_cycles = Vec::new();
    let mut deq_cycles = Vec::new();
    for i in 1..NUM {
        let enq_start = get_clock();
        lq0.task_enqueue(i);
        let enq_end = get_clock();
        enq_cycles.push(enq_end - enq_start);
    }
    for _i in 1..NUM {
        let deq_start = get_clock();
        let c = lq0.task_dequeue();
        let deq_end = get_clock();
        deq_cycles.push(c);
    }
    debug_println!("Enq cycles: {:?}", enq_cycles);
    debug_println!("---------------------------------");
    debug_println!("Deq cycles: {:?}", deq_cycles);
}


fn soft_intr_test() {
    init_utrap_handler();
    let hart_id = 0;
    let lq0 = TAIC.alloc_lq(1, 1).unwrap();
    LQ.init_by(lq0);
    LQ.whart(hart_id);
    LQ.register_receiver(1, 5, 0x109);

    let lq1 = TAIC.alloc_lq(1, 5).unwrap();
    lq1.register_sender(1, 1);
    lq1.send_intr(1, 1);
    debug_println!("Send softintr latencies end");

    // debug_println!("Send softintr latencys: {:?}", send_softintr_latency);
    // for _ in 0..NUM {
    //     lq1.send_intr(1, 2);
    // }

}

pub fn taic_test(boot_info: &BootInfo) {
    mmap::mmap_device_addr(boot_info, TAIC_BASE, TAIC_LEN, TAIC_OFFSET);
    enq_deq_test();
    // soft_intr_test();
    loop {
        
    }
}

pub fn taic_init(boot_info: &BootInfo) {
    mmap::mmap_device_addr(boot_info, TAIC_BASE, TAIC_LEN, TAIC_OFFSET);
}