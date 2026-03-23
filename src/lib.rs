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

#[derive(Debug, PartialEq, Eq)]
pub enum DivSufSortError {
    InvalidArgument,
    AllocationFailure,
}

impl std::fmt::Display for DivSufSortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "invalid argument"),
            Self::AllocationFailure => write!(f, "allocation failure"),
        }
    }
}

impl std::error::Error for DivSufSortError {}
