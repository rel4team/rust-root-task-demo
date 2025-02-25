#[repr(C)]
#[doc = "Simulate an external interrupt"]
#[doc(alias = "sim_extint_")]
pub struct SimExtint_ {
    sim_extint: SimExtint,
}
impl SimExtint_ {
    #[doc = "0x00..0x08 - Simulate an external interrupt."]
    #[inline(always)]
    pub const fn sim_extint(&self) -> &SimExtint {
        &self.sim_extint
    }
}
#[doc = "sim_extint (w) register accessor: Simulate an external interrupt.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`sim_extint::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@sim_extint`]
module"]
#[doc(alias = "sim_extint")]
pub type SimExtint = crate::Reg<sim_extint::SimExtintSpec>;
#[doc = "Simulate an external interrupt."]
pub mod sim_extint;
