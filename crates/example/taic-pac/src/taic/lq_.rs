#[repr(C)]
#[doc = "Related registers space of per local queue"]
#[doc(alias = "lq_")]
pub struct Lq_ {
    task_enqueue: TaskEnqueue,
    task_dequeue: TaskDequeue,
    _reserved2: [u8; 0x08],
    register_sender: RegisterSender,
    cancel_sender: CancelSender,
    register_receiver: RegisterReceiver,
    send_intr: SendIntr,
    whart: Whart,
    register_extint: [RegisterExtint_; 256],
}
impl Lq_ {
    #[doc = "0x00..0x08 - Add task into the local ready queue."]
    #[inline(always)]
    pub const fn task_enqueue(&self) -> &TaskEnqueue {
        &self.task_enqueue
    }
    #[doc = "0x08..0x10 - Fetch a task from the local ready queue."]
    #[inline(always)]
    pub const fn task_dequeue(&self) -> &TaskDequeue {
        &self.task_dequeue
    }
    #[doc = "0x18..0x20 - Register send capability."]
    #[inline(always)]
    pub const fn register_sender(&self) -> &RegisterSender {
        &self.register_sender
    }
    #[doc = "0x20..0x28 - Cancel send capability."]
    #[inline(always)]
    pub const fn cancel_sender(&self) -> &CancelSender {
        &self.cancel_sender
    }
    #[doc = "0x28..0x30 - Register receive capability."]
    #[inline(always)]
    pub const fn register_receiver(&self) -> &RegisterReceiver {
        &self.register_receiver
    }
    #[doc = "0x30..0x38 - Send interrupt to receiver."]
    #[inline(always)]
    pub const fn send_intr(&self) -> &SendIntr {
        &self.send_intr
    }
    #[doc = "0x38..0x40 - Write the hartid about the local queue."]
    #[inline(always)]
    pub const fn whart(&self) -> &Whart {
        &self.whart
    }
    #[doc = "0x40..0x840 - Register about external interrupt"]
    #[inline(always)]
    pub const fn register_extint(&self, n: usize) -> &RegisterExtint_ {
        &self.register_extint[n]
    }
    #[doc = "Iterator for array of:"]
    #[doc = "0x40..0x840 - Register about external interrupt"]
    #[inline(always)]
    pub fn register_extint_iter(&self) -> impl Iterator<Item = &RegisterExtint_> {
        self.register_extint.iter()
    }
}
#[doc = "task_enqueue (w) register accessor: Add task into the local ready queue.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`task_enqueue::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@task_enqueue`]
module"]
#[doc(alias = "task_enqueue")]
pub type TaskEnqueue = crate::Reg<task_enqueue::TaskEnqueueSpec>;
#[doc = "Add task into the local ready queue."]
pub mod task_enqueue;
#[doc = "task_dequeue (r) register accessor: Fetch a task from the local ready queue.\n\nYou can [`read`](crate::Reg::read) this register and get [`task_dequeue::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@task_dequeue`]
module"]
#[doc(alias = "task_dequeue")]
pub type TaskDequeue = crate::Reg<task_dequeue::TaskDequeueSpec>;
#[doc = "Fetch a task from the local ready queue."]
pub mod task_dequeue;
#[doc = "register_sender (w) register accessor: Register send capability.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`register_sender::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@register_sender`]
module"]
#[doc(alias = "register_sender")]
pub type RegisterSender = crate::Reg<register_sender::RegisterSenderSpec>;
#[doc = "Register send capability."]
pub mod register_sender;
#[doc = "cancel_sender (w) register accessor: Cancel send capability.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`cancel_sender::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@cancel_sender`]
module"]
#[doc(alias = "cancel_sender")]
pub type CancelSender = crate::Reg<cancel_sender::CancelSenderSpec>;
#[doc = "Cancel send capability."]
pub mod cancel_sender;
#[doc = "register_receiver (w) register accessor: Register receive capability.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`register_receiver::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@register_receiver`]
module"]
#[doc(alias = "register_receiver")]
pub type RegisterReceiver = crate::Reg<register_receiver::RegisterReceiverSpec>;
#[doc = "Register receive capability."]
pub mod register_receiver;
#[doc = "send_intr (w) register accessor: Send interrupt to receiver.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`send_intr::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@send_intr`]
module"]
#[doc(alias = "send_intr")]
pub type SendIntr = crate::Reg<send_intr::SendIntrSpec>;
#[doc = "Send interrupt to receiver."]
pub mod send_intr;
#[doc = "whart (w) register accessor: Write the hartid about the local queue.\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`whart::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@whart`]
module"]
#[doc(alias = "whart")]
pub type Whart = crate::Reg<whart::WhartSpec>;
#[doc = "Write the hartid about the local queue."]
pub mod whart;
#[doc = "Register about external interrupt"]
pub use self::register_extint_::RegisterExtint_;
#[doc = r"Cluster"]
#[doc = "Register about external interrupt"]
pub mod register_extint_;
