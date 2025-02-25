use lazy_init::LazyInit;
use core::arch::global_asm;
use alloc::boxed::Box;
use riscv::register::{
    ucause, uepc, uie, uip,
    ustatus::{self, Ustatus},
    utval,
    utvec::{self, TrapMode},
};
use sel4_root_task::debug_println;

global_asm!(include_str!("utrap.asm"));

extern "C" {
    fn __alltraps_u();
}

/// `Interrupt` bit in `ucause`
pub(super) const INTC_IRQ_BASE: usize = 1 << (usize::BITS - 1);

/// User software interrupt in `ucause`
#[allow(unused)]
pub(super) const U_SOFT: usize = INTC_IRQ_BASE + 0;

/// User timer interrupt in `ucause`
pub(super) const U_TIMER: usize = INTC_IRQ_BASE + 4;

/// User external interrupt in `ucause`
pub(super) const U_EXT: usize = INTC_IRQ_BASE + 8;

type Handler = Box<dyn Fn()>;

#[thread_local]
static mut SOFT_HANDLER: Option<Handler> = None;

pub struct UserTrapContext {
    pub x: [usize; 32],
    pub ustatus: Ustatus,
    pub uepc: usize,
    pub utvec: usize,
}

pub fn init_utrap_handler() {
    unsafe {
        utvec::write(__alltraps_u as usize, TrapMode::Direct);
        ustatus::set_uie();
        uie::set_usoft();
    }
}

#[no_mangle]
pub fn user_trap_handler(cx: &mut UserTrapContext) -> &mut UserTrapContext {
    let ucause = ucause::read();
    let utval = utval::read();
    match ucause.cause() {
        ucause::Trap::Interrupt(ucause::Interrupt::UserSoft) => unsafe {
            uip::clear_usoft();
            dispatch_irq(ucause.bits());
        },
        _ => {
            debug_println!(
                "Unsupported trap {:?}, utval = {:#x}, uepc = {:#x}!",
                ucause.cause(),
                utval,
                uepc::read()
            );
        }
    }
    cx
}

macro_rules! with_cause {
    ($cause: expr, @TIMER => $timer_op: expr, @EXT => $ext_op: expr, @SOFT => $soft_op: expr $(,)?) => {
        match $cause {
            U_TIMER => $timer_op,
            U_EXT => $ext_op,
            U_SOFT => $soft_op,
            _ => panic!("invalid trap cause: {:#x}", $cause),
        }
    };
}


pub fn register_usoft_handler(handler: Handler) {
    unsafe {
        debug_println!("register_usoft_handler");
        SOFT_HANDLER = Some(handler);
    }
}

pub fn dispatch_irq(ucause: usize) {
    with_cause!(
        ucause,
        @TIMER => {
            debug_println!("IRQ: timer");
            // TIMER_HANDLER();
        },
        @EXT => {
            // TODO: get IRQ number from PLIC
            debug_println!("IRQ: ext");
        },
        @SOFT => {
            unsafe {
                if let Some(handler) = &SOFT_HANDLER {
                    handler();
                }
            }
        },
    );
}
