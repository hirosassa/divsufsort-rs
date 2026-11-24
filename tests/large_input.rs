//! Multi-gigabyte inputs. All tests are `#[ignore]`d because of their memory needs; run them
//! manually, one at a time, on a machine with enough RAM:
//!
//! ```text
//! cargo test --release --test large_input -- --ignored --nocapture <name>
//! ```
//!
//! | test                                  | peak RAM  |
//! |---------------------------------------|-----------|
//! | `i32_at_i32_max_boundary`             | ~10 GiB   |
//! | `i64_beyond_i32_max`                  | ~19 GiB   |
//! | `i32_rejects_len_over_i32_max`        | ~2 GiB    |

use divsufsort_rs::{DivSufSortError, divsufsort, sufcheck};

fn random_text(n: usize) -> Vec<u8> {
    let mut state = 0x517c_c1b7_2722_0a95_u64;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            b'a' + ((state >> 33) % 26) as u8
        })
        .collect()
}

/// The largest length an `i32` suffix array supports: positions such as `isab + ...` and
/// `first - t` must not overflow anywhere near `i32::MAX`.
#[test]
#[ignore = "needs ~10 GiB of RAM"]
fn i32_at_i32_max_boundary() {
    let n = i32::MAX as usize;
    let t = random_text(n);
    let mut sa = vec![0i32; n];
    divsufsort(&t, &mut sa).unwrap();
    sufcheck(&t, &sa, false).unwrap();
}

/// A length that only `i64` can represent.
#[test]
#[ignore = "needs ~19 GiB of RAM"]
fn i64_beyond_i32_max() {
    let n = i32::MAX as usize + 1024;
    let t = random_text(n);
    let mut sa = vec![0i64; n];
    divsufsort(&t, &mut sa).unwrap();
    sufcheck(&t, &sa, false).unwrap();
}

#[test]
#[ignore = "needs ~2 GiB of RAM"]
fn i32_rejects_len_over_i32_max() {
    let t = vec![b'a'; i32::MAX as usize + 1];
    let mut sa: [i32; 0] = [];
    assert_eq!(
        divsufsort(&t, &mut sa),
        Err(DivSufSortError::InvalidArgument)
    );
}
