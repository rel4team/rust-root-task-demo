
use sel4::get_clock;

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


pub trait BitMap {
    fn set(&mut self, pos: usize);

    fn get(&self, pos: usize) -> bool ;

    fn clear(&mut self, pos: usize);

    fn find_first_one(&self) -> usize;

    fn find_first_zero(&self) -> usize;

}
#[derive(Copy, Clone)]
pub struct BitMap4096 {
    l1: BitMap64,
    l2: [BitMap64; 64],
}

impl BitMap4096 {
    #[inline]
    pub const fn new() -> Self {
        Self { l1:BitMap64::new() , l2: [BitMap64::new(); 64]}
    }

    #[inline]
    pub fn fetch(&mut self) -> Option<usize> {
        let pos = self.find_first_one();
        if pos >= 4096 {
            return None;
        }
        self.clear(pos);
        Some(pos)
    }

    #[inline]
    pub fn empty(&self) -> bool {
        self.l1.empty()
    }
}

impl BitMap for BitMap4096 {
    #[inline]
    fn set(&mut self, pos: usize) {
        let l1_index = pos >> 6;
        let l2_index = pos & 0b0011_1111;
        self.l1.set(l1_index);
        self.l2[l1_index].set(l2_index);
    }

    fn get(&self, pos: usize) -> bool {
        let l1_index = pos >> 6;
        let l2_index = pos & 0b0011_1111;
        self.l1.get(l1_index) && self.l2[l1_index].get(l2_index)
    }


    fn clear(&mut self, pos: usize) {
        let l1_index = pos >> 6;
        let l2_index = pos & 0b0011_1111;
        self.l2[l1_index].clear(l2_index);
        if self.l2[l1_index].data == 0 {
            self.l1.clear(l1_index);
        }
    }

    fn find_first_one(&self) -> usize {
        let l1_index = self.l1.find_first_one();
        if l1_index == 64 {
            return 64 << 6;
        }
        let l2_index = self.l2[l1_index].find_first_one();
        return (l1_index << 6) + l2_index;
    }

    fn find_first_zero(&self) -> usize {
        for l1_index in 0..64 {
            let l2_index = self.l2[l1_index].find_first_zero();
            if l2_index < 64 {
                return (l1_index << 6) + l2_index;
            }
        }
        64 * 64
    }
}


#[derive(Copy, Clone)]
pub struct BitMap64 {
    pub data: u64,
}

impl BitMap64 {
    #[inline]
    pub const fn new() -> Self {
        BitMap64 { data: 0 }
    }

    #[inline]
    pub fn empty(&self) -> bool {
        self.data == 0
    }
}

impl BitMap for BitMap64 {


    #[inline]
    fn set(&mut self, pos: usize) {
        assert!(pos < 64, "Position out of range");
        self.data |= 1 << pos;
    }

    fn get(&self, pos: usize) -> bool {
        self.data & (1 << pos) != 0
    }


    #[inline]
    fn clear(&mut self, pos: usize) {
        assert!(pos < 64, "Position out of range");
        self.data &= !(1 << pos);
    }

    #[inline]
    fn find_first_one(&self) -> usize {
        self.data.trailing_zeros() as usize
    }

    #[inline]
    fn find_first_zero(&self) -> usize {
        self.data.trailing_ones() as usize
    }
}

#[derive(Copy, Clone)]
pub struct RingBuffer<T, const SIZE: usize> {
    data: [T; SIZE],
    pub start: usize,
    pub end: usize,
}

impl<T, const SIZE: usize> RingBuffer<T, SIZE> where T: Default + Copy + Clone {
    pub fn new() -> Self {
        Self {
            data: [T::default(); SIZE],
            start: 0,
            end: 0,
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        (self.end + SIZE - self.start) % SIZE
    }

    #[inline]
    pub fn empty(&self) -> bool {
        self.end == self.start
    }

    #[inline]
    pub fn full(&self) -> bool {
        (self.end + 1) % SIZE == self.start
    }

    #[inline]
    pub fn push(&mut self, item: &T) -> Result<(), ()> {
        if !self.full() {
            self.data[self.end] = *item;
            self.end = (self.end + 1) % SIZE;
            return Ok(());
        }
        Err(())
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        return if !self.empty() {
            let ans = self.data[self.start];
            self.start = (self.start + 1) % SIZE;
            Some(ans)
        } else {
            None
        }
    }
}


