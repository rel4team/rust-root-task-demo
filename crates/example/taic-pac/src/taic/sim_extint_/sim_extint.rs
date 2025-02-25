#[doc = "Register `sim_extint` writer"]
pub type W = crate::W<SimExtintSpec>;
impl core::fmt::Debug for crate::generic::Reg<SimExtintSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Simulate an external interrupt.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`sim_extint::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct SimExtintSpec;
impl crate::RegisterSpec for SimExtintSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`sim_extint::W`](W) writer structure"]
impl crate::Writable for SimExtintSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets sim_extint to value 0"]
impl crate::Resettable for SimExtintSpec {
    const RESET_VALUE: u64 = 0;
}
