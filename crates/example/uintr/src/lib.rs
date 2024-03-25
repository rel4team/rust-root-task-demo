#![no_std]

use core::arch::global_asm;
use sel4::{TCB, Notification, Error};
use sel4::with_ipc_buffer;


global_asm!(include_str!("uintr.asm"));

/* User Trap Setup */
pub const CSR_USTATUS: usize = 0x000;
pub const CSR_UIE: usize = 0x004;
pub const CSR_UTVEC: usize =0x005;

/* User Trap Handling */
pub const CSR_USCRATCH: usize = 0x040;
pub const CSR_UEPC: usize = 0x041;
pub const CSR_UCAUSE: usize = 0x042;
pub const CSR_UTVAL: usize = 0x043;
pub const CSR_UIP: usize = 0x044;

/* ustatus CSR bits */
pub const USTATUS_UIE: usize = 0x00000001;
pub const USTATUS_UPIE: usize = 0x00000010;

pub const IRQ_U_SOFT: usize = 0;
pub const IRQ_U_TIMER: usize = 4;
pub const IRQ_U_EXT: usize = 8;

pub const MIE_USIE: usize = 1 << IRQ_U_SOFT;
pub const MIE_UTIE: usize = 1 << IRQ_U_TIMER;
pub const MIE_UEIE: usize = 1 << IRQ_U_EXT;


pub unsafe fn uipi_send(index: u64) {
    core::arch::asm!(".insn r 0b1111011, 0b110, 0b0000000, x0, {}, x0", in(reg) index);
}
pub unsafe fn uipi_read() -> usize {
    let mut ret: usize = 0;
    core::arch::asm!(".insn r 0b1111011, 0b110, 0b0000001, {}, x0, x0", out(reg) ret);
    ret
}

pub unsafe fn uipi_write(bits: usize) {
    core::arch::asm!(".insn r 0b1111011, 0b110, 0b0000010, x0, {}, x0", in(reg) bits);
}

// pub unsafe fn uipi_activate() {
//     core::arch::asm!(".insn i 0b1111011, 0b010, x0, x0, 0x3");
// }
//
// pub unsafe fn uipi_deactivate() {
//     core::arch::asm!(".insn i 0b1111011, 0b010, x0, x0, 0x4");
// }

pub struct uintr_frame {
    ra: u64, sp: u64, gp: u64, tp: u64,
    t0: u64, t1: u64, t2: u64, s0: u64,
    s1: u64, a0: u64, a1: u64, a2: u64,
    a3: u64, a4: u64, a5: u64, a6: u64,
    a7: u64, s2: u64, s3: u64, s4: u64,
    s5: u64, s6: u64, s7: u64, s8: u64,
    s9: u64, s10: u64, s11: u64, t3: u64,
    t4: u64, t5: u64, t6: u64,
}


#[inline]
#[allow(unused_variables)]
unsafe fn clear_csr_uip(bits: usize) {
    core::arch::asm!(concat!("csrc ", "0x044", ", {0}"), in(reg) bits);
}

#[no_mangle]
pub unsafe fn __handler_entry(frame: *mut uintr_frame, handler: u64) {
    // sel4::debug_println!("__handler_entry enter");
    let irqs = uipi_read();
    // sel4::debug_println!("__handler_entry enter2");
    clear_csr_uip(MIE_USIE);
    let handler_func: fn(*mut uintr_frame, usize) -> usize = core::mem::transmute(handler);
    let irqs = handler_func(frame, irqs);
    uipi_write(irqs);
}

pub fn register_receiver(tcb: TCB, ntfn: Notification, handler: usize) -> Result<(), Error> {
    extern "C" {
        fn uintrvec();
    }
    unsafe {
        core::arch::asm!(concat!("csrw ", "0x005", ", {0}"), in(reg) uintrvec as usize);
        core::arch::asm!(concat!("csrw ", "0x040", ", {0}"), in(reg) handler);
        core::arch::asm!(concat!("csrs ", "0x000", ", {0}"), in(reg) USTATUS_UIE);
        core::arch::asm!(concat!("csrs ", "0x004", ", {0}"), in(reg) MIE_USIE);
    }
    return ntfn.register_receiver(tcb.cptr());
}

pub fn register_sender(ntfn: Notification) -> Result<u64, Error> {
    // sel4::debug_println!("register_sender");
    ntfn.register_sender()?;
    Ok(with_ipc_buffer(|buffer| {
        buffer.inner().uintr_flag
        // sel4::debug_println!("buffer ptr: {:#x}", buffer as *const IPCBuffer as usize);
        // a
    }))
    // Ok(1)
}