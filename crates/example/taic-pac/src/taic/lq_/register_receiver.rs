#[doc = "Register `register_receiver` writer"]
pub type W = crate::W<RegisterReceiverSpec>;
impl core::fmt::Debug for crate::generic::Reg<RegisterReceiverSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Register receive capability.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`register_receiver::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct RegisterReceiverSpec;
impl crate::RegisterSpec for RegisterReceiverSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`register_receiver::W`](W) writer structure"]
impl crate::Writable for RegisterReceiverSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets register_receiver to value 0"]
impl crate::Resettable for RegisterReceiverSpec {
    const RESET_VALUE: u64 = 0;
}
