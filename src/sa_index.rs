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
    #[must_use]
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
                i64::from(self)
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
impl_sa_index!(i64);

#[cfg(test)]
mod tests {
    use super::*;

    fn check<I: SaIndex>() {
        assert_eq!(I::ZERO.to_i64(), 0);
        assert_eq!(I::ONE.to_i64(), 1);
        assert_eq!(I::from_usize(12345).to_usize(), 12345);
        assert_eq!(I::from_isize(-7).to_isize(), -7);
        assert_eq!(I::MAX.to_usize(), I::MAX.to_i64() as usize);
        // `!x` flips the marker state: non-negative <-> negative, and is an involution.
        let x = I::from_usize(42);
        assert!(x.bit_not() < I::ZERO);
        assert_eq!(x.bit_not().bit_not(), x);
        assert_eq!(I::ZERO.bit_not().to_i64(), -1);
    }

    #[test]
    fn test_sa_index_i32() {
        check::<i32>();
        assert_eq!(<i32 as SaIndex>::MAX.to_i64(), i64::from(i32::MAX));
    }

    #[test]
    fn test_sa_index_i64() {
        check::<i64>();
        assert_eq!(<i64 as SaIndex>::MAX.to_i64(), i64::MAX);
    }

    #[test]
    fn test_negative_to_usize_sign_extends_like_as() {
        // Callers rely on `to_usize` matching `as usize` (a negative value wraps).
        assert_eq!((-1i32).to_usize(), usize::MAX);
        assert_eq!((-1i64).to_usize(), usize::MAX);
    }
}
