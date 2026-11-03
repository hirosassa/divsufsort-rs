//! Main suffix array and Burrows-Wheeler transform construction.
//!
//! Orchestrates the induced-sorting strategy:
//! 1. `sort_typebstar` — classify, sort B*-suffixes (parallel sssort + trsort), scatter back
//! 2. `construct_sa` / `construct_bwt` — induce the full SA or BWT from sorted B*-suffixes

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use crate::DivSufSortError;
use crate::constants::{ALPHABET_SIZE, BUCKET_A_SIZE, BUCKET_B_SIZE};
use crate::sssort::{SsortCtx, sssort};
use crate::trsort::trsort;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Bucket table of A-type suffixes, indexed by the first character.
type BucketA = [i32; BUCKET_A_SIZE];
/// Bucket table of B / B*-type suffixes, indexed by `(c1 << 8) | c0`.
type BucketB = [i32; BUCKET_B_SIZE];

/// Allocates a zeroed fixed-size table on the heap (avoids a large stack temporary).
fn zeroed_table<const N: usize>() -> Box<[i32; N]> {
    vec![0i32; N]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!("a boxed slice of length N always converts to [i32; N]"))
}

#[inline(always)]
const fn bucket_a(ba: &BucketA, c0: usize) -> i32 {
    ba[c0]
}
#[inline(always)]
const fn bucket_a_mut(ba: &mut BucketA, c0: usize) -> &mut i32 {
    &mut ba[c0]
}

/// BUCKET_B(c0, c1) = bucket_B[(c1 << 8) | c0]
#[inline(always)]
const fn bucket_b(bb: &BucketB, c0: usize, c1: usize) -> i32 {
    bb[(c1 << 8) | c0]
}
#[inline(always)]
const fn bucket_b_mut(bb: &mut BucketB, c0: usize, c1: usize) -> &mut i32 {
    &mut bb[(c1 << 8) | c0]
}

/// BUCKET_BSTAR(c0, c1) = bucket_B[(c0 << 8) | c1]
#[inline(always)]
const fn bucket_bstar(bb: &BucketB, c0: usize, c1: usize) -> i32 {
    bb[(c0 << 8) | c1]
}
#[inline(always)]
const fn bucket_bstar_mut(bb: &mut BucketB, c0: usize, c1: usize) -> &mut i32 {
    &mut bb[(c0 << 8) | c1]
}

/// Scans text right-to-left, classifying suffixes into A/B/B* types and counting
/// bucket sizes. B*-suffix positions are stored at the tail of `sa`.
/// Returns `m` = number of B*-suffixes found.
#[inline(always)]
fn count_and_classify_suffixes(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut BucketA,
    bkt_b: &mut BucketB,
    n: usize,
) -> usize {
    for x in bkt_a.iter_mut() {
        *x = 0;
    }
    for x in bkt_b.iter_mut() {
        *x = 0;
    }

    let mut m = n;
    let mut i = n as isize - 1;
    let mut c0 = t[n - 1] as usize;

    while 0 <= i {
        let mut c1;
        loop {
            c1 = c0;
            *bucket_a_mut(bkt_a, c1) += 1;
            i -= 1;
            if i < 0 {
                break;
            }
            c0 = t[i as usize] as usize;
            if c0 < c1 {
                break;
            }
        }
        if 0 <= i {
            *bucket_bstar_mut(bkt_b, c0, c1) += 1;
            m -= 1;
            sa[m] = i as i32;
            i -= 1;
            c1 = c0;
            while 0 <= i {
                c0 = t[i as usize] as usize;
                if c0 > c1 {
                    break;
                }
                *bucket_b_mut(bkt_b, c0, c1) += 1;
                i -= 1;
                c1 = c0;
            }
        }
    }
    n - m
}

