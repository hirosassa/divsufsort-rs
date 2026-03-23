//! Suffix-suffix sort (sssort): merge-based sorting for B*-suffixes.
//!
//! Implements Larsson-Sadakane style sorting using introspective sort
//! (`ss_mintrosort`) with heapsort fallback, and a merge phase
//! (`ss_swapmerge`) that combines sorted blocks using forward/backward merging.

use crate::constants::{
    Depth, FixedStack, LG_TABLE, SS_BLOCKSIZE, SS_INSERTIONSORT_THRESHOLD, SS_MISORT_STACKSIZE,
    SS_SMERGE_STACKSIZE,
};

/// Access PA (= SA[pab..]) at a possibly-negative index, replicating C's pointer arithmetic.
/// In C: `PA[idx]` = `SA[pab + idx]`, even when idx is negative (equal-group markers).
#[inline(always)]
fn pa_val(pa: &[i32], pab: usize, idx: i32) -> i32 {
    // SAFETY: `pa` is the full SA buffer of length n+1. `pab = n − m` where m ≤ n/2.
    // `idx` originates from SA elements, which fall into exactly three cases:
    //   (a) Non-negative suffix index k ∈ [0, m−1]:
    //       abs = pab + k ∈ [n−m, n−1]  ⊂  [0, n+1)
    //   (b) Equal-group marker !k for k ∈ [0, m−1]  (i.e. idx = −(k+1)):
    //       abs = pab − k − 1 ∈ [n−2m, n−m−1].
    //       Lower bound ≥ 0 because m ≤ n/2 ⟹ n−2m ≥ 0.  ⊂  [0, n+1)
    //   (c) One-past-end probe k+1 for k ∈ [0, m−1]  (ss_partition boundary check):
    //       abs = pab + k + 1 ∈ [n−m+1, n].
    //       Upper bound = n < n+1 = pa.len(), reaching the sentinel sa[n] = 0.  ⊂  [0, n+1)
    // All three cases produce abs ∈ [0, n] ⊂ [0, n+1), so get_unchecked is sound.
    let abs = (pab as i64 + idx as i64) as usize;
    unsafe { *pa.get_unchecked(abs) }
}

static SQQ_TABLE: [i32; 256] = [
    0, 16, 22, 27, 32, 35, 39, 42, 45, 48, 50, 53, 55, 57, 59, 61, 64, 65, 67, 69, 71, 73, 75, 76,
    78, 80, 81, 83, 84, 86, 87, 89, 90, 91, 93, 94, 96, 97, 98, 99, 101, 102, 103, 104, 106, 107,
    108, 109, 110, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 128,
    128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 144, 145,
    146, 147, 148, 149, 150, 150, 151, 152, 153, 154, 155, 155, 156, 157, 158, 159, 160, 160, 161,
    162, 163, 163, 164, 165, 166, 167, 167, 168, 169, 170, 170, 171, 172, 173, 173, 174, 175, 176,
    176, 177, 178, 178, 179, 180, 181, 181, 182, 183, 183, 184, 185, 185, 186, 187, 187, 188, 189,
    189, 190, 191, 192, 192, 193, 193, 194, 195, 195, 196, 197, 197, 198, 199, 199, 200, 201, 201,
    202, 203, 203, 204, 204, 205, 206, 206, 207, 208, 208, 209, 209, 210, 211, 211, 212, 212, 213,
    214, 214, 215, 215, 216, 217, 217, 218, 218, 219, 219, 220, 221, 221, 222, 222, 223, 224, 224,
    225, 225, 226, 226, 227, 227, 228, 229, 229, 230, 230, 231, 231, 232, 232, 233, 234, 234, 235,
    235, 236, 236, 237, 237, 238, 238, 239, 240, 240, 241, 241, 242, 242, 243, 243, 244, 244, 245,
    245, 246, 246, 247, 247, 248, 248, 249, 249, 250, 250, 251, 251, 252, 252, 253, 253, 254, 254,
    255,
];

#[inline(always)]
/// Integer log2 for 16-bit values, using the lookup table.
fn ss_ilg(n: i32) -> i32 {
    if n & 0xff00 != 0 {
        8 + LG_TABLE[((n >> 8) & 0xff) as usize]
    } else {
        LG_TABLE[(n & 0xff) as usize]
    }
}

#[inline(always)]
/// Integer square root using the SQQ_TABLE lookup, clamped to SS_BLOCKSIZE.
fn ss_isqrt(x: i32) -> i32 {
    let blocksize = SS_BLOCKSIZE as i32;
    if x >= blocksize * blocksize {
        return blocksize;
    }
    let e = if x & 0xffff0000u32 as i32 != 0 {
        if x & 0xff000000u32 as i32 != 0 {
            24 + LG_TABLE[((x >> 24) & 0xff) as usize]
        } else {
            16 + LG_TABLE[((x >> 16) & 0xff) as usize]
        }
    } else if x & 0x0000ff00 != 0 {
        8 + LG_TABLE[((x >> 8) & 0xff) as usize]
    } else {
        LG_TABLE[(x & 0xff) as usize]
    };

    let y = if e >= 16 {
        let mut y = SQQ_TABLE[(x >> ((e - 6) - (e & 1))) as usize] << ((e >> 1) - 7);
        if e >= 24 {
            y = (y + 1 + x / y) >> 1;
        }
        (y + 1 + x / y) >> 1
    } else if e >= 8 {
        (SQQ_TABLE[(x >> ((e - 6) - (e & 1))) as usize] >> (7 - (e >> 1))) + 1
    } else {
        return SQQ_TABLE[x as usize] >> 4;
    };

    if x < y * y { y - 1 } else { y }
}

