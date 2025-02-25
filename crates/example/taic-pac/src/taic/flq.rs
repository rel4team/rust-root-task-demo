#[doc = "Register `flq` writer"]
pub type W = crate::W<FlqSpec>;
impl core::fmt::Debug for crate::generic::Reg<FlqSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Free a local ready queue.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`flq::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct FlqSpec;
impl crate::RegisterSpec for FlqSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`flq::W`](W) writer structure"]
impl crate::Writable for FlqSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets flq to value 0"]
impl crate::Resettable for FlqSpec {
    const RESET_VALUE: u64 = 0;
}
