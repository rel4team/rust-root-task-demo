//
// #[derive(Copy, Clone)]
// pub struct IndexAllocator<const SIZE: usize> where
//     [(); (SIZE + 7) / 8]: {
//     bitmap: [u8; (SIZE + 7) / 8]
// }
//
// impl<const SIZE: usize> IndexAllocator<SIZE> where
//     [(); (SIZE + 7) / 8]: {
//     pub fn new() -> Self {
//         Self {
//             bitmap :[0; (SIZE + 7) / 8]
//         }
//     }
//
//     pub fn allocate(&mut self) -> Option<usize> {
//         (0..SIZE).find(|i| {self.bitmap[i / 8] & (1 << (i % 8)) == 0 }).map(|index| {
//             self.bitmap[index / 8] |= 1 << (index % 8);
//             index
//         })
//     }
//
//     pub fn release(&mut self, index: usize) {
//         self.bitmap[index / 8] &= !(1 << (index % 8));
//     }
// }
//
// pub struct BitMap64 {
//     data: u64,
// }
//
// impl BitMap64 {
//     #[inline]
//     pub fn new() -> Self {
//         BitMap64 { data: 0 }
//     }
//
//     #[inline]
//     pub fn set(&mut self, pos: usize) {
//         assert!(pos < 64, "Position out of range");
//         self.data |= 1 << pos;
//     }
//
//     #[inline]
//     pub fn clear(&mut self, pos: usize) {
//         assert!(pos < 64, "Position out of range");
//         self.data &= !(1 << pos);
//     }
//
//     #[inline]
//     pub fn find_first_one(&self) -> usize {
//         self.data.leading_zeros() as usize
//     }
//
//     #[inline]
//     pub fn find_first_zero(&self) -> usize {
//         self.data.leading_ones() as usize
//     }
// }
