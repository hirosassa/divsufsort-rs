/// FFI bindings to the vendored C libdivsufsort (32-bit and 64-bit index variants).
use core::ffi::{c_int, c_uchar};

#[link(name = "divsufsort64_c", kind = "static")]
#[expect(unsafe_code, reason = "SAFETY: declaration of the vendored C function")]
unsafe extern "C" {
    /// 64-bit index variant of `divsufsort`.
    pub fn divsufsort64(t: *const c_uchar, sa: *mut i64, n: i64) -> c_int;
}

#[link(name = "divsufsort_c", kind = "static")]
#[expect(unsafe_code, reason = "SAFETY: declaration of the vendored C function")]
unsafe extern "C" {
    /// Constructs the suffix array of T[0..n-1].
    /// Returns 0 on success, negative on error.
    pub fn divsufsort(t: *const c_uchar, sa: *mut c_int, n: c_int) -> c_int;
}

/// Safe wrapper that mirrors the Rust `divsufsort` API.
///
/// # Panics
///
/// Panics if `t` and `sa` differ in length.
pub fn divsufsort_c(t: &[u8], sa: &mut [i32]) -> i32 {
    assert_eq!(t.len(), sa.len());
    #[expect(
        unsafe_code,
        reason = "SAFETY: lengths are equal (asserted); C reads/writes exactly n elements"
    )]
    // SAFETY: both pointers are valid for `t.len()` elements (equal lengths asserted above).
    let r = unsafe { divsufsort(t.as_ptr(), sa.as_mut_ptr(), t.len() as c_int) };
    r
}

/// Safe wrapper around the 64-bit index C variant (`divsufsort64`).
///
/// # Panics
///
/// Panics if `t` and `sa` differ in length.
pub fn divsufsort64_c(t: &[u8], sa: &mut [i64]) -> i32 {
    assert_eq!(t.len(), sa.len());
    #[expect(
        unsafe_code,
        reason = "SAFETY: lengths are equal (asserted); C reads/writes exactly n elements"
    )]
    // SAFETY: both pointers are valid for `t.len()` elements (equal lengths asserted above).
    let r = unsafe { divsufsort64(t.as_ptr(), sa.as_mut_ptr(), t.len() as i64) };
    r
}
