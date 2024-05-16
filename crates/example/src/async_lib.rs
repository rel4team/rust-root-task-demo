use alloc::collections::BTreeMap;
use sel4_logging::log::debug;
use sel4_root_task::debug_println;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::Ordering::SeqCst;
use core::task::{Context, Poll};
use async_runtime::{coroutine_delay_wake, coroutine_get_current, coroutine_possible_switch, coroutine_wake, AsyncMessageLabel, CoroutineId, IPCItem, NewBuffer, MAX_TASK_NUM};
use async_runtime::utils::{IndexAllocator};
use sel4::{CPtr, CPtrBits, CapRights, LocalCPtr, MessageInfo, Notification, TCB};
use sel4::sys::invocation_label;
use sel4::ObjectBlueprint;
use sel4::get_clock;
use sel4::wake_syscall_handler;
use uintr::{register_sender, uintr_frame, uipi_send};

use crate::image_utils::UserImageUtils;

pub const MAX_UINT_VEC: usize = 64;

#[thread_local]
static mut UINT_VEC_ALLOCATOR: IndexAllocator<MAX_UINT_VEC> = IndexAllocator::new();

#[thread_local]
pub static mut UINT_TRIGGER: usize = 0;

pub type SenderID = i64;
#[thread_local]
static mut SENDER_MAP: [usize; 64] = [0; 64];
// static mut SENDER_MAP: BTreeMap<SenderID, &'static mut NewBuffer> = BTreeMap::new();

#[thread_local]
static mut IMMEDIATE_VALUE: [Option<IPCItem>; MAX_TASK_NUM] = [None; MAX_TASK_NUM];

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

// pub fn register_async_syscall_buffer(new_buffer: &'static mut NewBuffer) {
pub fn register_async_syscall_buffer(new_buffer_ptr: usize) {
    // unsafe { SENDER_MAP.insert(63 as SenderID, new_buffer); }
    unsafe {
        SENDER_MAP[63] = new_buffer_ptr;
    }
}