/// Corresponds to ss_compare in C.
/// p1 and p2 are separate slices (for lastsuffix, PAi and PA are different slices).
/// p1_idx and p2_idx are indices within their respective slices.
#[inline(always)]
fn ss_compare(t: &[u8], p1: &[i32], p1_idx: usize, p2: &[i32], p2_idx: usize, depth: Depth) -> i32 {
    let u1_start = (depth + p1[p1_idx]) as usize;
    let u2_start = (depth + p2[p2_idx]) as usize;
    let u1n = (p1[p1_idx + 1] + 2) as usize;
    let u2n = (p2[p2_idx + 1] + 2) as usize;

    let mut u1 = u1_start;
    let mut u2 = u2_start;
    while u1 < u1n && u2 < u2n && t[u1] == t[u2] {
        u1 += 1;
        u2 += 1;
    }

    if u1 < u1n {
        if u2 < u2n {
            t[u1] as i32 - t[u2] as i32
        } else {
            1
        }
    } else if u2 < u2n {
        -1
    } else {
        0
    }
}

/// Insertion sort for small ranges within the suffix sort, skipping negative group markers.
fn ss_insertionsort_impl(
    t: &[u8],
    pa: &[i32],
    sa: &mut [i32],
    first: usize,
    last: usize,
    depth: Depth,
) {
    if last <= first + 1 {
        return;
    }
    let mut i = last as isize - 2;
    while first as isize <= i {
        let tv = sa[i as usize];
        let mut j = i as usize + 1;
        let r: i32 = 'outer: loop {
            // if sa[j] is negative, !sa[j] is the actual value (equivalent to PA + *j in C)
            let sj = if sa[j] >= 0 {
                sa[j] as usize
            } else {
                !sa[j] as usize
            };
            let r = ss_compare(t, pa, tv as usize, pa, sj, depth);
            if r <= 0 {
                break r;
            }
            // r > 0: shift sa[j] into sa[j-1] and advance, skipping over negative elements
            loop {
                sa[j - 1] = sa[j];
                j += 1;
                if j >= last {
                    break 'outer 1;
                }
                if sa[j] >= 0 {
                    break;
                }
                // negative: continue shifting
            }
        };
        if r == 0 {
            sa[j] = !sa[j];
        }
        sa[j - 1] = tv;
        i -= 1;
    }
}

/// Sift-down operation for the max-heap used by `ss_heapsort`.
fn ss_fixdown(td: &[u8], pa: &[i32], pab: usize, sa: &mut [i32], mut i: usize, size: usize) {
    let v = sa[i];
    let c = td[pa_val(pa, pab, v) as usize];
    loop {
        let j = 2 * i + 1;
        if j >= size {
            break;
        }
        let mut k = j;
        let d_k = td[pa_val(pa, pab, sa[k]) as usize];
        let j1 = j + 1;
        let d = if j1 < size {
            let e = td[pa_val(pa, pab, sa[j1]) as usize];
            if (d_k as i32) < (e as i32) {
                k = j1;
                e
            } else {
                d_k
            }
        } else {
            d_k
        };
        if (d as i32) <= (c as i32) {
            break;
        }
        sa[i] = sa[k];
        i = k;
    }
    sa[i] = v;
}

