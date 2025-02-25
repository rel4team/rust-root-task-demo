#[doc = "Register `send_intr` writer"]
pub type W = crate::W<SendIntrSpec>;
impl core::fmt::Debug for crate::generic::Reg<SendIntrSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Send interrupt to receiver.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`send_intr::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct SendIntrSpec;
impl crate::RegisterSpec for SendIntrSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`send_intr::W`](W) writer structure"]
impl crate::Writable for SendIntrSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets send_intr to value 0"]
impl crate::Resettable for SendIntrSpec {
    const RESET_VALUE: u64 = 0;
}
