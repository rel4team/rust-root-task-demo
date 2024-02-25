use alloc::collections::BTreeMap;
use async_runtime::{coroutine_get_current, coroutine_wake, CoroutineId, IPCItem, NewBuffer};
use async_runtime::utils::{IndexAllocator, yield_now};
use sel4::{CPtrBits, MessageInfo, Notification};
use sel4_root_task::debug_println;
use uintr::{register_sender, uipi_send};

pub const MAX_UINT_VEC: usize = 64;

#[thread_local]
static mut UINT_VEC_ALLOCATOR: IndexAllocator<MAX_UINT_VEC> = IndexAllocator::new();


pub type SenderID = u64;
#[thread_local]
static mut SENDER_MAP: BTreeMap<SenderID, &'static mut NewBuffer> = BTreeMap::new();

pub type UIntVec = usize;

#[thread_local]
static mut WAKE_MAP: BTreeMap<UIntVec, CoroutineId> = BTreeMap::new();

pub fn register_recv_cid(cid: &CoroutineId) -> Option<UIntVec> {
    unsafe {
        if let Some(vec) = UINT_VEC_ALLOCATOR.allocate() {
            WAKE_MAP.insert(vec, *cid);
            return Some(vec);
        }
        return None;
    }
}

pub fn register_sender_buffer(ntfn: Notification, new_buffer: &'static mut NewBuffer) -> Result<SenderID, ()> {
    if let Ok(sender_id) = register_sender(ntfn) {
        unsafe { SENDER_MAP.insert(sender_id, new_buffer); }
        return Ok(sender_id);
    }
    return Err(());
}

pub fn wake_recv_coroutine(vec: usize) -> Result<(), ()> {
    unsafe {
        if let Some(cid) = WAKE_MAP.get(&vec) {
            coroutine_wake(cid);
            return Ok(());
        }
        return Err(())
    }
}

pub struct AsyncArgs {
    pub req_ntfn: Option<CPtrBits>,
    pub reply_ntfn: Option<CPtrBits>,
    pub server_sender_id: Option<SenderID>,
    pub client_sender_id: Option<SenderID>,
    pub child_tcb: Option<CPtrBits>,
    pub ipc_new_buffer: Option<&'static mut NewBuffer>,
    pub server_ready: bool,
}

impl AsyncArgs {
    pub fn new() -> Self {
        Self {
            req_ntfn: None,
            reply_ntfn: None,
            server_sender_id: None,
            client_sender_id: None,
            child_tcb: None,
            ipc_new_buffer: None,
            server_ready: false,
        }
    }

    #[inline]
    pub fn get_ptr(&self) -> usize {
        self as *const Self as usize
    }

    #[inline]
    pub fn from_ptr(ptr: usize) -> &'static mut Self {
        unsafe {
            &mut *(ptr as *mut Self)
        }
    }
}


pub async fn seL4_Call(sender_id: &SenderID, message_info: MessageInfo) -> Result<MessageInfo, ()> {
    if let Some(new_buffer) = unsafe { SENDER_MAP.get_mut(sender_id) } {
        let cid = coroutine_get_current();
        let req_item = IPCItem::from(cid, message_info.inner().0.inner()[0]);
        new_buffer.req_items.write_free_item(&req_item)?;
        if new_buffer.recv_req_status == false {
            new_buffer.recv_req_status = true;
            unsafe {
                uipi_send(*sender_id);
            }
        }
        if let Some(res) = yield_now().await {
            let mut reply = MessageInfo::new(0, 0, 0, 0);
            reply.inner_mut().0.inner_mut()[0] = res;
            return Ok(reply);
        }
    }
    Err(())
}