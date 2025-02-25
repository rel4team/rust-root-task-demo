#[doc = "Register `task_enqueue` writer"]
pub type W = crate::W<TaskEnqueueSpec>;
impl core::fmt::Debug for crate::generic::Reg<TaskEnqueueSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Add task into the local ready queue.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`task_enqueue::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct TaskEnqueueSpec;
impl crate::RegisterSpec for TaskEnqueueSpec {
    type Ux = u64;
}
#[doc = "`write(|w| ..)` method takes [`task_enqueue::W`](W) writer structure"]
impl crate::Writable for TaskEnqueueSpec {
    type Safety = crate::Unsafe;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: u64 = 0;
}
#[doc = "`reset()` method sets task_enqueue to value 0"]
impl crate::Resettable for TaskEnqueueSpec {
    const RESET_VALUE: u64 = 0;
}
