use core::sync::atomic::AtomicBool;
use spin::Mutex;
use crate::coroutine::CoroutineId;
use sel4::r#yield;
use crate::utils::RingBuffer;

pub const MAX_ITEM_NUM: usize = 4096;
pub const MAX_IPC_MSG_LEN: usize = 8;
#[repr(align(8))]
#[derive(Clone, Copy, Debug)]
pub struct IPCItem {
    pub cid: CoroutineId,
    pub msg_info: u32,
    pub extend_msg: [u16; MAX_IPC_MSG_LEN],
}

impl Default for IPCItem {
    fn default() -> Self {
        Self {
            cid: Default::default(),
            msg_info: 0,
            extend_msg: [0; MAX_IPC_MSG_LEN],
        }
    }
}

impl IPCItem {
    pub const fn new() -> Self {
        Self {
            cid: CoroutineId(0),
            msg_info: 0,
            extend_msg: [0u16; MAX_IPC_MSG_LEN],
        }
    }

    pub fn from(cid: CoroutineId, msg: u32) -> Self {
        Self {
            cid,
            msg_info: msg,
            extend_msg: [0u16; MAX_IPC_MSG_LEN],
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
        // let _lock = self.lock.lock();
        // return self.buffer.push(item);
        loop {
            if let Some(_lock) = self.lock.try_lock() {
                return self.buffer.push(item);
            } else {
                // sel4::debug_println!("w");
                r#yield();
            }
        }
    }

    #[inline]
    pub fn get_first_item(&mut self) -> Option<IPCItem> {
        // let _lock = self.lock.lock();
        // return self.buffer.pop();
        loop {
            if let Some(_lock) = self.lock.try_lock() {
                return self.buffer.pop();
            } else {
                // sel4::debug_println!("g");
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