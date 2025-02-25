#[doc = "Register `register_extint` writer"]
pub type W = crate::W<RegisterExtintSpec>;
impl core::fmt::Debug for crate::generic::Reg<RegisterExtintSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Register external interrupt handler.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`register_extint::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct RegisterExtintSpec;
impl crate::RegisterSpec for RegisterExtintSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`register_extint::W`](W) writer structure"]
impl crate::Writable for RegisterExtintSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets register_extint to value 0"]
impl crate::Resettable for RegisterExtintSpec {
    const RESET_VALUE: u64 = 0;
}