/// Builds the inverse suffix array (ISA) from sorted B*-suffixes.
/// Processes sa[0..m] and writes group labels into sa[isab..isab+m].
#[inline(always)]
const fn build_isa_from_sorted_bstar(sa: &mut [i32], isab: usize, m: usize) {
    let mut i = m as isize - 1;
    while 0 <= i {
        if sa[i as usize] >= 0 {
            let j_val = i;
            loop {
                sa[isab + sa[i as usize] as usize] = i as i32;
                i -= 1;
                if i < 0 || sa[i as usize] < 0 {
                    break;
                }
            }
            sa[(i + 1) as usize] = (i - j_val) as i32;
            if i <= 0 {
                break;
            }
        }
        let j_val = i;
        loop {
            sa[i as usize] = !sa[i as usize];
            sa[isab + sa[i as usize] as usize] = j_val as i32;
            i -= 1;
            if i < 0 || sa[i as usize] >= 0 {
                break;
            }
        }
        if i >= 0 {
            sa[isab + sa[i as usize] as usize] = j_val as i32;
        }
        i -= 1;
    }
}

/// Scatters sorted B*-suffix positions back to their text positions.
/// Reads ranks from sa[isab..] and writes text positions into sa[rank].
#[inline(always)]
const fn scatter_bstar_to_sa(t: &[u8], sa: &mut [i32], isab: usize, n: usize, m: usize) {
    let mut i = n as isize - 1;
    let mut j_val = m;
    let mut c0 = t[n - 1] as usize;
    while 0 <= i {
        i -= 1;
        let mut c1 = c0;
        while 0 <= i && {
            c0 = t[i as usize] as usize;
            c0 >= c1
        } {
            i -= 1;
            c1 = c0;
        }
        if 0 <= i {
            let tv = i as usize;
            i -= 1;
            c1 = c0;
            while 0 <= i && {
                c0 = t[i as usize] as usize;
                c0 <= c1
            } {
                i -= 1;
                c1 = c0;
            }
            j_val -= 1;
            let rank = sa[isab + j_val] as usize;
            sa[rank] = if tv == 0 || (tv as isize - i) > 1 {
                tv as i32
            } else {
                !(tv as i32)
            };
        }
    }
}

/// Copies sorted B*-suffixes into their final bucket positions and updates
/// bucket boundaries for the subsequent construct_sa/construct_bwt phase.
#[inline(always)]
fn place_sorted_bstar_in_buckets(
    sa: &mut [i32],
    bkt_a: &BucketA,
    bkt_b: &mut BucketB,
    m: usize,
    n: usize,
) {
    *bucket_b_mut(bkt_b, ALPHABET_SIZE - 1, ALPHABET_SIZE - 1) = n as i32;
    let mut k = m as isize - 1;
    for c0 in (0..=(ALPHABET_SIZE as isize - 2)).rev() {
        let c0 = c0 as usize;
        let mut i_val = bucket_a(bkt_a, c0 + 1) as isize - 1;
        for c1 in (c0 + 1..ALPHABET_SIZE).rev() {
            let t_val = i_val - bucket_b(bkt_b, c0, c1) as isize;
            *bucket_b_mut(bkt_b, c0, c1) = i_val as i32;

            let j_val = bucket_bstar(bkt_b, c0, c1) as isize;
            let mut ii = t_val;
            let mut kk = k;
            while j_val <= kk {
                // Move in place, as in C: ii >= kk and both walk downwards, so a source is
                // never overwritten before it has been moved.
                sa[ii as usize] = sa[kk as usize];
                ii -= 1;
                kk -= 1;
            }
            i_val = ii;
            k = kk;
        }
        *bucket_bstar_mut(bkt_b, c0, c0 + 1) =
            (i_val - bucket_b(bkt_b, c0, c0) as isize + 1) as i32;
        *bucket_b_mut(bkt_b, c0, c0) = i_val as i32;
    }
}

