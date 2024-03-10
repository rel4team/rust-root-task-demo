use alloc::collections::BTreeMap;
use core::sync::atomic::Ordering::SeqCst;
use async_runtime::{coroutine_get_current, coroutine_wake, coroutine_wake_with_value, CoroutineId, IPCItem, NewBuffer};
use async_runtime::utils::{IndexAllocator, yield_now};
use sel4::{CPtr, CPtrBits, MessageInfo, Notification};
use sel4::sys::invocation_label;
use sel4::ObjectBlueprint;
use sel4::get_clock;
use uintr::{register_sender, uintr_frame, uipi_send};

pub const MAX_UINT_VEC: usize = 64;

#[thread_local]
static mut UINT_VEC_ALLOCATOR: IndexAllocator<MAX_UINT_VEC> = IndexAllocator::new();


pub type SenderID = i64;
#[thread_local]
static mut SENDER_MAP: [usize; 64] = [0; 64];
// static mut SENDER_MAP: BTreeMap<SenderID, &'static mut NewBuffer> = BTreeMap::new();

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
        // unsafe { SENDER_MAP.insert(sender_id as SenderID, new_buffer); }
        unsafe {
            SENDER_MAP[sender_id as usize] = new_buffer as *const NewBuffer as usize;
        }
        return Ok(sender_id as SenderID);
    }
    return Err(());
}

pub fn register_async_syscall_buffer(new_buffer: &'static mut NewBuffer) {
    // unsafe { SENDER_MAP.insert(-1 as SenderID, new_buffer); }
    unsafe {
        SENDER_MAP[63] = new_buffer as *const NewBuffer as usize;
    }
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


#[inline]
pub async fn seL4_Call(sender_id: &SenderID, message_info: MessageInfo) -> Result<MessageInfo, ()> {
    let req_item = IPCItem::from(coroutine_get_current(), message_info.inner().0.inner()[0] as u32);
    seL4_Call_with_item(sender_id, &req_item).await
}


pub async fn recv_reply_coroutine(arg: usize, reply_num: usize) {
    // let cid = coroutine_get_current();
    static mut REPLY_COUNT: usize = 0;
    let async_args = AsyncArgs::from_ptr(arg);
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    loop {
        if let Some(mut item) = new_buffer.res_items.get_first_item() {
            // debug_println!("recv req: {:?}", item);
            coroutine_wake_with_value(&item.cid, item.msg_info as u64);
            unsafe {
                REPLY_COUNT += 1;
                if REPLY_COUNT == reply_num {
                    break;
                }
            }
        } else {
            new_buffer.recv_reply_status.store(false, SeqCst);
            // coroutine_wake(&cid);
            yield_now().await;
        }
    }
}

pub fn uintr_handler(frame: *mut uintr_frame, irqs: usize) -> usize {
    // debug_println!("Hello, uintr_handler!: {}, exec_ptr: {:#x}", irqs, get_executor_ptr());
    let mut local = irqs;
    let mut bit_index = 0;
    while local != 0 {
        if local & 1 == 1 {
            wake_recv_coroutine(bit_index).unwrap();
        }
        local >>= 1;
        bit_index += 1;
    }

    return 0;
}

#[inline]
fn convert_option_mut_ref<T>(ptr: usize) -> Option<&'static mut T> {
    if ptr == 0 {
        return None;
    }
    return Some(unsafe {
        &mut *(ptr as *mut T)
    })
}

pub async fn seL4_Call_with_item(sender_id: &SenderID, item: &IPCItem) -> Result<MessageInfo, ()> {
    // let start = get_clock();
    if let Some(new_buffer) = unsafe { convert_option_mut_ref::<NewBuffer>(SENDER_MAP[*sender_id as usize]) } {
        new_buffer.req_items.write_free_item(&item)?;
        // debug_println!("seL4_Call_with_item: {}", get_clock() - start);
        if new_buffer.recv_req_status.load(SeqCst) == false {
            new_buffer.recv_req_status.store(true, SeqCst);
            if *sender_id != -1 {
                // debug_println!("send uipi");
                unsafe {
                    uipi_send(*sender_id as u64);
                }
            } else {
                // todo: submit syscall
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

pub async fn seL4_Untyped_Retype(service: CPtr,
                                 r#type: ObjectBlueprint,
                                 size_bits: usize,
                                 root: CPtr,
                                 node_index: usize,
                                 node_depth: usize,
                                 node_offset: usize,
                                 num_objects: usize

) -> Result<MessageInfo, ()> {
    let sender_id = -1;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = invocation_label::UntypedRetype.into();
    syscall_item.extend_msg[0] = service.bits() as u16;
    syscall_item.extend_msg[1] = r#type.ty().into_sys() as u16;
    syscall_item.extend_msg[2] = size_bits as u16;
    syscall_item.extend_msg[3] = root.bits() as u16;
    syscall_item.extend_msg[4] = node_index as u16;
    syscall_item.extend_msg[5] = node_depth as u16;
    syscall_item.extend_msg[6] = node_offset as u16;
    syscall_item.extend_msg[7] = num_objects as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await
}