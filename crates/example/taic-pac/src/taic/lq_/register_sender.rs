#[doc = "Register `register_sender` writer"]
pub type W = crate::W<RegisterSenderSpec>;
impl core::fmt::Debug for crate::generic::Reg<RegisterSenderSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Register send capability.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`register_sender::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct RegisterSenderSpec;
impl crate::RegisterSpec for RegisterSenderSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`register_sender::W`](W) writer structure"]
impl crate::Writable for RegisterSenderSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets register_sender to value 0"]
impl crate::Resettable for RegisterSenderSpec {
    const RESET_VALUE: u64 = 0;
}