/// Sorts B*-type suffixes using the induced-sorting framework.
///
/// Phases: classify suffixes → compute bucket boundaries → distribute B*-suffixes →
/// parallel sssort → build ISA → trsort equal groups → scatter back to final positions.
/// Returns the number of B*-suffixes found.
fn sort_typebstar(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut BucketA,
    bkt_b: &mut BucketB,
    n: usize,
) -> usize {
    let m = count_and_classify_suffixes(t, sa, bkt_a, bkt_b, n);

    {
        let mut ii: i32 = 0;
        let mut j: i32 = 0;
        for c in 0..ALPHABET_SIZE {
            let t_val = ii + bucket_a(bkt_a, c);
            *bucket_a_mut(bkt_a, c) = ii + j; // start point
            ii = t_val + bucket_b(bkt_b, c, c);
            for c1 in (c + 1)..ALPHABET_SIZE {
                j += bucket_bstar(bkt_b, c, c1);
                *bucket_bstar_mut(bkt_b, c, c1) = j; // end point
                ii += bucket_b(bkt_b, c, c1);
            }
        }
    }

    if m > 0 {
        let pab = n - m; // PAb = SA + n - m (offset within SA)
        let isab = m; // ISAb = SA + m

        for i in (0..=(m as isize - 2)).rev() {
            let tv = sa[pab + i as usize];
            let c0 = t[tv as usize] as usize;
            let c1 = t[(tv + 1) as usize] as usize;
            let pos = bucket_bstar_mut(bkt_b, c0, c1);
            *pos -= 1;
            let pos_val = *pos as usize;
            sa[pos_val] = i as i32;
        }
        {
            let tv = sa[pab + m - 1];
            let c0 = t[tv as usize] as usize;
            let c1 = t[(tv + 1) as usize] as usize;
            let pos = bucket_bstar_mut(bkt_b, c0, c1);
            *pos -= 1;
            let pos_val = *pos as usize;
            sa[pos_val] = (m - 1) as i32;
        }

        // Phase 1: collect all non-trivial bucket sort jobs (i_val, j, lastsuffix).
        // Maintaining `j` sequentially is required because each bucket's end equals the
        // previous bucket's start (the ranges tile sa[0..m] contiguously).
        let jobs: Vec<(usize, usize, bool)> = {
            let mut jobs = Vec::new();
            let mut c0_iter = ALPHABET_SIZE as isize - 2;
            let mut j = m;
            while 0 < j && c0_iter >= 0 {
                let c0 = c0_iter as usize;
                let mut c1 = ALPHABET_SIZE - 1;
                while c0 < c1 {
                    let i_val = bucket_bstar(bkt_b, c0, c1) as usize;
                    if j - i_val > 1 {
                        let lastsuffix = sa[i_val] == (m as i32 - 1);
                        jobs.push((i_val, j, lastsuffix));
                    }
                    j = i_val;
                    if c1 == 0 {
                        break;
                    }
                    c1 -= 1;
                }
                c0_iter -= 1;
            }
            jobs
        };

        // Phase 2: execute bucket sorts in parallel.
        //
        // The jobs' ranges [i_val..j) tile sa[0..m] without overlap, so sa[..m] is split into
        // one exclusive `&mut` chunk per job. PA = sa[pab..n] lies in sa[m..] (m <= n/2, so
        // pab = n - m >= m) and is only read, so it is shared as `sa_right`. sssort uses
        // bufsize=0, i.e. the tail of each job's own chunk as its merge buffer.
        {
            let (sa_left, sa_right) = sa.split_at_mut(m);
            let sa_right: &[i32] = sa_right;
            let ctx = SsortCtx {
                t,
                pa: sa_right,
                pab: pab - m,
                depth: 2,
                n: n as i32,
            };

            // `jobs` is ordered by descending position; split from the left end instead.
            let mut chunks: Vec<(&mut [i32], bool)> = Vec::with_capacity(jobs.len());
            let mut rest = sa_left;
            let mut consumed = 0usize;
            for &(i_val, j_val, lastsuffix) in jobs.iter().rev() {
                let (_, tail) = core::mem::take(&mut rest).split_at_mut(i_val - consumed);
                let (chunk, tail) = tail.split_at_mut(j_val - i_val);
                chunks.push((chunk, lastsuffix));
                rest = tail;
                consumed = j_val;
            }

            #[cfg(feature = "rayon")]
            let iter = chunks.par_iter_mut();
            #[cfg(not(feature = "rayon"))]
            let iter = chunks.iter_mut();
            iter.for_each(|(chunk, lastsuffix)| {
                let len = chunk.len();
                sssort(&ctx, chunk, 0, len, 0, 0, *lastsuffix);
            });
        }

        build_isa_from_sorted_bstar(sa, isab, m);

        {
            let (sa_left, sa_right) = sa.split_at_mut(m);
            trsort(sa_right, sa_left, m as i32, 1);
        }

        scatter_bstar_to_sa(t, sa, isab, n, m);

        place_sorted_bstar_in_buckets(sa, bkt_a, bkt_b, m, n);
    }

    m
}

