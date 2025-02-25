#[doc = "Register `task_dequeue` reader"]
pub type R = crate::R<TaskDequeueSpec>;
#[doc = "Field `is_preempt` reader - The preempt attribute about task."]
pub type IsPreemptR = crate::BitReader;
#[doc = "Field `priority` reader - The priority attribute about task."]
pub type PriorityR = crate::FieldReader;
#[doc = "Field `tcb` reader - The pointer of task control block."]
pub type TcbR = crate::FieldReader<u64>;
impl R {
    #[doc = "Bit 0 - The preempt attribute about task."]
    #[inline(always)]
    pub fn is_preempt(&self) -> IsPreemptR {
        IsPreemptR::new((self.bits & 1) != 0)
    }
    #[doc = "Bits 1:5 - The priority attribute about task."]
    #[inline(always)]
    pub fn priority(&self) -> PriorityR {
        PriorityR::new(((self.bits >> 1) & 0x1f) as u8)
    }
    #[doc = "Bits 6:64 - The pointer of task control block."]
    #[inline(always)]
    pub fn tcb(&self) -> TcbR {
        TcbR::new((self.bits >> 6) & 0x07ff_ffff_ffff_ffff)
    }
}
#[doc = "Fetch a task from the local ready queue.\n\nYou can [`read`](crate::Reg::read) this register and get [`task_dequeue::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct TaskDequeueSpec;
impl crate::RegisterSpec for TaskDequeueSpec {
    type Ux = u64;
}
#[doc = "`read()` method returns [`task_dequeue::R`](R) reader structure"]
impl crate::Readable for TaskDequeueSpec {}
#[doc = "`reset()` method sets task_dequeue to value 0"]
impl crate::Resettable for TaskDequeueSpec {
    const RESET_VALUE: u64 = 0;
}
