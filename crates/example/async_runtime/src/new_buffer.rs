use core::sync::atomic::AtomicBool;
use spin::Mutex;
use crate::coroutine::CoroutineId;
use sel4::get_clock;
use sel4::r#yield;
use crate::utils::RingBuffer;

pub const MAX_ITEM_NUM: usize = 4096;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct IPCItem {
    pub cid: CoroutineId,
    pub msg_info: u32,
    pub extend_msg: [u16; 8],
}

impl IPCItem {
    pub const fn new() -> Self {
        Self {
            cid: CoroutineId(0),
            msg_info: 0,
            extend_msg: [0u16; 8],
        }
    }

    pub fn from(cid: CoroutineId, msg: u32) -> Self {
        Self {
            cid,
            msg_info: msg,
            extend_msg: [0u16; 8],
        }
    }
}

pub struct ItemsQueue {
    buffer: RingBuffer<IPCItem, MAX_ITEM_NUM>,
    lock: Mutex<()>,
}

impl ItemsQueue {
    pub fn new() -> Self {
        Self {
            buffer: RingBuffer::new(),
            lock: Mutex::new(()),
        }
    }

    #[inline]
    pub fn write_free_item(&mut self, item: &IPCItem) -> Result<(), ()> {
        loop {
            if let Some(_lock) = self.lock.try_lock() {
                return self.buffer.push(item);
            } else {
                r#yield();
            }
        }
    }

    #[inline]
    pub fn get_first_item(&mut self) -> Option<IPCItem> {
        loop {
            if let Some(_lock) = self.lock.try_lock() {
                return self.buffer.pop();
            } else {
                r#yield();
            }
        }
    }
}


#[repr(align(4096))]
pub struct NewBuffer {
    pub recv_req_status: AtomicBool,
    pub recv_reply_status: AtomicBool,
    pub req_items: ItemsQueue,
    pub res_items: ItemsQueue,
}

impl NewBuffer {
    pub fn new() -> Self {
        Self {
            recv_req_status: AtomicBool::new(false),
            recv_reply_status: AtomicBool::new(false),
            req_items: ItemsQueue::new(),
            res_items: ItemsQueue::new(),
        }
    }
}