/// Induces the full suffix array from the sorted B*-suffixes by scanning
/// bucket boundaries left-to-right (A-type) and right-to-left (B-type).
fn construct_sa(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut BucketA,
    bkt_b: &mut BucketB,
    n: usize,
    m: usize,
) {
    if m > 0 {
        for c1 in (0..=(ALPHABET_SIZE - 2)).rev() {
            let i_start = bucket_bstar(bkt_b, c1, c1 + 1) as usize;
            let j_end = bucket_a(bkt_a, c1 + 1) as usize;
            let mut k_idx: usize = 0;
            let mut c2: isize = -1;

            let mut j = j_end as isize - 1;
            while i_start as isize <= j {
                let s = sa[j as usize];
                if s > 0 {
                    let s = s as usize;
                    sa[j as usize] = !sa[j as usize];
                    let c0 = t[s - 1] as usize;
                    let s_val = if s > 1 && t[s - 2] as usize > c0 {
                        !((s - 1) as i32)
                    } else {
                        (s - 1) as i32
                    };
                    if c0 != c2 as usize {
                        if c2 >= 0 {
                            *bucket_b_mut(bkt_b, c2 as usize, c1) = k_idx as i32;
                        }
                        c2 = c0 as isize;
                        k_idx = bucket_b(bkt_b, c0, c1) as usize;
                    }
                    sa[k_idx] = s_val;
                    k_idx = k_idx.saturating_sub(1);
                } else {
                    sa[j as usize] = !s;
                }
                j -= 1;
            }
        }
    }

    let c2_init = t[n - 1] as usize;
    let mut k_idx = bucket_a(bkt_a, c2_init) as usize;
    sa[k_idx] = if (t[n - 2] as usize) < c2_init {
        !((n - 1) as i32)
    } else {
        (n - 1) as i32
    };
    k_idx += 1;
    let mut c2 = c2_init as isize;

    for i in 0..n {
        let s = sa[i];
        if s > 0 {
            let s = s as usize;
            let c0 = t[s - 1] as usize;
            let s_val = if s == 1 || (t[s - 2] as usize) < c0 {
                !((s - 1) as i32)
            } else {
                (s - 1) as i32
            };
            if c0 != c2 as usize {
                *bucket_a_mut(bkt_a, c2 as usize) = k_idx as i32;
                c2 = c0 as isize;
                k_idx = bucket_a(bkt_a, c0) as usize;
            }
            sa[k_idx] = s_val;
            k_idx += 1;
        } else {
            sa[i] = !s;
        }
    }
}

