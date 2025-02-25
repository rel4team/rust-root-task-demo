#[doc = "Register `alq` reader"]
pub type R = crate::R<AlqSpec>;
#[doc = "Register `alq` writer"]
pub type W = crate::W<AlqSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "Alloc a local ready queue.\n\nYou can [`read`](crate::Reg::read) this register and get [`alq::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`alq::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct AlqSpec;
impl crate::RegisterSpec for AlqSpec {
    type Ux = u64;
}
#[doc = "`read()` method returns [`alq::R`](R) reader structure"]
impl crate::Readable for AlqSpec {}
#[doc = "`write(|w| ..)` method takes [`alq::W`](W) writer structure"]
impl crate::Writable for AlqSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets alq to value 0"]
impl crate::Resettable for AlqSpec {
    const RESET_VALUE: u64 = 0;
}
