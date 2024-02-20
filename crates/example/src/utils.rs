use sel4::{LocalCPtr, Notification};
use sel4::cap_type::Untyped;

#[derive(Copy, Clone)]
pub struct IndexAllocator<const SIZE: usize> where
    [(); (SIZE + 7) / 8]: {
    bitmap: [u8; (SIZE + 7) / 8]
}

impl<const SIZE: usize> IndexAllocator<SIZE> where
    [(); (SIZE + 7) / 8]: {
    pub fn new() -> Self {
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

