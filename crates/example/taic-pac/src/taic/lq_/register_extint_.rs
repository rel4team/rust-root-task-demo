#[repr(C)]
#[doc = "Register about external interrupt"]
#[doc(alias = "register_extint_")]
pub struct RegisterExtint_ {
    register_extint: RegisterExtint,
}
impl RegisterExtint_ {
    #[doc = "0x00..0x08 - Register external interrupt handler."]
    #[inline(always)]
    pub const fn register_extint(&self) -> &RegisterExtint {
        &self.register_extint
    }
}
#[doc = "register_extint (w) register accessor: Register external interrupt handler.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`register_extint::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@register_extint`]
module"]
#[doc(alias = "register_extint")]
pub type RegisterExtint = crate::Reg<register_extint::RegisterExtintSpec>;
#[doc = "Register external interrupt handler."]
pub mod register_extint;
