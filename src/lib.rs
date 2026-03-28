#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod constants;
mod divsufsort;
#[cfg(feature = "c-bench")]
pub mod ffi;
mod sssort;
mod trsort;
mod utils;

pub use divsufsort::{divbwt, divsufsort};
pub use utils::{
    SufCheckError, bw_transform, inverse_bw_transform, sa_search, sa_simplesearch, sufcheck,
};

/// Errors returned by the public API of this crate.
#[derive(Debug, PartialEq, Eq)]
pub enum DivSufSortError {
    /// A supplied argument is invalid (e.g. slice length mismatch or out-of-range index).
    InvalidArgument,
    /// An internal allocation failed. Currently unused; reserved for future use.
    AllocationFailure,
}

impl core::fmt::Display for DivSufSortError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "invalid argument"),
            Self::AllocationFailure => write!(f, "allocation failure"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DivSufSortError {}
