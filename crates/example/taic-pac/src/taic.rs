#[repr(C)]
#[doc = "Register block"]
pub struct RegisterBlock {
    alq: Alq,
    flq: Flq,
    sim_extint: [SimExtint_; 256],
    _reserved3: [u8; 0x07f0],
    lqs: (),
}
impl RegisterBlock {
    #[doc = "0x00..0x08 - Alloc a local ready queue."]
    #[inline(always)]
    pub const fn alq(&self) -> &Alq {
        &self.alq
    }
    #[doc = "0x08..0x10 - Free a local ready queue."]
    #[inline(always)]
    pub const fn flq(&self) -> &Flq {
        &self.flq
    }
    #[doc = "0x10..0x810 - Simulate an external interrupt"]
    #[inline(always)]
    pub const fn sim_extint(&self, n: usize) -> &SimExtint_ {
        &self.sim_extint[n]
    }
    #[doc = "Iterator for array of:"]
    #[doc = "0x10..0x810 - Simulate an external interrupt"]
    #[inline(always)]
    pub fn sim_extint_iter(&self) -> impl Iterator<Item = &SimExtint_> {
        self.sim_extint.iter()
    }
    #[doc = "0x1000..0x85000 - Related registers space of per local queue"]
    #[inline(always)]
    pub const fn lqs(&self, n: usize) -> &Lq_ {
        #[allow(clippy::no_effect)]
        [(); 256][n];
        unsafe {
            &*core::ptr::from_ref(self)
                .cast::<u8>()
                .add(4096)
                .add(4096 * n)
                .cast()
        }
    }
    #[doc = "Iterator for array of:"]
    #[doc = "0x1000..0x85000 - Related registers space of per local queue"]
    #[inline(always)]
    pub fn lqs_iter(&self) -> impl Iterator<Item = &Lq_> {
        (0..256).map(move |n| unsafe {
            &*core::ptr::from_ref(self)
                .cast::<u8>()
                .add(4096)
                .add(4096 * n)
                .cast()
        })
    }
}
#[doc = "alq (rw) register accessor: Alloc a local ready queue.\n\nYou can [`read`](crate::Reg::read) this register and get [`alq::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`alq::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@alq`]
module"]
#[doc(alias = "alq")]
pub type Alq = crate::Reg<alq::AlqSpec>;
#[doc = "Alloc a local ready queue."]
pub mod alq;
#[doc = "flq (w) register accessor: Free a local ready queue.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`flq::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@flq`]
module"]
#[doc(alias = "flq")]
pub type Flq = crate::Reg<flq::FlqSpec>;
#[doc = "Free a local ready queue."]
pub mod flq;
#[doc = "Simulate an external interrupt"]
pub use self::sim_extint_::SimExtint_;
#[doc = r"Cluster"]
#[doc = "Simulate an external interrupt"]
pub mod sim_extint_;
#[doc = "Related registers space of per local queue"]
pub use self::lq_::Lq_;
#[doc = r"Cluster"]
#[doc = "Related registers space of per local queue"]
pub mod lq_;
