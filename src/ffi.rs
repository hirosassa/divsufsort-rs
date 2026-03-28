/// FFI bindings to the vendored C libdivsufsort (32-bit index variant).
use core::ffi::{c_int, c_uchar};

#[link(name = "divsufsort_c", kind = "static")]
unsafe extern "C" {
    /// Constructs the suffix array of T[0..n-1].
    /// Returns 0 on success, negative on error.
    pub fn divsufsort(t: *const c_uchar, sa: *mut c_int, n: c_int) -> c_int;
}

/// Safe wrapper that mirrors the Rust `divsufsort` API.
pub fn divsufsort_c(t: &[u8], sa: &mut [i32]) -> i32 {
    assert_eq!(t.len(), sa.len());
    unsafe { divsufsort(t.as_ptr(), sa.as_mut_ptr(), t.len() as c_int) }
}
