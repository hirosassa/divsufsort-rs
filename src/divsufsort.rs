use crate::DivSufSortError;
use crate::constants::{ALPHABET_SIZE, BUCKET_A_SIZE, BUCKET_B_SIZE};
use crate::sssort::{SsortCtx, sssort};
use crate::trsort::trsort;
use rayon::prelude::*;

#[inline(always)]
fn bucket_a(ba: &[i32], c0: usize) -> i32 {
    ba[c0]
}
#[inline(always)]
fn bucket_a_mut(ba: &mut [i32], c0: usize) -> &mut i32 {
    &mut ba[c0]
}

/// BUCKET_B(c0, c1) = bucket_B[(c1 << 8) | c0]
#[inline(always)]
fn bucket_b(bb: &[i32], c0: usize, c1: usize) -> i32 {
    bb[(c1 << 8) | c0]
}
#[inline(always)]
fn bucket_b_mut(bb: &mut [i32], c0: usize, c1: usize) -> &mut i32 {
    &mut bb[(c1 << 8) | c0]
}

/// BUCKET_BSTAR(c0, c1) = bucket_B[(c0 << 8) | c1]
#[inline(always)]
fn bucket_bstar(bb: &[i32], c0: usize, c1: usize) -> i32 {
    bb[(c0 << 8) | c1]
}
#[inline(always)]
fn bucket_bstar_mut(bb: &mut [i32], c0: usize, c1: usize) -> &mut i32 {
    &mut bb[(c0 << 8) | c1]
}

fn sort_typebstar(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut [i32],
    bkt_b: &mut [i32],
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
    m = n - m;

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
        // Safety invariants that make the raw-pointer aliasing sound:
        //   • Each job's sort range [i_val..j) is a disjoint subrange of sa[0..m].
        //     The ranges tile sa[0..m] without overlap (j of job k == i_val of job k−1).
        //   • sssort is called with bufsize=0, so it uses the tail of each job's own
        //     sort range [middle..j) as its merge buffer (middle = j − sqrt(j−i_val)).
        //     This buffer region is also within the job's exclusive [i_val..j) range.
        //   • pa = sa[pab..n+1] = sa[n−m..n+1] is read-only during sssort.
        //     Because m ≤ n/2, pab = n−m ≥ m, so [0..m) and [n−m..n+1) are disjoint.
        //   • sa[n] = 0 (sentinel, never overwritten) satisfies ss_compare's PA[m] read.
        //   • t is read-only throughout, satisfying Send + Sync for the closure.
        {
            // Convert the raw pointer to usize so that the closure is Send + Sync.
            // usize is Copy + Send + Sync; we cast back to *mut i32 inside each thread.
            let sa_addr: usize = sa.as_mut_ptr() as usize;
            let n1 = n + 1;
            jobs.par_iter().for_each(|&(i_val, j_val, lastsuffix)| {
                let sa_ptr = sa_addr as *mut i32;
                let sa_local = unsafe { std::slice::from_raw_parts_mut(sa_ptr, n1) };
                let pa_local = unsafe { std::slice::from_raw_parts(sa_ptr as *const i32, n1) };
                let ctx = SsortCtx {
                    t,
                    pa: pa_local,
                    pab,
                    depth: 2,
                    n: n as i32,
                };
                sssort(&ctx, sa_local, i_val, j_val, 0, 0, lastsuffix);
            });
        }

        {
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
                i -= 1; // corresponds to --i in the C for-loop
            }
        }

        {
            let (sa_left, sa_right) = sa.split_at_mut(m);
            // ISAb = sa[m..], SA = sa[0..m]
            trsort(sa_right, sa_left, m as i32, 1);
        }

        {
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

        // After scatter, SA[0..m] holds the sorted B*-suffix text positions needed by copy.
        // SA[m..n] contains stale data (ISAb, sssort buffer, PAb) no longer needed.
        // construct_sa requires every SA position that is NOT a B*-bucket endpoint to be 0.
        // Strategy: save the scatter output, zero all of SA[0..n], then run copy reading
        // from the saved scatter output so only the bucket endpoints get non-zero values.
        let scatter_out: Vec<i32> = sa[0..m].to_vec();
        for x in sa.iter_mut() {
            *x = 0;
        }
        *bucket_b_mut(bkt_b, ALPHABET_SIZE - 1, ALPHABET_SIZE - 1) = n as i32; // end point
        let mut k = m as isize - 1;
        for c0 in (0..=(ALPHABET_SIZE as isize - 2)).rev() {
            let c0 = c0 as usize;
            let mut i_val = bucket_a(bkt_a, c0 + 1) as isize - 1;
            for c1 in (c0 + 1..ALPHABET_SIZE).rev() {
                let t_val = i_val - bucket_b(bkt_b, c0, c1) as isize;
                *bucket_b_mut(bkt_b, c0, c1) = i_val as i32; // end point

                let j_val = bucket_bstar(bkt_b, c0, c1) as isize;
                let mut ii = t_val;
                let mut kk = k;
                while j_val <= kk {
                    sa[ii as usize] = scatter_out[kk as usize];
                    ii -= 1;
                    kk -= 1;
                }
                i_val = ii;
                k = kk;
            }
            *bucket_bstar_mut(bkt_b, c0, c0 + 1) =
                (i_val - bucket_b(bkt_b, c0, c0) as isize + 1) as i32; // start point
            *bucket_b_mut(bkt_b, c0, c0) = i_val as i32; // end point
        }
    }

    m
}