/// Heapsort fallback for `ss_mintrosort` when the introsort recursion limit is exhausted.
fn ss_heapsort(td: &[u8], pa: &[i32], pab: usize, sa: &mut [i32], first: usize, size: usize) {
    let mut m = size;
    if size.is_multiple_of(2) {
        m -= 1;
        if td[pa_val(pa, pab, sa[first + m / 2]) as usize]
            < td[pa_val(pa, pab, sa[first + m]) as usize]
        {
            sa.swap(first + m, first + m / 2);
        }
    }
    if m >= 2 {
        let mut i = m / 2;
        loop {
            ss_fixdown(td, pa, pab, &mut sa[first..], i, m);
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
    if size.is_multiple_of(2) {
        sa.swap(first, first + m);
        ss_fixdown(td, pa, pab, &mut sa[first..], 0, m);
    }
    let mut i = m;
    while i > 0 {
        i -= 1;
        let t = sa[first];
        sa[first] = sa[first + i];
        ss_fixdown(td, pa, pab, &mut sa[first..], 0, i);
        sa[first + i] = t;
    }
}

#[inline(always)]
fn ss_median3_idx(
    td: &[u8],
    pa: &[i32],
    pab: usize,
    sa: &[i32],
    mut v1: usize,
    mut v2: usize,
    v3: usize,
) -> usize {
    if td[pa_val(pa, pab, sa[v1]) as usize] > td[pa_val(pa, pab, sa[v2]) as usize] {
        std::mem::swap(&mut v1, &mut v2);
    }
    if td[pa_val(pa, pab, sa[v2]) as usize] > td[pa_val(pa, pab, sa[v3]) as usize] {
        if td[pa_val(pa, pab, sa[v1]) as usize] > td[pa_val(pa, pab, sa[v3]) as usize] {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

#[inline(always)]
fn ss_median5_idx(td: &[u8], pa: &[i32], pab: usize, sa: &[i32], v: [usize; 5]) -> usize {
    let [mut v1, mut v2, mut v3, mut v4, mut v5] = v;
    if td[pa_val(pa, pab, sa[v2]) as usize] > td[pa_val(pa, pab, sa[v3]) as usize] {
        std::mem::swap(&mut v2, &mut v3);
    }
    if td[pa_val(pa, pab, sa[v4]) as usize] > td[pa_val(pa, pab, sa[v5]) as usize] {
        std::mem::swap(&mut v4, &mut v5);
    }
    if td[pa_val(pa, pab, sa[v2]) as usize] > td[pa_val(pa, pab, sa[v4]) as usize] {
        std::mem::swap(&mut v2, &mut v4);
        std::mem::swap(&mut v3, &mut v5);
    }
    if td[pa_val(pa, pab, sa[v1]) as usize] > td[pa_val(pa, pab, sa[v3]) as usize] {
        std::mem::swap(&mut v1, &mut v3);
    }
    if td[pa_val(pa, pab, sa[v1]) as usize] > td[pa_val(pa, pab, sa[v4]) as usize] {
        std::mem::swap(&mut v1, &mut v4);
        std::mem::swap(&mut v3, &mut v5);
    }
    if td[pa_val(pa, pab, sa[v3]) as usize] > td[pa_val(pa, pab, sa[v4]) as usize] {
        v4
    } else {
        v3
    }
}

/// Selects a pivot index using median-of-3 or median-of-5 depending on range size.
fn ss_pivot_idx(td: &[u8], pa: &[i32], pab: usize, sa: &[i32], first: usize, last: usize) -> usize {
    let t = (last - first) as i32;
    let middle = first + (t / 2) as usize;
    if t <= 512 {
        if t <= 32 {
            ss_median3_idx(td, pa, pab, sa, first, middle, last - 1)
        } else {
            let t2 = (t >> 2) as usize;
            ss_median5_idx(
                td,
                pa,
                pab,
                sa,
                [first, first + t2, middle, last - 1 - t2, last - 1],
            )
        }
    } else {
        let t2 = (t >> 3) as usize;
        let f = ss_median3_idx(td, pa, pab, sa, first, first + t2, first + 2 * t2);
        let m = ss_median3_idx(td, pa, pab, sa, middle - t2, middle, middle + t2);
        let l = ss_median3_idx(td, pa, pab, sa, last - 1 - 2 * t2, last - 1 - t2, last - 1);
        ss_median3_idx(td, pa, pab, sa, f, m, l)
    }
}

/// Partitions [first..last) into two parts and returns the boundary index.
/// pa is the full SA snapshot; pab is the PAb offset (PA = SA[pab..]).
fn ss_partition(
    pa: &[i32],
    pab: usize,
    sa: &mut [i32],
    first: usize,
    last: usize,
    depth: Depth,
) -> usize {
    let mut a = first;
    let mut b = last;
    loop {
        // C: for(; (++a < b) && ((PA[*a] + depth) >= (PA[*a + 1] + 1));) { *a = ~*a; }
        loop {
            if a >= b {
                break;
            }
            let v = sa[a];
            if (pa_val(pa, pab, v) + depth) < (pa_val(pa, pab, v + 1) + 1) {
                break;
            }
            sa[a] = !v;
            a += 1;
        }
        // C: for(; (a < --b) && ((PA[*b] + depth) < (PA[*b + 1] + 1));) { }
        loop {
            if b <= a {
                break;
            }
            b -= 1;
            let v = sa[b];
            if (pa_val(pa, pab, v) + depth) >= (pa_val(pa, pab, v + 1) + 1) {
                break;
            }
        }
        if b <= a {
            break;
        }
        let t = !sa[b];
        sa[b] = sa[a];
        sa[a] = t;
        a += 1;
    }
    if first < a {
        sa[first] = !sa[first];
    }
    a
}

#[derive(Clone, Copy, Default)]
struct MiSortFrame {
    first: usize,
    last: usize,
    depth: Depth,
    limit: i32,
}

/// Bentley-McIlroy 3-way partition on sa[first..last] around pivot value `v`.
/// Assumes sa[first] already holds the pivot element (swapped there by the caller).
///
/// Returns `Some((lt_end, eq_part, gt_start))` on success:
///   - sa[first..lt_end]: elements < v
///   - sa[lt_end..gt_start]: elements == v (eq_part is the ss_partition boundary within)
///   - sa[gt_start..last]: elements > v
///
/// Returns `None` if partition is degenerate (no equal elements spanning both sides).
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn ss_three_way_partition(
    t: &[u8],
    pa: &[i32],
    pab: usize,
    sa: &mut [i32],
    first: usize,
    last: usize,
    depth: Depth,
    v: i32,
) -> Option<(usize, usize, usize)> {
    let td_offset = depth as usize;
    // Left scan: for(b = first; (++b < last) && ((x = Td[PA[*b]]) == v);)
    let mut b = first + 1;
    let mut x = 0i32;
    while b < last {
        x = t[td_offset + pa_val(pa, pab, sa[b]) as usize] as i32;
        if x != v {
            break;
        }
        b += 1;
    }
    let mut a = b;
    // if(((a = b) < last) && (x < v))
    if a < last && x < v {
        // for(; (++b < last) && ((x = Td[PA[*b]]) <= v);) { if x==v swap }
        loop {
            b += 1;
            if b >= last {
                break;
            }
            x = t[td_offset + pa_val(pa, pab, sa[b]) as usize] as i32;
            if x > v {
                break;
            }
            if x == v {
                sa.swap(b, a);
                a += 1;
            }
        }
    }
    // Right scan: for(c = last; (b < --c) && ((x = Td[PA[*c]]) == v);)
    let mut c = last;
    loop {
        if c == 0 {
            break;
        }
        c -= 1;
        if c <= b {
            break;
        }
        x = t[td_offset + pa_val(pa, pab, sa[c]) as usize] as i32;
        if x != v {
            break;
        }
    }
    // d = c  (C: if((b < (d = c)) && (x > v)))
    let mut d = c;
    if b < c && x > v {
        // for(; (b < --c) && ((x = Td[PA[*c]]) >= v);) { if x==v SWAP(*c,*d); --d }
        loop {
            if c == 0 {
                break;
            }
            c -= 1;
            if c <= b {
                break;
            }
            x = t[td_offset + pa_val(pa, pab, sa[c]) as usize] as i32;
            if x < v {
                break;
            }
            if x == v {
                sa.swap(c, d);
                d -= 1;
            }
        }
    }
    // Main loop: for(; b < c;)
    while b < c {
        sa.swap(b, c);
        // Inner left: for(; (++b < c) && ((x = Td[PA[*b]]) <= v);) { if x==v swap }
        loop {
            b += 1;
            if b >= c {
                break;
            }
            x = t[td_offset + pa_val(pa, pab, sa[b]) as usize] as i32;
            if x > v {
                break;
            }
            if x == v {
                sa.swap(b, a);
                a += 1;
            }
        }
        // Inner right: for(; (b < --c) && ((x = Td[PA[*c]]) >= v);) { if x==v SWAP(*c,*d); --d }
        loop {
            if c == 0 {
                break;
            }
            c -= 1;
            if c <= b {
                break;
            }
            x = t[td_offset + pa_val(pa, pab, sa[c]) as usize] as i32;
            if x < v {
                break;
            }
            if x == v {
                sa.swap(c, d);
                d -= 1;
            }
        }
    }

    if a > d {
        return None;
    }

    // Block swap: move equals from edges to center
    // C: c = b - 1 (reset for blockswap)
    let c_new = b - 1;
    let s = (a - first).min(b - a);
    for k in 0..s {
        sa.swap(first + k, b - s + k);
    }
    // C uses signed: if((s = d - c) > (t = last - d - 1)) { s = t; }
    let dc = d as i64 - c_new as i64; // d - (b - 1)
    let ldd = last as i64 - d as i64 - 1; // last - d - 1 (can be negative)
    let s2 = if dc > 0 && ldd > 0 {
        dc.min(ldd) as usize
    } else {
        0
    };
    for k in 0..s2 {
        sa.swap(b + k, last - s2 + k);
    }

    let new_a = first + (b - a);
    let new_c = (last as i64 - dc) as usize;

    let check_idx_ba = td_offset as i64 + pa_val(pa, pab, sa[new_a]) as i64 - 1;
    let b2 = if v <= t[check_idx_ba as usize] as i32 {
        new_a
    } else {
        ss_partition(pa, pab, sa, new_a, new_c, depth)
    };

    Some((new_a, b2, new_c))
}

/// Introspective sort for B*-suffix subranges.
///
/// Uses quicksort with 3-way partitioning (Bentley-McIlroy scheme),
/// falling back to heapsort when the recursion depth limit is exhausted.
/// Small ranges (≤ SS_INSERTIONSORT_THRESHOLD) use insertion sort.
fn ss_mintrosort(
    t: &[u8],
    pa: &[i32],
    pab: usize,
    sa: &mut [i32],
    first: usize,
    last: usize,
    depth: Depth,
) {
    let mut stack = FixedStack::<MiSortFrame, { SS_MISORT_STACKSIZE }>::new();
    let pa_slice = &pa[pab..]; // for insertionsort (uses non-negative PAb indices)

    let mut first = first;
    let mut last = last;
    let mut depth = depth;
    let mut limit = ss_ilg((last - first) as i32);

    loop {
        if last - first <= SS_INSERTIONSORT_THRESHOLD {
            if last - first > 1 {
                ss_insertionsort_impl(t, pa_slice, sa, first, last, depth);
            }
            if let Some(f) = stack.pop() {
                first = f.first;
                last = f.last;
                depth = f.depth;
                limit = f.limit;
            } else {
                return;
            }
            continue;
        }

        let td_offset = depth as usize;
        limit -= 1;
        if limit == -1 {
            ss_heapsort(&t[td_offset..], pa, pab, sa, first, last - first);
        }
        if limit < 0 {
            let mut a = first + 1;
            let mut v = t[td_offset + pa_val(pa, pab, sa[first]) as usize] as i32;
            while a < last {
                let x = t[td_offset + pa_val(pa, pab, sa[a]) as usize] as i32;
                if x != v {
                    if a - first > 1 {
                        break;
                    }
                    v = x;
                    first = a;
                }
                a += 1;
            }
            let check_idx_hs = td_offset as i64 + pa_val(pa, pab, sa[first]) as i64 - 1;
            if (t[check_idx_hs as usize] as i32) < v {
                first = ss_partition(pa, pab, sa, first, a, depth);
            }
            if a - first <= last - a {
                if a - first > 1 {
                    stack.push(MiSortFrame {
                        first: a,
                        last,
                        depth,
                        limit: -1,
                    });
                    last = a;
                    depth += 1;
                    limit = ss_ilg((a - first) as i32);
                } else {
                    first = a;
                    limit = -1;
                }
            } else if last - a > 1 {
                stack.push(MiSortFrame {
                    first,
                    last: a,
                    depth: depth + 1,
                    limit: ss_ilg((a - first) as i32),
                });
                first = a;
                limit = -1;
            } else {
                last = a;
                depth += 1;
                limit = ss_ilg((a - first) as i32);
            }
            continue;
        }

        let pivot_idx = ss_pivot_idx(&t[td_offset..], pa, pab, sa, first, last);
        let v = t[td_offset + pa_val(pa, pab, sa[pivot_idx]) as usize] as i32;
        sa.swap(first, pivot_idx);

        if let Some((new_a, b2, new_c)) =
            ss_three_way_partition(t, pa, pab, sa, first, last, depth, v)
        {
            // Push three sub-problems [first..new_a), [b2..new_c), [new_c..last)
            // in size order (smallest processed next via loop vars, larger two on stack).
            if new_a - first <= last - new_c {
                if last - new_c <= new_c - b2 {
                    stack.push(MiSortFrame {
                        first: b2,
                        last: new_c,
                        depth: depth + 1,
                        limit: ss_ilg((new_c - b2) as i32),
                    });
                    stack.push(MiSortFrame {
                        first: new_c,
                        last,
                        depth,
                        limit,
                    });
                    last = new_a;
                } else if new_a - first <= new_c - b2 {
                    stack.push(MiSortFrame {
                        first: new_c,
                        last,
                        depth,
                        limit,
                    });
                    stack.push(MiSortFrame {
                        first: b2,
                        last: new_c,
                        depth: depth + 1,
                        limit: ss_ilg((new_c - b2) as i32),
                    });
                    last = new_a;
                } else {
                    stack.push(MiSortFrame {
                        first: new_c,
                        last,
                        depth,
                        limit,
                    });
                    stack.push(MiSortFrame {
                        first,
                        last: new_a,
                        depth,
                        limit,
                    });
                    first = b2;
                    last = new_c;
                    depth += 1;
                    limit = ss_ilg((new_c - b2) as i32);
                }
            } else if new_a - first <= new_c - b2 {
                stack.push(MiSortFrame {
                    first: b2,
                    last: new_c,
                    depth: depth + 1,
                    limit: ss_ilg((new_c - b2) as i32),
                });
                stack.push(MiSortFrame {
                    first,
                    last: new_a,
                    depth,
                    limit,
                });
                first = new_c;
            } else if last - new_c <= new_c - b2 {
                stack.push(MiSortFrame {
                    first,
                    last: new_a,
                    depth,
                    limit,
                });
                stack.push(MiSortFrame {
                    first: b2,
                    last: new_c,
                    depth: depth + 1,
                    limit: ss_ilg((new_c - b2) as i32),
                });
                first = new_c;
            } else {
                stack.push(MiSortFrame {
                    first,
                    last: new_a,
                    depth,
                    limit,
                });
                stack.push(MiSortFrame {
                    first: new_c,
                    last,
                    depth,
                    limit,
                });
                first = b2;
                last = new_c;
                depth += 1;
                limit = ss_ilg((new_c - b2) as i32);
            }
        } else {
            limit += 1;
            let check_idx_el = td_offset as i64 + pa_val(pa, pab, sa[first]) as i64 - 1;
            if (t[check_idx_el as usize] as i32) < v {
                first = ss_partition(pa, pab, sa, first, last, depth);
                limit = ss_ilg((last - first) as i32);
            }
            depth += 1;
        }
    }
}

fn ss_blockswap(sa: &mut [i32], a: usize, b: usize, n: usize) {
    for k in 0..n {
        sa.swap(a + k, b + k);
    }
}

fn ss_rotate(sa: &mut [i32], first: usize, middle: usize, last: usize) {
    // Implements the rotation using the same algorithm as C (gcd-based in-place rotation)
    // Simpler: use std rotate equivalent
    sa[first..last].rotate_left(middle - first);
}

/// In-place merge of two adjacent sorted runs using binary search and rotation.
fn ss_inplacemerge(
    t: &[u8],
    pa: &[i32],
    sa: &mut [i32],
    first: usize,
    middle: usize,
    last: usize,
    depth: Depth,
) {
    let mut middle = middle;
    let mut last = last;
    loop {
        let (x, p) = if sa[last - 1] < 0 {
            (1i32, !sa[last - 1] as usize)
        } else {
            (0i32, sa[last - 1] as usize)
        };

        let mut a = first;
        let mut len = (middle - first) as i32;
        let mut half = len >> 1;
        let mut r = -1i32;
        while 0 < len {
            let b = a + half as usize;
            let bv = if sa[b] >= 0 {
                sa[b] as usize
            } else {
                !sa[b] as usize
            };
            let q = ss_compare(t, pa, bv, pa, p, depth);
            if q < 0 {
                a = b + 1;
                half -= (len & 1) ^ 1;
            } else {
                r = q;
            }
            len = half;
            half >>= 1;
        }
        if a < middle {
            if r == 0 {
                sa[a] = !sa[a];
            }
            ss_rotate(sa, a, middle, last);
            last -= middle - a;
            middle = a;
            if first == middle {
                break;
            }
        }
        last -= 1;
        if x != 0 {
            while sa[last - 1] < 0 {
                last -= 1;
            }
        }
        if middle == last {
            break;
        }
    }
}

struct MergeRange {
    first: usize,
    middle: usize,
    last: usize,
}

struct BufInfo {
    buf: usize,
    bufsize: usize,
}

/// Two-way merge in ascending direction using a temporary buffer.
/// Merges sa[first..middle) and sa[middle..last) by copying the left half into `buf`.
fn ss_mergeforward(
    t: &[u8],
    pa: &[i32],
    sa: &mut [i32],
    range: MergeRange,
    buf: usize,
    depth: Depth,
) {
    let MergeRange {
        first,
        middle,
        last,
    } = range;
    let bufend = buf + (middle - first) - 1;
    ss_blockswap(sa, buf, first, middle - first);

    let mut a = first;
    let mut b = buf;
    let mut c = middle;
    let tv = sa[a];

    loop {
        let r = ss_compare(t, pa, sa[b] as usize, pa, sa[c] as usize, depth);
        if r < 0 {
            loop {
                sa[a] = sa[b];
                a += 1;
                if bufend <= b {
                    sa[bufend] = tv;
                    return;
                }
                sa[b] = sa[a];
                b += 1;
                if sa[b] >= 0 {
                    break;
                }
            }
        } else if r > 0 {
            loop {
                sa[a] = sa[c];
                a += 1;
                sa[c] = sa[a];
                c += 1;
                if c >= last {
                    while b < bufend {
                        sa[a] = sa[b];
                        a += 1;
                        sa[b] = sa[a];
                        b += 1;
                    }
                    sa[a] = sa[b];
                    sa[b] = tv;
                    return;
                }
                if sa[c] >= 0 {
                    break;
                }
            }
        } else {
            sa[c] = !sa[c];
            loop {
                sa[a] = sa[b];
                a += 1;
                if bufend <= b {
                    sa[bufend] = tv;
                    return;
                }
                sa[b] = sa[a];
                b += 1;
                if sa[b] >= 0 {
                    break;
                }
            }
            loop {
                sa[a] = sa[c];
                a += 1;
                sa[c] = sa[a];
                c += 1;
                if c >= last {
                    while b < bufend {
                        sa[a] = sa[b];
                        a += 1;
                        sa[b] = sa[a];
                        b += 1;
                    }
                    sa[a] = sa[b];
                    sa[b] = tv;
                    return;
                }
                if sa[c] >= 0 {
                    break;
                }
            }
        }
    }
}

/// Bit flag: the buf-side element (sa[b]) is a negative group marker.
const MERGE_BUF_NEGATIVE: i32 = 1;
/// Bit flag: the middle-side element (sa[c]) is a negative group marker.
const MERGE_MID_NEGATIVE: i32 = 2;

/// Skips over negative group markers in sa, copying elements backward from
/// position `src` toward `limit`, writing into `dest` (also moving backward).
/// Returns the updated `(dest, src)` positions.
#[inline(always)]
fn skip_negatives_backward(
    sa: &mut [i32],
    mut dest: usize,
    mut src: usize,
    limit: usize,
) -> (usize, usize) {
    loop {
        sa[dest] = sa[src];
        dest = dest.saturating_sub(1);
        sa[src] = sa[dest];
        if src > limit {
            src -= 1;
        } else {
            break;
        }
        if sa[src] >= 0 {
            break;
        }
    }
    (dest, src)
}

/// Two-way merge in descending direction using a temporary buffer.
/// Merges sa[first..middle) and sa[middle..last) by copying the right half into `buf`.
fn ss_mergebackward(
    t: &[u8],
    pa: &[i32],
    sa: &mut [i32],
    range: MergeRange,
    buf: usize,
    depth: Depth,
) {
    let MergeRange {
        first,
        middle,
        last,
    } = range;
    let bufend = buf + (last - middle) - 1;
    ss_blockswap(sa, buf, middle, last - middle);

    let mut x = 0i32;
    let p1 = if sa[bufend] < 0 {
        x |= MERGE_BUF_NEGATIVE;
        !sa[bufend] as usize
    } else {
        sa[bufend] as usize
    };
    let p2 = if sa[middle - 1] < 0 {
        x |= MERGE_MID_NEGATIVE;
        !sa[middle - 1] as usize
    } else {
        sa[middle - 1] as usize
    };

    let tv = sa[last - 1];
    let mut a = last - 1;
    let mut b = bufend;
    let mut c = middle - 1;
    let mut p1 = p1;
    let mut p2 = p2;

    loop {
        let r = ss_compare(t, pa, p1, pa, p2, depth);
        if r > 0 {
            if x & MERGE_BUF_NEGATIVE != 0 {
                (a, b) = skip_negatives_backward(sa, a, b, buf);
                x ^= MERGE_BUF_NEGATIVE;
            }
            sa[a] = sa[b];
            a = a.saturating_sub(1);
            if b <= buf {
                sa[buf] = tv;
                break;
            }
            b -= 1;
            sa[b + 1] = sa[a]; // *b-- = *a  (b already decremented, so b+1 = b_old)
            p1 = if sa[b] < 0 {
                x |= MERGE_BUF_NEGATIVE;
                !sa[b] as usize
            } else {
                sa[b] as usize
            };
        } else if r < 0 {
            if x & MERGE_MID_NEGATIVE != 0 {
                (a, c) = skip_negatives_backward(sa, a, c, first);
                x ^= MERGE_MID_NEGATIVE;
            }
            sa[a] = sa[c];
            a = a.saturating_sub(1);
            sa[c] = sa[a];
            if c == 0 || c <= first {
                while buf < b {
                    sa[a] = sa[b];
                    a = a.saturating_sub(1);
                    sa[b] = sa[a];
                    if b > buf {
                        b -= 1;
                    }
                }
                sa[a] = sa[b];
                sa[b] = tv;
                break;
            }
            c -= 1;
            p2 = if sa[c] < 0 {
                x |= MERGE_MID_NEGATIVE;
                !sa[c] as usize
            } else {
                sa[c] as usize
            };
        } else {
            if x & MERGE_BUF_NEGATIVE != 0 {
                (a, b) = skip_negatives_backward(sa, a, b, buf);
                x ^= MERGE_BUF_NEGATIVE;
            }
            sa[a] = !sa[b];
            a = a.saturating_sub(1);
            if b <= buf {
                sa[buf] = tv;
                break;
            }
            b -= 1;
            sa[b + 1] = sa[a]; // b already decremented, so b+1 = b_old
            if x & MERGE_MID_NEGATIVE != 0 {
                (a, c) = skip_negatives_backward(sa, a, c, first);
                x ^= MERGE_MID_NEGATIVE;
            }
            sa[a] = sa[c];
            a = a.saturating_sub(1);
            sa[c] = sa[a];
            if c == 0 || c <= first {
                while buf < b {
                    sa[a] = sa[b];
                    a = a.saturating_sub(1);
                    sa[b] = sa[a];
                    if b > buf {
                        b -= 1;
                    }
                }
                sa[a] = sa[b];
                sa[b] = tv;
                break;
            }
            c -= 1;
            p1 = if sa[b] < 0 {
                x |= MERGE_BUF_NEGATIVE;
                !sa[b] as usize
            } else {
                sa[b] as usize
            };
            p2 = if sa[c] < 0 {
                x |= MERGE_MID_NEGATIVE;
                !sa[c] as usize
            } else {
                sa[c] as usize
            };
        }
    }
}

#[derive(Clone, Copy, Default)]
struct SmergeFrame {
    first: usize,
    middle: usize,
    last: usize,
    check: i32,
}

/// Merge phase that combines sorted blocks using a temporary buffer.
/// Chooses between forward merge, backward merge, and in-place merge
/// based on block sizes and available buffer space.
fn ss_swapmerge(
    t: &[u8],
    pa: &[i32],
    sa: &mut [i32],
    range: MergeRange,
    buf_info: BufInfo,
    depth: Depth,
) {
    let BufInfo { buf, bufsize } = buf_info;
    let mut stack = FixedStack::<SmergeFrame, { SS_SMERGE_STACKSIZE }>::new();

    let mut first = range.first;
    let mut middle = range.middle;
    let mut last = range.last;
    let mut check = 0i32;

    loop {
        if last - middle <= bufsize {
            if first < middle && middle < last {
                ss_mergebackward(
                    t,
                    pa,
                    sa,
                    MergeRange {
                        first,
                        middle,
                        last,
                    },
                    buf,
                    depth,
                );
            }
            // MERGE_CHECK
            merge_check(t, pa, sa, first, last, check, depth);
            // STACK_POP
            if let Some(f) = stack.pop() {
                first = f.first;
                middle = f.middle;
                last = f.last;
                check = f.check;
            } else {
                return;
            }
            continue;
        }

        if middle - first <= bufsize {
            if first < middle {
                ss_mergeforward(
                    t,
                    pa,
                    sa,
                    MergeRange {
                        first,
                        middle,
                        last,
                    },
                    buf,
                    depth,
                );
            }
            merge_check(t, pa, sa, first, last, check, depth);
            if let Some(f) = stack.pop() {
                first = f.first;
                middle = f.middle;
                last = f.last;
                check = f.check;
            } else {
                return;
            }
            continue;
        }

        let len_l = middle - first;
        let len_r = last - middle;
        let mut len = len_l.min(len_r) as i32;
        let mut half = len >> 1;
        let mut m = 0usize;

        while 0 < len {
            let li = {
                let vi = middle + m + half as usize;
                if sa[vi] >= 0 {
                    sa[vi] as usize
                } else {
                    !sa[vi] as usize
                }
            };
            let ri = {
                let vi2 = middle - m - half as usize - 1;
                if sa[vi2] >= 0 {
                    sa[vi2] as usize
                } else {
                    !sa[vi2] as usize
                }
            };
            if ss_compare(t, pa, li, pa, ri, depth) < 0 {
                m += half as usize + 1;
                half -= (len & 1) ^ 1;
            }
            len = half;
            half >>= 1;
        }

        if m > 0 {
            let lm = middle - m;
            let rm = middle + m;
            ss_blockswap(sa, lm, middle, m);
            let mut l = middle;
            let mut r = middle;
            let mut next = 0i32;
            if rm < last {
                if sa[rm] < 0 {
                    sa[rm] = !sa[rm];
                    if first < lm {
                        loop {
                            if l == 0 {
                                break;
                            }
                            l -= 1;
                            if sa[l] >= 0 {
                                break;
                            }
                        }
                        next |= 4;
                    }
                    next |= 1;
                } else if first < lm {
                    while sa[r] < 0 {
                        r += 1;
                    }
                    next |= 2;
                }
            }

            if l - first <= last - r {
                stack.push(SmergeFrame {
                    first: r,
                    middle: rm,
                    last,
                    check: (next & 3) | (check & 4),
                });
                middle = lm;
                last = l;
                check = (check & 3) | (next & 4);
            } else {
                if (next & 2) != 0 && r == middle {
                    next ^= 6;
                }
                stack.push(SmergeFrame {
                    first,
                    middle: lm,
                    last: l,
                    check: (check & 3) | (next & 4),
                });
                first = r;
                middle = rm;
                check = (next & 3) | (check & 4);
            }
        } else {
            if ss_compare(
                t,
                pa,
                sa[middle - 1] as usize,
                pa,
                sa[middle] as usize,
                depth,
            ) == 0
            {
                sa[middle] = !sa[middle];
            }
            merge_check(t, pa, sa, first, last, check, depth);
            if let Some(f) = stack.pop() {
                first = f.first;
                middle = f.middle;
                last = f.last;
                check = f.check;
            } else {
                return;
            }
        }
    }
}

#[inline(always)]
const fn getidx(a: i32) -> usize {
    if a >= 0 { a as usize } else { !a as usize }
}

/// Post-merge verification: marks equal suffixes at merge boundaries with negative flags.
fn merge_check(
    t: &[u8],
    pa: &[i32],
    sa: &mut [i32],
    first: usize,
    last: usize,
    check: i32,
    depth: Depth,
) {
    if (check & 1) != 0
        || ((check & 2) != 0
            && ss_compare(t, pa, getidx(sa[first - 1]), pa, sa[first] as usize, depth) == 0)
    {
        sa[first] = !sa[first];
    }
    if (check & 4) != 0
        && ss_compare(t, pa, getidx(sa[last - 1]), pa, sa[last] as usize, depth) == 0
    {
        sa[last] = !sa[last];
    }
}

/// Context for the B*-suffix sort, holding read-only references to text and PA array.
pub struct SsortCtx<'a> {
    pub t: &'a [u8],
    /// Full SA snapshot (sa[0..n+1]). PA = pa[pab..], i.e. pa[pab + i] = PAb[i].
    pub pa: &'a [i32],
    /// Offset of PAb within pa (= n - m, number of B*-suffixes from the end).
    pub pab: usize,
    pub depth: Depth,
    pub n: i32,
}

/// Sorts B*-suffixes in sa[first..last) using introspective sort and block merging.
///
/// This is the main entry point for the suffix-suffix sort phase.
/// Called from `sort_typebstar` for each non-trivial bucket.
pub fn sssort(
    ctx: &SsortCtx,
    sa: &mut [i32],
    first: usize,
    last: usize,
    buf: usize,
    bufsize: i32,
    lastsuffix: bool,
) {
    let (t, pa, pab, depth, n) = (ctx.t, ctx.pa, ctx.pab, ctx.depth, ctx.n);
    let pa_slice = &pa[pab..]; // shifted slice for ss_compare and merge functions
    let mut first = first;
    if lastsuffix {
        first += 1;
    }

    let (middle, limit, bufsize, buf) =
        if (bufsize as usize) < SS_BLOCKSIZE && (bufsize as usize) < last - first {
            let limit = ss_isqrt((last - first) as i32);
            let limit = limit.min(SS_BLOCKSIZE as i32);
            let middle = last - limit as usize;
            (middle, limit, limit, middle)
        } else {
            (last, 0i32, bufsize, buf)
        };

    let mut a = first;
    let mut i = 0usize;
    while SS_BLOCKSIZE < middle - a {
        ss_mintrosort(t, pa, pab, sa, a, a + SS_BLOCKSIZE, depth);
        let curbufsize = last - (a + SS_BLOCKSIZE);
        let (curbuf, curbufsize) = if curbufsize <= bufsize as usize {
            (buf, bufsize as usize)
        } else {
            (a + SS_BLOCKSIZE, curbufsize)
        };
        let mut b = a;
        let mut k = SS_BLOCKSIZE;
        let mut j = i;
        while j & 1 != 0 {
            ss_swapmerge(
                t,
                pa_slice,
                sa,
                MergeRange {
                    first: b - k,
                    middle: b,
                    last: b + k,
                },
                BufInfo {
                    buf: curbuf,
                    bufsize: curbufsize,
                },
                depth,
            );
            b -= k;
            k <<= 1;
            j >>= 1;
        }
        a += SS_BLOCKSIZE;
        i += 1;
    }
    ss_mintrosort(t, pa, pab, sa, a, middle, depth);

    let mut k = SS_BLOCKSIZE;
    let mut ii = i;
    while ii != 0 {
        if ii & 1 != 0 {
            ss_swapmerge(
                t,
                pa_slice,
                sa,
                MergeRange {
                    first: a - k,
                    middle: a,
                    last: middle,
                },
                BufInfo {
                    buf,
                    bufsize: bufsize as usize,
                },
                depth,
            );
            a -= k;
        }
        k <<= 1;
        ii >>= 1;
    }

    if limit != 0 {
        ss_mintrosort(t, pa, pab, sa, middle, last, depth);
        ss_inplacemerge(t, pa_slice, sa, first, middle, last, depth);
    }

    if lastsuffix {
        let pai0 = pa_val(pa, pab, sa[first - 1]);
        let pai1 = n - 2;
        // PAi = [pai0, pai1]: the lastsuffix B*-entry and its "next" boundary
        let pai = [pai0, pai1];
        let iv = sa[first - 1];
        let mut a2 = first;
        while a2 < last
            && (sa[a2] < 0 || ss_compare(t, &pai, 0, pa_slice, sa[a2] as usize, depth) > 0)
        {
            sa[a2 - 1] = sa[a2];
            a2 += 1;
        }
        sa[a2 - 1] = iv;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ss_ilg() {
        assert_eq!(ss_ilg(1), 0);
        assert_eq!(ss_ilg(2), 1);
        assert_eq!(ss_ilg(4), 2);
        assert_eq!(ss_ilg(255), 7);
        assert_eq!(ss_ilg(256), 8);
        assert_eq!(ss_ilg(1024), 10);
    }

    #[test]
    fn test_ss_isqrt() {
        assert_eq!(ss_isqrt(0), 0);
        assert_eq!(ss_isqrt(1), 1);
        assert_eq!(ss_isqrt(4), 2);
        assert_eq!(ss_isqrt(9), 3);
        assert_eq!(ss_isqrt(16), 4);
        assert_eq!(ss_isqrt(100), 10);
        // boundary where x = y*y
        for y in 1..=32i32 {
            let x = y * y;
            assert_eq!(ss_isqrt(x), y, "ss_isqrt({x}) should be {y}");
            assert_eq!(
                ss_isqrt(x - 1),
                y - 1,
                "ss_isqrt({}) should be {}",
                x - 1,
                y - 1
            );
        }
    }

    #[test]
    fn test_ss_compare_equal() {
        // T = "abab", PA = [0, 2] (next B*-suffix after 0 is at 2)
        // depth=0, p1=0 (PA[0]=0), p2=0 (PA[0]=0) → equal → 0
        let t = b"abab";
        let pa = [0i32, 2];
        assert_eq!(ss_compare(t, &pa, 0, &pa, 0, 0), 0);
    }

    #[test]
    fn test_ss_compare_less() {
        // T = "ba", PA = [1, 0] → depth=0
        // p1_idx=0: PA[0]=1, PA[1]=0 → U1=T[0+1]='a', U1n=T[0+2]=out of bound→2
        // p2_idx=1: PA[1]=0, PA[2] is out of range... extend PA
        let _t = b"bab\x00"; // sentinel (unused; t2 below is used)
        let pa = [1i32, 0, 3]; // PA[0]=1, PA[1]=0, PA[2]=3(sentinel)
        // p1: U1=T[depth+PA[0]]=T[0+1]='a', U1n=T[PA[1]+2]=T[2]='b'
        // p2: U2=T[depth+PA[1]]=T[0+0]='b', U2n=T[PA[2]+2]=T[5] (out of bounds in t)
        // 比較: 'a' vs 'b' → 'a' < 'b' → -1
        // Adjust: need t to be long enough
        let t2 = b"bab\x00\x00\x00";
        let r = ss_compare(t2, &pa, 0, &pa, 1, 0);
        assert!(r < 0, "expected < 0 but got {r}");
    }

    #[test]
    fn test_ss_heapsort() {
        let t = b"banana\x00";
        // PA = [5, 3, 1, 0, 4, 2] (suffix array of "banana")
        // SA values are indices into PA
        let pa = [5i32, 3, 1, 0, 4, 2, 0]; // extra 0 for sentinel
        let mut sa = [0i32, 1, 2, 3, 4, 5]; // indices to sort
        let td_offset = 0usize;
        let pab = 0usize; // pa is indexed directly (pab=0 means pa[0+i] = pa[i])
        ss_heapsort(&t[td_offset..], &pa, pab, &mut sa, 0, 6);
        // verify sorted by t[pa[pab + sa[i]]]
        for i in 1..6 {
            assert!(
                t[pa[sa[i - 1] as usize] as usize] <= t[pa[sa[i] as usize] as usize],
                "not sorted at {i}"
            );
        }
    }

    #[test]
    fn test_ss_insertionsort_impl() {
        // T = "dcba____" (8 bytes)
        // PA = [3, 2, 1, 0, 0]: PA[i] = string position of i-th B* suffix
        //   PA[0]=3→'a', PA[1]=2→'b', PA[2]=1→'c', PA[3]=0→'d', PA[4]=0(sentinel)
        // SA values (0..3) are indices into PA; we sort by ss_compare
        // Initial SA = [3,2,1,0] (reverse order by key 'd','c','b','a')
        // Expected: after sort, adjacent pairs satisfy ss_compare <= 0
        let t = b"dcba\x00\x00\x00\x00";
        let pa = [3i32, 2, 1, 0, 0];
        let mut sa = [3i32, 2, 1, 0];
        ss_insertionsort_impl(t, &pa, &mut sa, 0, 4, 0);
        for i in 1..4 {
            let a = if sa[i - 1] >= 0 {
                sa[i - 1] as usize
            } else {
                !sa[i - 1] as usize
            };
            let b = if sa[i] >= 0 {
                sa[i] as usize
            } else {
                !sa[i] as usize
            };
            assert!(
                ss_compare(t, &pa, a, &pa, b, 0) <= 0,
                "not sorted at {i}: sa[{}]={} sa[{}]={}",
                i - 1,
                sa[i - 1],
                i,
                sa[i]
            );
        }
    }
}
