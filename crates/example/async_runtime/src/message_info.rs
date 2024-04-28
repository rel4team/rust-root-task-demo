#[derive(Eq, PartialEq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum AsyncMessageLabel {
    UntypedRetype                       = 0,
    PutChar,
    RISCVPageTableMap,
    RISCVPageTableUnmap,
    RISCVPageMap,
    RISCVPageUnmap,
    RISCVPageGetAddress,
    CNodeRevoke,
    CNodeDelete,
    CNodeCancelBadgedSends,
    CNodeCopy,
    CNodeMint,
    CNodeMove,
    CNodeMutate,
    CNodeRotate,
    TCBBindNotification,
    TCBUnbindNotification,
    PutString,
    UnknownLabel
}

impl From<AsyncMessageLabel> for u32 {
    fn from(value: AsyncMessageLabel) -> Self {
        value as u32
    }
}

impl From<u32> for AsyncMessageLabel {
    fn from(value: u32) -> Self {
        match value {
            0 => AsyncMessageLabel::UntypedRetype,
            1 => AsyncMessageLabel::PutChar,
            2 => AsyncMessageLabel::RISCVPageTableMap,
            3 => AsyncMessageLabel::RISCVPageTableUnmap,
            4 => AsyncMessageLabel::RISCVPageMap,
            5 => AsyncMessageLabel::RISCVPageTableUnmap,
            6 => AsyncMessageLabel::RISCVPageGetAddress,
            7 => AsyncMessageLabel::CNodeRevoke,
            8 => AsyncMessageLabel::CNodeDelete,
            9 => AsyncMessageLabel::CNodeCancelBadgedSends,
            10 => AsyncMessageLabel::CNodeCopy,
            11 => AsyncMessageLabel::CNodeMint,
            12 => AsyncMessageLabel::CNodeMove,
            13 => AsyncMessageLabel::CNodeMutate,
            14 => AsyncMessageLabel::CNodeRotate,
            15 => AsyncMessageLabel::TCBBindNotification,
            16 => AsyncMessageLabel::TCBUnbindNotification,
            17 => AsyncMessageLabel::PutString,
            _ => AsyncMessageLabel::UnknownLabel
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum AsyncErrorLabel {
    NoError                       = 0,
    SyscallError
}

impl From<AsyncErrorLabel> for u16 {
    fn from(value: AsyncErrorLabel) -> Self {
        value as u16
    }
}

impl From<u16> for AsyncErrorLabel {
    fn from(value: u16) -> Self {
        match value {
            0 => AsyncErrorLabel:: NoError,
            _ => AsyncErrorLabel::SyscallError
        }
    }
}