fn construct_sa(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut [i32],
    bkt_b: &mut [i32],
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

fn construct_bwt(
    t: &[u8],
    sa: &mut [i32],
    bkt_a: &mut [i32],
    bkt_b: &mut [i32],
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

    let mut bkt_a = vec![0i32; BUCKET_A_SIZE];
    let mut bkt_b = vec![0i32; BUCKET_B_SIZE];

    // Allocate one extra element so that sa[n] == 0 serves as a sentinel for ss_compare.
    // ss_compare reads pa[pab + k + 1] where k can be m-1 (the last B*-suffix index),
    // making the access pa[n] = sa[n].  The extra zero is never written by any algorithm
    // phase, so it remains 0 throughout.  The caller's sa[0..n] is copied back at the end.
    let mut sa_buf = vec![0i32; n + 1];
    let m = sort_typebstar(t, &mut sa_buf, &mut bkt_a, &mut bkt_b, n);
    construct_sa(t, &mut sa_buf, &mut bkt_a, &mut bkt_b, n, m);
    sa.copy_from_slice(&sa_buf[..n]);

    Ok(())
}

pub fn divbwt(t: &[u8], u: &mut [u8], a: Option<&mut [i32]>) -> Result<i32, DivSufSortError> {
    let n = t.len();
    if n == 0 {
        return Ok(0);
    }
    if n == 1 {
        u[0] = t[0];
        return Ok(1);
    }

    let mut bkt_a = vec![0i32; BUCKET_A_SIZE];
    let mut bkt_b = vec![0i32; BUCKET_B_SIZE];

    // Always use an n+1 internal buffer so that sa[n] == 0 is available as a PAb sentinel
    // (see divsufsort for the full explanation).  The optional `a` parameter was previously
    // used as scratch space to avoid this allocation, but it could only hold n elements,
    // one short of the n+1 needed.
    if let Some(b_arr) = &a
        && b_arr.len() < n
    {
        return Err(DivSufSortError::InvalidArgument);
    }
    let mut b_arr = vec![0i32; n + 1];
    let m = sort_typebstar(t, &mut b_arr, &mut bkt_a, &mut bkt_b, n);
    let pidx = construct_bwt(t, &mut b_arr, &mut bkt_a, &mut bkt_b, n, m);
    u[0] = t[n - 1];
    for i in 0..pidx {
        u[i + 1] = b_arr[i] as u8;
    }
    for i in (pidx + 1)..n {
        u[i] = b_arr[i] as u8;
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
