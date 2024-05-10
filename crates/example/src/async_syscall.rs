use sel4::{CPtr, CapRights, Notification, ObjectBlueprint, PageTable, VMAttributes, TCB};

use crate::async_lib::{seL4_CNode_Copy, seL4_CNode_Delete, seL4_CNode_Mint, seL4_Putchar, seL4_Putstring, seL4_RISCV_PageTable_Map, seL4_RISCV_PageTable_Unmap, seL4_RISCV_Page_Get_Address, seL4_RISCV_Page_Map, seL4_RISCV_Page_Unmap, seL4_TCB_Bind_Notification, seL4_TCB_Unbind_Notification, seL4_Untyped_Retype};

pub async fn syscall_untyped_retype(
    service: CPtr,
    r#type: ObjectBlueprint,
    size_bits: usize,
    root: CPtr,
    node_index: usize,
    node_depth: usize,
    node_offset: usize,
    num_objects: usize
) {
    seL4_Untyped_Retype(service, r#type, size_bits, root, node_index, node_depth, node_offset, num_objects).await;
}

pub async fn syscall_riscv_page_get_address(
    vaddr: usize
) {
    seL4_RISCV_Page_Get_Address(vaddr).await;
}

pub async fn syscall_putchar(
    c: u16
) {
    seL4_Putchar(c).await;
}

pub async fn syscall_putstring(
    data: &[u16]
) {
    seL4_Putstring(data).await;
}

pub async fn syscall_tcb_bind_notification(tcb: TCB, notification: Notification) {
    seL4_TCB_Bind_Notification(tcb, notification).await;
}

pub async fn syscall_tcb_unbind_notification(tcb: TCB) {
    seL4_TCB_Unbind_Notification(tcb).await;
}

pub async fn syscall_cnode_copy(
    dest_root_cptr: CPtr,
    dest_index: usize,
    dest_depth: usize,
    src_root_cptr: CPtr,
    src_index: usize,
    src_depth: usize,
    cap_right: CapRights
) {
    seL4_CNode_Copy(dest_root_cptr, dest_index, dest_depth, src_root_cptr, src_index, src_depth, cap_right).await;
}

pub async fn syscall_cnode_delete(
    service: CPtr,
    node_index: usize,
    node_depth: usize
) {
    seL4_CNode_Delete(service, node_index, node_depth).await;
}

pub async fn syscall_cnode_mint(
    dest_root_cptr: CPtr,
    dest_index: usize,
    dest_depth: usize,
    src_root_cptr: CPtr,
    src_index: usize,
    src_depth: usize,
    cap_right: CapRights,
    badge: u64
) {
    seL4_CNode_Mint(dest_root_cptr, dest_index, dest_depth, src_root_cptr, src_index, src_depth, cap_right, badge).await;
}

pub async fn syscall_riscv_page_map(
    service: CPtr,
    page_table: CPtr,
    vaddr: usize,
    rights: usize,
    attrs: usize,
) {
    seL4_RISCV_Page_Map(service, page_table, vaddr, rights, attrs).await;
}

pub async fn syscall_riscv_page_unmap(
    service: CPtr,
) {
    seL4_RISCV_Page_Unmap(service).await;
}

pub async fn syscall_riscv_pagetable_map(
    service: CPtr,
    vspace: CPtr,
    vaddr: usize,
    attrs: usize,
) {
    seL4_RISCV_PageTable_Map(service, vspace, vaddr, attrs).await;
}

pub async fn syscall_riscv_pagetable_unmap(
    service: CPtr,
) {
    seL4_RISCV_PageTable_Unmap(service).await;
}