pub fn wake_recv_coroutine(vec: usize) -> Result<(), ()> {
    // sel4::debug_println!("Hello, wake_recv_coroutine!: {}", vec);
    unsafe {
        if let Some(cid) = WAKE_MAP.get(&vec) {
            coroutine_delay_wake(cid);
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
pub async fn yield_now() -> Option<IPCItem> {
    let helper = YieldHelper::new();
    helper.await;
    unsafe {
        IMMEDIATE_VALUE[coroutine_get_current().0 as usize].take()
    }
}

#[inline]
pub fn wake_with_value(cid: &CoroutineId, item: &IPCItem) {
    unsafe {
        IMMEDIATE_VALUE[cid.0 as usize] = Some(*item);
        coroutine_wake(&cid);
    }
}

#[inline]
pub async fn possible_switch() {
    if coroutine_possible_switch() {
        coroutine_wake(&coroutine_get_current());
        yield_now().await;
    }
}

struct YieldHelper(bool);

impl YieldHelper {
    pub fn new() -> Self {
        Self {
            0: false,
        }
    }
}

impl Future for YieldHelper {
    type Output = ();

    #[inline]
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 == false {
            self.0 = true;
            return Poll::Pending;
        }
        return Poll::Ready(());
    }
}



#[inline]
pub async fn seL4_Call(sender_id: &SenderID, mut message_info: MessageInfo) -> Result<MessageInfo, ()> {
    let req_item = IPCItem::from(coroutine_get_current(), message_info.inner().0.inner()[0] as u32);
    match seL4_Call_with_item(sender_id, &req_item).await {
        Ok(res) => {
            // let mut reply = MessageInfo::new(0, 0, 0, 0);
            message_info.inner_mut().0.inner_mut()[0] = res.msg_info as u64;
            Ok(message_info)
        }
        _ => {
            Err(())
        }
    }
}


pub async fn recv_reply_coroutine(arg: usize, reply_num: usize) {
    // let cid = coroutine_get_current();
    static mut REPLY_COUNT: usize = 0;
    let async_args = AsyncArgs::from_ptr(arg);
    let new_buffer = async_args.ipc_new_buffer.as_mut().unwrap();
    loop {
        if let Some(item) = new_buffer.res_items.get_first_item() {
            // debug_println!("recv req: {:?}", item);
            // coroutine_wake_with_value(&item.cid, item.msg_info as u64);
            unsafe {
                IMMEDIATE_VALUE[item.cid.0 as usize] = Some(item);
                coroutine_wake(&item.cid);
            }
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

pub async fn recv_reply_coroutine_async_syscall(new_buffer_ptr: usize, reply_num: usize) {
    // let cid = coroutine_get_current();
    static mut REPLY_COUNT: usize = 0;
    let new_buffer = NewBuffer::from_ptr(new_buffer_ptr);
    loop {
        if let Some(item) = new_buffer.res_items.get_first_item() {
            // debug_println!("recv req: {:?}", item);
            // coroutine_wake_with_value(&item.cid, item.msg_info as u64);
            // unsafe {
            //     IMMEDIATE_VALUE[item.cid.0 as usize] = Some(item);
            //     coroutine_wake(&item.cid);
            // }
            // debug_println!("recv_reply_coroutine_async_syscall: get item: {:?}", item);
            let label: AsyncMessageLabel = AsyncMessageLabel::from(item.msg_info);
            match label {
                AsyncMessageLabel::RISCVPageGetAddress => {
                    let mut paddr: usize = 0;
                    paddr = paddr + (item.extend_msg[1] as usize) << 48;
                    paddr = paddr + (item.extend_msg[2] as usize) << 32;
                    paddr = paddr + (item.extend_msg[3] as usize) << 16;
                    paddr = paddr + (item.extend_msg[4] as usize);
                    debug_println!("recv_reply_coroutine_async_syscall: async RISCVPageGetAddress get paddr: {:#x}", paddr);
                }
                _ => {
                }
            }
            wake_with_value(&item.cid, &item);
            unsafe {
                REPLY_COUNT += 1;
                // debug_println!("Reply count: {:?}", REPLY_COUNT);
                if REPLY_COUNT == reply_num {
                    break;
                }
            }
        } else {
            new_buffer.recv_reply_status.store(false, SeqCst);
            // coroutine_wake(&cid);
            yield_now().await;
            // debug_println!("wake");
        }
    }
}


pub fn uintr_handler(_frame: *mut uintr_frame, irqs: usize) -> usize {
    unsafe {
        UINT_TRIGGER += 1;
    }
    // sel4::debug_println!("Hello, uintr_handler!: {}", irqs);
    let mut local = irqs;
    let mut bit_index = 0;
    while local != 0 {
        if local & 1 == 1 {
            // sel4::debug_println!("Hello, uintr_handler!: {}", irqs);
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

pub static mut SUBMIT_SYSCALL_CNT: usize = 0;

pub async fn seL4_Call_with_item(sender_id: &SenderID, item: &IPCItem) -> Result<IPCItem, ()> {
    if let Some(new_buffer) = unsafe { convert_option_mut_ref::<NewBuffer>(SENDER_MAP[*sender_id as usize]) } {
        // todo: bugs need to fix
        let msg_info = item.msg_info;
        new_buffer.req_items.write_free_item(&item).unwrap();
        // debug_println!("seL4_Call_with_item: write item: {:?}", msg_info);
        if new_buffer.recv_req_status.load(SeqCst) == false {
            new_buffer.recv_req_status.store(true, SeqCst);
            if *sender_id != 63 {
                // debug_println!("send uipi");
                unsafe {
                    uipi_send(*sender_id as u64);
                }
            } else {
                // todo: submit syscall
                // debug_println!("seL4_Call_with_item: Submit Syscall!");
                unsafe {
                    SUBMIT_SYSCALL_CNT += 1;
                }
                wake_syscall_handler();
            }
        }

        if let Some(res) = yield_now().await {
            return Ok(res);
        }
    }
    Err(())
}

pub async fn seL4_Send_with_item(sender_id: &SenderID, item: &IPCItem) -> Result<IPCItem, ()> {
    // let start = get_clock();
    if let Some(new_buffer) = unsafe { convert_option_mut_ref::<NewBuffer>(SENDER_MAP[*sender_id as usize]) } {
        // todo: bugs need to fix
        let msg_info = item.msg_info;
        new_buffer.req_items.write_free_item(&item).unwrap();
        // debug_println!("seL4_Call_with_item: write item: {:?}", msg_info);
        if new_buffer.recv_req_status.load(SeqCst) == false {
            new_buffer.recv_req_status.store(true, SeqCst);
            if *sender_id != 63 {
                // debug_println!("send uipi");
                unsafe {
                    uipi_send(*sender_id as u64);
                }
            } else {
                // todo: submit syscall
                // debug_println!("seL4_Call_with_item: Submit Syscall!");
                wake_syscall_handler();
            }
        }
        // if let Some(res) = yield_now().await {
        //     return Ok(res);
        // }
    }
    Err(())
    // Ok(())
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
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::UntypedRetype.into();
    syscall_item.extend_msg[0] = service.bits() as u16;
    syscall_item.extend_msg[1] = r#type.ty().into_sys() as u16;
    syscall_item.extend_msg[2] = size_bits as u16;
    syscall_item.extend_msg[3] = root.bits() as u16;
    syscall_item.extend_msg[4] = node_index as u16;
    syscall_item.extend_msg[5] = node_depth as u16;
    syscall_item.extend_msg[6] = node_offset as u16;
    syscall_item.extend_msg[7] = num_objects as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_Putchar(
    c: u16
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::PutChar.into();
    syscall_item.extend_msg[0] = c;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_Putstring(
    data: &[u16]
) -> Result<MessageInfo, ()> {
    let cid = coroutine_get_current();
    let length = data.len();
    // debug_println!("reL4_Putstring: length: {:?}", length);
    let round = length / 7;
    for i in 0..=round {
        let sender_id = 63;
        let mut syscall_item = IPCItem::new();
        syscall_item.cid = cid;
        syscall_item.msg_info = AsyncMessageLabel::PutString.into();
        let num = if i < round {
            7
        } else {
            length - 7 * i
        };
        syscall_item.extend_msg[0] = num as u16;
        // debug_println!("reL4_Putstring: num: {:?}", num);
        let offset = i * 7;
        for j in 0..num {
            syscall_item.extend_msg[j + 1] = data[offset + j];
        }
        seL4_Call_with_item(&sender_id, &syscall_item).await;              
    }      
    Err(())
}

pub async fn seL4_RISCV_Page_Get_Address(
    vaddr: usize
) -> Result<MessageInfo, ()> {
    let offset = vaddr % 4096;
    let new_vaddr = vaddr - offset;
    let frame_cap = UserImageUtils.get_user_image_frame_slot(new_vaddr);
    let frame = LocalCPtr::<sel4::cap_type::_4KPage>::from_bits(frame_cap as u64);
    // frame.frame_get_address().unwrap() + offset;
    let bits = frame.cptr().bits();
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::RISCVPageGetAddress.into();
    syscall_item.extend_msg[0] = bits as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_TCB_Bind_Notification(
    service: TCB,
    notification: Notification
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::TCBBindNotification.into();
    syscall_item.extend_msg[0] = service.bits() as u16;
    syscall_item.extend_msg[1] = notification.bits() as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_TCB_Unbind_Notification(
    service: TCB
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::TCBUnbindNotification.into();
    syscall_item.extend_msg[0] = service.bits() as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_CNode_Delete(
    service: CPtr,
    node_index: usize,
    node_depth: usize,
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::CNodeDelete.into();
    syscall_item.extend_msg[0] = service.bits() as u16;
    syscall_item.extend_msg[1] = node_index as u16;
    syscall_item.extend_msg[2] = node_depth as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_CNode_Copy(
    dest_root_cptr: CPtr,
    dest_index: usize,
    dest_depth: usize,
    src_root_cptr: CPtr,
    src_index: usize,
    src_depth: usize,
    cap_right: CapRights
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::CNodeCopy.into();
    syscall_item.extend_msg[0] = dest_root_cptr.bits() as u16;
    syscall_item.extend_msg[1] = dest_index as u16;
    syscall_item.extend_msg[2] = dest_depth as u16;
    syscall_item.extend_msg[3] = src_root_cptr.bits() as u16;
    syscall_item.extend_msg[4] = src_index as u16;
    syscall_item.extend_msg[5] = src_depth as u16;
    syscall_item.extend_msg[6] = cap_right.into_inner().0.inner()[0] as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_CNode_Mint(
    dest_root_cptr: CPtr,
    dest_index: usize,
    dest_depth: usize,
    src_root_cptr: CPtr,
    src_index: usize,
    src_depth: usize,
    cap_right: CapRights,
    badge: u64
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::CNodeCopy.into();
    syscall_item.extend_msg[0] = dest_root_cptr.bits() as u16;
    syscall_item.extend_msg[1] = dest_index as u16;
    syscall_item.extend_msg[2] = dest_depth as u16;
    syscall_item.extend_msg[3] = src_root_cptr.bits() as u16;
    syscall_item.extend_msg[4] = src_index as u16;
    syscall_item.extend_msg[5] = src_depth as u16;
    syscall_item.extend_msg[6] = cap_right.into_inner().0.inner()[0] as u16;
    syscall_item.extend_msg[7] = badge as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_RISCV_PageTable_Map(
    service_cptr: CPtr,
    vspace_cptr: CPtr,
    vaddr: usize,
    attrs: usize
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::RISCVPageTableMap.into();
    syscall_item.extend_msg[0] = service_cptr.bits() as u16;
    syscall_item.extend_msg[1] = vspace_cptr.bits() as u16;
    // debug_println!("seL4_RISCV_PageTable_Map: vaddr >> 12 = {:#x}", vaddr >> 12);
    syscall_item.extend_msg[2] = (vaddr >> 12) as u16;
    syscall_item.extend_msg[3] = attrs as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_RISCV_PageTable_Unmap(
    service_cptr: CPtr,
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::RISCVPageTableUnmap.into();
    syscall_item.extend_msg[0] = service_cptr.bits() as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_RISCV_Page_Map(
    service_cptr: CPtr,
    page_table_cptr: CPtr,
    vaddr: usize,
    rights: usize,
    attrs: usize
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    // debug_println!("seL4_RISCV_Page_Map: service: {:#x}, page_table: {:x}, vaddr: {:#x}, rights: {:?}, attrs: {:?}", service_cptr.bits(), page_table_cptr.bits(), vaddr, rights, attrs);
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::RISCVPageMap.into();
    syscall_item.extend_msg[0] = service_cptr.bits() as u16;
    syscall_item.extend_msg[1] = page_table_cptr.bits() as u16;
    syscall_item.extend_msg[2] = (vaddr >> 12) as u16;
    syscall_item.extend_msg[3] = rights as u16;
    syscall_item.extend_msg[4] = attrs as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}

pub async fn seL4_RISCV_Page_Unmap(
    service_cptr: CPtr,
) -> Result<MessageInfo, ()> {
    let sender_id = 63;
    let mut syscall_item = IPCItem::new();
    let cid = coroutine_get_current();
    syscall_item.cid = cid;
    syscall_item.msg_info = AsyncMessageLabel::RISCVPageUnmap.into();
    syscall_item.extend_msg[0] = service_cptr.bits() as u16;
    seL4_Call_with_item(&sender_id, &syscall_item).await;
    Err(())
}
