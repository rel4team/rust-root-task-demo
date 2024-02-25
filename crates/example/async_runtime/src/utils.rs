use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use crate::{coroutine_get_current, coroutine_get_immediate_value};

#[derive(Copy, Clone)]
pub struct IndexAllocator<const SIZE: usize> where
    [(); (SIZE + 7) / 8]: {
    bitmap: [u8; (SIZE + 7) / 8]
}

impl<const SIZE: usize> IndexAllocator<SIZE> where
    [(); (SIZE + 7) / 8]: {
    pub const fn new() -> Self {
        Self {
            bitmap :[0; (SIZE + 7) / 8]
        }
    }

    pub fn allocate(&mut self) -> Option<usize> {
        (0..SIZE).find(|i| {self.bitmap[i / 8] & (1 << (i % 8)) == 0 }).map(|index| {
            self.bitmap[index / 8] |= 1 << (index % 8);
            index
        })
    }

    pub fn release(&mut self, index: usize) {
        self.bitmap[index / 8] &= !(1 << (index % 8));
    }
}

pub struct BitMap64 {
    data: u64,
}

impl BitMap64 {
    #[inline]
    pub const fn new() -> Self {
        BitMap64 { data: 0 }
    }

    #[inline]
    pub fn set(&mut self, pos: usize) {
        assert!(pos < 64, "Position out of range");
        self.data |= 1 << pos;
    }

    #[inline]
    pub fn full(&self) -> bool {
        self.find_first_zero() == 64
    }

    #[inline]
    pub fn emtpy(&self) -> bool {
        self.find_first_one() == 64
    }

    #[inline]
    pub fn clear(&mut self, pos: usize) {
        assert!(pos < 64, "Position out of range");
        self.data &= !(1 << pos);
    }

    #[inline]
    pub fn find_first_one(&self) -> usize {
        self.data.trailing_zeros() as usize
    }

    #[inline]
    pub fn find_first_zero(&self) -> usize {
        self.data.trailing_ones() as usize
    }
}

pub async fn yield_now() -> Option<u64> {
    let mut helper = Box::new(YieldHelper::new());
    helper.await;
    coroutine_get_immediate_value(&coroutine_get_current())
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

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 == false {
            self.0 = true;
            return Poll::Pending;
        }
        return Poll::Ready(());
    }
}
