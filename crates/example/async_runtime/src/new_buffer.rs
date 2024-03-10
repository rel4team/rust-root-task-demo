use core::ops::Index;
use core::sync::atomic::AtomicBool;
use spin::Mutex;
use crate::utils::BitMap;
use crate::coroutine::CoroutineId;
use super::utils::{BitMap64, BitMap4096};
use sel4::get_clock;
use sel4::r#yield;
pub const MAX_ITEM_NUM: usize = 4096;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    pub bitmap: BitMap4096,
    pub items: [IPCItem; MAX_ITEM_NUM],
    lock: Mutex<()>,
}


impl ItemsQueue {
    pub const fn new() -> Self {
        Self {
            bitmap: BitMap4096::new(),
            items: [IPCItem::new(); MAX_ITEM_NUM],
            lock: Mutex::new(())
        }
    }

    #[inline]
    pub fn write_free_item(&mut self, item: &IPCItem) -> Result<(), ()> {
        loop {
            if let Some(_lock) = self.lock.try_lock() {
                let index = self.bitmap.find_first_zero();
                // sel4::debug_println!("[write_free_item] index: {}", index);
                return {
                    if index < MAX_ITEM_NUM {
                        self.items[index] = item.clone();
                        self.bitmap.set(index);
                        Ok(())
                    } else {
                        Err(())
                    }
                }
            } else {
                r#yield();
            }
        }

    }
    #[inline]
    pub fn get_first_item(&mut self) -> Option<IPCItem> {
        loop {
            if let Some(_lock) = self.lock.try_lock() {
                let index = self.bitmap.find_first_one();
                // sel4::debug_println!("[get_first_item] index: {}", index);
                return {
                    if index < MAX_ITEM_NUM {
                        let ans = Some(self.items[index]);
                        self.bitmap.clear(index);
                        ans
                    } else {
                        None
                    }
                }
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
    pub const fn new() -> Self {
        Self {
            recv_req_status: AtomicBool::new(false),
            recv_reply_status: AtomicBool::new(false),
            req_items: ItemsQueue::new(),
            res_items: ItemsQueue::new(),
        }
    }
}