/// Induces the Burrows-Wheeler transform from the sorted B*-suffixes,
/// similar to `construct_sa` but outputs BWT characters instead of suffix positions.
fn construct_bwt(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut BucketA,
    bkt_b: &mut BucketB,
    n: usize,
    m: usize,
) -> usize {
    if m > 0 {
        for c1 in (0..=(ALPHABET_SIZE - 2)).rev() {
            let i_start = bucket_bstar(bkt_b, c1, c1 + 1) as usize;
            let j_end = bucket_a(bkt_a, c1 + 1) as usize;

            let mut k_idx: usize = 0;
            let mut c2: isize = -1;

            let mut j = j_end as isize - 1;
            while i_start as isize <= j {
                let s = sa[j as usize];
                if s > 0 {
                    let s = s as usize;
                    let c0 = t[s - 1] as usize;
                    sa[j as usize] = !(c0 as i32);
                    let s_val = if s > 1 && t[s - 2] as usize > c0 {
                        !(s as i32 - 1)
                    } else {
                        s as i32 - 1
                    };
                    if c0 != c2 as usize {
                        if c2 >= 0 {
                            *bucket_b_mut(bkt_b, c2 as usize, c1) = k_idx as i32;
                        }
                        c2 = c0 as isize;
                        k_idx = bucket_b(bkt_b, c0, c1) as usize;
                    }
                    sa[k_idx] = s_val;
                    k_idx = k_idx.saturating_sub(1);
                } else if s != 0 {
                    sa[j as usize] = !s;
                }
                j -= 1;
            }
        }
    }

    let c2_init = t[n - 1] as usize;
    let mut k_idx = bucket_a(bkt_a, c2_init) as usize;
    sa[k_idx] = if (t[n - 2] as usize) < c2_init {
        !(t[n - 2] as i32)
    } else {
        (n - 1) as i32
    };
    k_idx += 1;
    let mut c2 = c2_init as isize;
    let mut orig = 0usize;

    for i in 0..n {
        let s = sa[i];
        if s > 0 {
            let s = s as usize;
            let c0 = t[s - 1] as usize;
            sa[i] = c0 as i32;
            let s_val = if s > 1 && (t[s - 2] as usize) < c0 {
                !(t[s - 2] as i32)
            } else {
                s as i32 - 1
            };
            if c0 != c2 as usize {
                *bucket_a_mut(bkt_a, c2 as usize) = k_idx as i32;
                c2 = c0 as isize;
                k_idx = bucket_a(bkt_a, c0) as usize;
            }
            sa[k_idx] = s_val;
            k_idx += 1;
        } else if s != 0 {
            sa[i] = !s;
        } else {
            orig = i;
        }
    }

    orig
}

/// Constructs the suffix array of `t` in O(n) time using the induced-sorting algorithm.
///
/// On success, `sa[i]` holds the starting position of the `i`-th lexicographically
/// smallest suffix of `t`.
///
/// # Errors
///
/// Returns [`DivSufSortError::InvalidArgument`] if `sa.len() != t.len()`.
pub fn divsufsort(t: &[u8], sa: &mut [i32]) -> Result<(), DivSufSortError> {
    let n = t.len();
    if sa.len() != n {
        return Err(DivSufSortError::InvalidArgument);
    }
    if n == 0 {
        return Ok(());
    }
    if n == 1 {
        sa[0] = 0;
        return Ok(());
    }
    if n == 2 {
        let m = (t[0] < t[1]) as usize;
        sa[m ^ 1] = 0;
        sa[m] = 1;
        return Ok(());
    }

    let mut bkt_a = zeroed_table::<BUCKET_A_SIZE>();
    let mut bkt_b = zeroed_table::<BUCKET_B_SIZE>();

    let m = sort_typebstar(t, sa, &mut bkt_a, &mut bkt_b, n);
    construct_sa(t, sa, &mut bkt_a, &mut bkt_b, n, m);

    Ok(())
}

