#[doc = "Register `whart` writer"]
pub type W = crate::W<WhartSpec>;
impl core::fmt::Debug for crate::generic::Reg<WhartSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Write the hartid about the local queue.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`whart::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct WhartSpec;
impl crate::RegisterSpec for WhartSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`whart::W`](W) writer structure"]
impl crate::Writable for WhartSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets whart to value 0"]
impl crate::Resettable for WhartSpec {
    const RESET_VALUE: u64 = 0;
}
