pub const ALPHABET_SIZE: usize = 256;
pub const BUCKET_A_SIZE: usize = ALPHABET_SIZE;
pub const BUCKET_B_SIZE: usize = ALPHABET_SIZE * ALPHABET_SIZE;

pub const SS_INSERTIONSORT_THRESHOLD: usize = 8;
pub const SS_BLOCKSIZE: usize = 1024;
pub const SS_MISORT_STACKSIZE: usize = 24;
pub const SS_SMERGE_STACKSIZE: usize = 32;

pub const TR_INSERTIONSORT_THRESHOLD: usize = 8;
pub const TR_STACKSIZE: usize = 64;

/// Fixed-capacity stack backed by an array. Zero-cost abstraction over
/// the `stack[ssize] = item; ssize += 1` / `ssize -= 1; stack[ssize]` pattern
/// used throughout the sorting algorithms.
pub struct FixedStack<T: Copy + Default, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T: Copy + Default, const N: usize> FixedStack<T, N> {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
            len: 0,
        }
    }

    #[inline(always)]
    pub const fn push(&mut self, item: T) {
        self.data[self.len] = item;
        self.len += 1;
    }

    #[inline(always)]
    pub const fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.data[self.len])
        }
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }
}

impl<T: Copy + Default, const N: usize> std::ops::Index<usize> for FixedStack<T, N> {
    type Output = T;
    #[inline(always)]
    fn index(&self, idx: usize) -> &T {
        &self.data[idx]
    }
}

impl<T: Copy + Default, const N: usize> std::ops::IndexMut<usize> for FixedStack<T, N> {
    #[inline(always)]
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.data[idx]
    }
}

pub static LG_TABLE: [i32; 256] = [
    -1, 0, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7,
];