/// Constructs the Burrows-Wheeler Transform (BWT) of `t` in O(n) time.
///
/// The BWT is written into `u`. The returned value is the primary index (1-based position
/// of the original string's last character in the BWT output), which is needed to invert
/// the transform.
///
/// `a` is an optional scratch buffer of length ≥ `t.len()`. If `None`, an internal
/// allocation is used.
///
/// # Errors
///
/// Returns [`DivSufSortError::InvalidArgument`] if a provided `a` buffer is too short.
pub fn divbwt(t: &[u8], u: &mut [u8], a: Option<&mut [i32]>) -> Result<i32, DivSufSortError> {
    let n = t.len();
    if n == 0 {
        return Ok(0);
    }
    if n == 1 {
        u[0] = t[0];
        return Ok(1);
    }

    let mut bkt_a = zeroed_table::<BUCKET_A_SIZE>();
    let mut bkt_b = zeroed_table::<BUCKET_B_SIZE>();

    if let Some(work) = &a
        && work.len() < n
    {
        return Err(DivSufSortError::InvalidArgument);
    }
    // Use the caller's scratch buffer when given; otherwise allocate one.
    let mut owned = Vec::new();
    let work: &mut [i32] = a.map_or_else(
        || {
            owned.resize(n, 0);
            &mut owned[..]
        },
        |work| &mut work[..n],
    );
    let m = sort_typebstar(t, work, &mut bkt_a, &mut bkt_b, n);
    let pidx = construct_bwt(t, work, &mut bkt_a, &mut bkt_b, n, m);
    u[0] = t[n - 1];
    for i in 0..pidx {
        u[i + 1] = work[i] as u8;
    }
    for i in (pidx + 1)..n {
        u[i] = work[i] as u8;
    }
    let pidx = pidx + 1;

    Ok(pidx as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::sufcheck;

    #[test]
    fn test_divsufsort_empty() {
        let t: &[u8] = b"";
        let mut sa: Vec<i32> = vec![];
        assert!(divsufsort(t, &mut sa).is_ok());
    }

    #[test]
    fn test_divsufsort_single() {
        let t = b"a";
        let mut sa = vec![0i32; 1];
        divsufsort(t, &mut sa).unwrap();
        assert_eq!(sa, vec![0]);
    }

    #[test]
    fn test_divsufsort_two_chars_sorted() {
        let t = b"ab";
        let mut sa = vec![0i32; 2];
        divsufsort(t, &mut sa).unwrap();
        assert_eq!(sa, vec![0, 1]);
    }

    #[test]
    fn test_divsufsort_two_chars_reverse() {
        let t = b"ba";
        let mut sa = vec![0i32; 2];
        divsufsort(t, &mut sa).unwrap();
        assert_eq!(sa, vec![1, 0]);
    }

    #[test]
    fn test_divsufsort_banana() {
        let t = b"banana";
        let mut sa = vec![0i32; t.len()];
        divsufsort(t, &mut sa).unwrap();
        assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn test_divsufsort_mississippi() {
        let t = b"mississippi";
        let mut sa = vec![0i32; t.len()];
        divsufsort(t, &mut sa).unwrap();
        assert_eq!(sa, vec![10, 7, 4, 1, 0, 9, 8, 6, 3, 5, 2]);
    }

    #[test]
    fn test_divsufsort_sufcheck_banana() {
        let t = b"banana";
        let mut sa = vec![0i32; t.len()];
        divsufsort(t, &mut sa).unwrap();
        sufcheck(t, &sa, false).unwrap();
    }

    #[test]
    fn test_divsufsort_sufcheck_mississippi() {
        let t = b"mississippi";
        let mut sa = vec![0i32; t.len()];
        divsufsort(t, &mut sa).unwrap();
        sufcheck(t, &sa, false).unwrap();
    }

    #[test]
    fn test_divsufsort_sufcheck_abracadabra() {
        let t = b"abracadabra";
        let mut sa = vec![0i32; t.len()];
        divsufsort(t, &mut sa).unwrap();
        sufcheck(t, &sa, false).unwrap();
    }

    #[test]
    fn test_divsufsort_all_same() {
        let t = b"aaaa";
        let mut sa = vec![0i32; t.len()];
        divsufsort(t, &mut sa).unwrap();
        sufcheck(t, &sa, false).unwrap();
    }

    #[test]
    fn test_divbwt_banana() {
        let t = b"banana";
        let mut u = vec![0u8; t.len()];
        let pidx = divbwt(t, &mut u, None).unwrap();
        // BWT of "banana" = "annbaa", pidx=4
        assert_eq!(&u, b"annbaa");
        assert_eq!(pidx, 4);
    }

    #[test]
    fn test_divbwt_roundtrip() {
        use crate::utils::inverse_bw_transform;
        let t = b"mississippi";
        let mut bwt = vec![0u8; t.len()];
        let pidx = divbwt(t, &mut bwt, None).unwrap();
        let mut restored = vec![0u8; t.len()];
        inverse_bw_transform(&bwt, &mut restored, None, pidx).unwrap();
        assert_eq!(restored, t.to_vec());
    }
}
