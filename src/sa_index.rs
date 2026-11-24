//! The signed integer type used to store suffix array entries.
//!
//! The algorithms encode state in the sign of an entry (a negative value is a group
//! marker or skip length, and `!x` flips that flag), so the index type must be a signed
//! integer. The trait is sealed: only [`i32`] and [`i64`] implement it.

use core::fmt::Debug;
use core::ops::{Add, AddAssign, Not, Sub, SubAssign};

mod sealed {
    pub trait Sealed {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
}

/// Signed integer type usable as a suffix array entry (`i32` or `i64`).
///
/// `i32` supports inputs of up to `i32::MAX` bytes and needs 4 bytes per input byte for the
/// suffix array; `i64` lifts the limit to `i64::MAX` at 8 bytes per input byte.
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait SaIndex:
    sealed::Sealed
    + Copy
    + Default
    + Ord
    + Debug
    + Send
    + Sync
    + 'static
    + Add<Output = Self>
    + Sub<Output = Self>
    + Not<Output = Self>
    + AddAssign
    + SubAssign
{
    /// The value `0`.
    const ZERO: Self;
    /// The value `1`.
    const ONE: Self;
    /// The largest representable value, i.e. the longest supported input length.
    const MAX: Self;

    /// Converts to `usize` as an `as` cast does: a negative value sign-extends.
    fn to_usize(self) -> usize;
    /// Converts to `isize` as an `as` cast does.
    fn to_isize(self) -> isize;
    /// Converts to `i64` (lossless).
    fn to_i64(self) -> i64;
    /// Converts from `usize` as an `as` cast does. Callers guarantee `v <= Self::MAX`.
    fn from_usize(v: usize) -> Self;
    /// Converts from `isize` as an `as` cast does.
    fn from_isize(v: isize) -> Self;
    /// Bitwise NOT (`!x`), which flips the "marker" state of an entry.
    #[inline(always)]
    fn bit_not(self) -> Self {
        !self
    }
}

macro_rules! impl_sa_index {
    ($ty:ty) => {
        impl SaIndex for $ty {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = <$ty>::MAX;

            #[inline(always)]
            fn to_usize(self) -> usize {
                self as usize
            }
            #[inline(always)]
            fn to_isize(self) -> isize {
                self as isize
            }
            #[inline(always)]
            fn to_i64(self) -> i64 {
                self as i64
            }
            #[inline(always)]
            fn from_usize(v: usize) -> Self {
                v as Self
            }
            #[inline(always)]
            fn from_isize(v: isize) -> Self {
                v as Self
            }
        }
    };
}

impl_sa_index!(i32);
