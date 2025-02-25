#[doc = "Register `cancel_sender` writer"]
pub type W = crate::W<CancelSenderSpec>;
impl core::fmt::Debug for crate::generic::Reg<CancelSenderSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Cancel send capability.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`cancel_sender::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct CancelSenderSpec;
impl crate::RegisterSpec for CancelSenderSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`cancel_sender::W`](W) writer structure"]
impl crate::Writable for CancelSenderSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets cancel_sender to value 0"]
impl crate::Resettable for CancelSenderSpec {
    const RESET_VALUE: u64 = 0;
}
