// use crate::coroutine::CoroutineId;
// use sel4::MessageInfo;
// use super::utils::BitMap64;
// pub const MAX_ITEM_NUM: usize = 64;
// #[repr(C)]
// #[derive(Clone, Copy)]
// pub struct IPCItem {
//     pub cid: CoroutineId,
//     pub msg_info: usize,
// }
//
// impl Default for IPCItem {
//     fn default() -> Self {
//         Self {
//             cid: CoroutineId(0),
//             msg_info: 0,
//         }
//     }
// }
//
// pub struct ItemsQueue {
//     pub bitmap: BitMap64,
//     pub items: [IPCItem; MAX_ITEM_NUM],
// }
//
// impl ItemsQueue {
//     pub fn new() -> Self {
//
//         Self {
//             bitmap: BitMap64::new(),
//             items: [IPCItem::default(); MAX_ITEM_NUM]
//         }
//     }
// }
//
// #[repr(C)]
// pub struct NewBuffer {
//     pub recv_req_status: bool,
//     pub recv_reply_status: bool,
//     pub req_items: ItemsQueue,
//     pub res_items: ItemsQueue,
// }