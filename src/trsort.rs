//! Tandem repeat sort (trsort): sorts equal-group suffixes left by the B*-suffix sort.
//!
//! When multiple suffixes share the same initial key, trsort resolves their
//! relative order by comparing at increasing depths (doubling the ISA offset).
//! Uses introspective sort with budget-based recursion control to avoid O(n²) worst case.

use crate::constants::{FixedStack, LG_TABLE, TR_INSERTIONSORT_THRESHOLD, TR_STACKSIZE};
use crate::sa_index::SaIndex;

#[inline(always)]
/// Integer log2 using the lookup table (handles up to 64-bit sizes; the result is at most 63).
fn tr_ilg(n: usize) -> i32 {
    let n = n as u64;
    if n >> 32 != 0 {
        32 + tr_ilg32((n >> 32) as u32)
    } else {
        tr_ilg32(n as u32)
    }
}

#[inline(always)]
fn tr_ilg32(n: u32) -> i32 {
    if n & 0xffff_0000 != 0 {
        if n & 0xff00_0000 != 0 {
            24 + LG_TABLE[((n >> 24) & 0xff) as usize]
        } else {
            16 + LG_TABLE[((n >> 16) & 0xff) as usize]
        }
    } else if n & 0x0000_ff00 != 0 {
        8 + LG_TABLE[((n >> 8) & 0xff) as usize]
    } else {
        LG_TABLE[(n & 0xff) as usize]
    }
}

/// Insertion sort for small ranges, skipping negative group markers during shifts.
#[inline]
fn tr_insertionsort<I: SaIndex>(isad: &[I], sa: &mut [I], first: usize, last: usize) {
    // C: for(a = first + 1; a < last; ++a) {
    //      for(t = *a, b = a - 1; 0 > (r = ISAd[t] - ISAd[*b]);) {
    //        do { *(b + 1) = *b; } while((first <= --b) && (*b < 0));
    //        if(b < first) { break; }
    //      }
    //      if(r == 0) { *b = ~*b; }
    //      *(b + 1) = t;
    //    }
    // negative SA values are "same group" markers (bitwise NOT) - must jump over them while shifting
    let first_i = first as isize;
    let mut a = first_i + 1;
    while a < last as isize {
        let t = sa[a as usize];
        let mut b = a - 1; // b = a - 1 (always >= first since a > first)
        let mut r = I::ZERO;
        while b >= first_i {
            r = isad[t.to_usize()] - isad[sa[b as usize].to_usize()];
            if r >= I::ZERO {
                break;
            }
            // C: do { *(b+1) = *b; } while((first <= --b) && (*b < 0))
            sa[(b + 1) as usize] = sa[b as usize]; // shift non-negative element
            b -= 1;
            // skip (shift) contiguous negative elements
            while b >= first_i && sa[b as usize] < I::ZERO {
                sa[(b + 1) as usize] = sa[b as usize];
                b -= 1;
            }
            // after: b < first_i OR sa[b] >= I::ZERO
        }
        if r == I::ZERO && b >= 0 {
            sa[b as usize] = sa[b as usize].bit_not();
        }
        sa[(b + 1) as usize] = t;
        a += 1;
    }
}

// first is the absolute index of the heap root.
// child node absolute index: j = 2*(i - first) + 1 + first = 2*i - first + 1
fn tr_fixdown<I: SaIndex>(isad: &[I], sa: &mut [I], first: usize, mut i: usize, size: usize) {
    let v = sa[i];
    let c = isad[v.to_usize()];
    loop {
        let j = 2 * i - first + 1;
        if j >= size {
            break;
        }
        let mut k = j;
        let d_k = isad[sa[k].to_usize()];
        let j1 = j + 1;
        let d = if j1 < size {
            let e = isad[sa[j1].to_usize()];
            if d_k < e {
                k = j1;
                e
            } else {
                d_k
            }
        } else {
            d_k
        };
        if d <= c {
            break;
        }
        sa[i] = sa[k];
        i = k;
    }
    sa[i] = v;
}

/// Heapsort fallback for `tr_introsort` when the recursion limit is exhausted.
fn tr_heapsort<I: SaIndex>(isad: &[I], sa: &mut [I], first: usize, size: usize) {
    let mut m = size;
    if size.is_multiple_of(2) {
        m -= 1;
        if isad[sa[first + m / 2].to_usize()] < isad[sa[first + m].to_usize()] {
            sa.swap(first + m, first + m / 2);
        }
    }
    if m >= 2 {
        let mut i = m / 2;
        loop {
            tr_fixdown(isad, sa, first, first + i, first + m);
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
    if size.is_multiple_of(2) {
        sa.swap(first, first + m);
        tr_fixdown(isad, sa, first, first, first + m);
    }
    let mut i = m;
    while i > 0 {
        i -= 1;
        let t = sa[first];
        sa[first] = sa[first + i];
        tr_fixdown(isad, sa, first, first, first + i);
        sa[first + i] = t;
    }
}

#[inline(always)]
fn tr_median3_idx<I: SaIndex>(
    isad: &[I],
    sa: &[I],
    mut v1: usize,
    mut v2: usize,
    v3: usize,
) -> usize {
    if isad[sa[v1].to_usize()] > isad[sa[v2].to_usize()] {
        core::mem::swap(&mut v1, &mut v2);
    }
    if isad[sa[v2].to_usize()] > isad[sa[v3].to_usize()] {
        if isad[sa[v1].to_usize()] > isad[sa[v3].to_usize()] {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

#[inline(always)]
fn tr_median5_idx<I: SaIndex>(
    isad: &[I],
    sa: &[I],
    mut v1: usize,
    mut v2: usize,
    mut v3: usize,
    mut v4: usize,
    mut v5: usize,
) -> usize {
    if isad[sa[v2].to_usize()] > isad[sa[v3].to_usize()] {
        core::mem::swap(&mut v2, &mut v3);
    }
    if isad[sa[v4].to_usize()] > isad[sa[v5].to_usize()] {
        core::mem::swap(&mut v4, &mut v5);
    }
    if isad[sa[v2].to_usize()] > isad[sa[v4].to_usize()] {
        core::mem::swap(&mut v2, &mut v4);
        core::mem::swap(&mut v3, &mut v5);
    }
    if isad[sa[v1].to_usize()] > isad[sa[v3].to_usize()] {
        core::mem::swap(&mut v1, &mut v3);
    }
    if isad[sa[v1].to_usize()] > isad[sa[v4].to_usize()] {
        core::mem::swap(&mut v1, &mut v4);
        core::mem::swap(&mut v3, &mut v5);
    }
    if isad[sa[v3].to_usize()] > isad[sa[v4].to_usize()] {
        v4
    } else {
        v3
    }
}

/// Selects a pivot index using median-of-3 or median-of-5 depending on range size.
fn tr_pivot_idx<I: SaIndex>(isad: &[I], sa: &[I], first: usize, last: usize) -> usize {
    let t = (last - first) as i32;
    let middle = first + (t / 2) as usize;
    if t <= 512 {
        if t <= 32 {
            tr_median3_idx(isad, sa, first, middle, last - 1)
        } else {
            let t2 = (t >> 2) as usize;
            tr_median5_idx(isad, sa, first, first + t2, middle, last - 1 - t2, last - 1)
        }
    } else {
        let t2 = (t >> 3) as usize;
        let f = tr_median3_idx(isad, sa, first, first + t2, first + 2 * t2);
        let m = tr_median3_idx(isad, sa, middle - t2, middle, middle + t2);
        let l = tr_median3_idx(isad, sa, last - 1 - 2 * t2, last - 1 - t2, last - 1);
        tr_median3_idx(isad, sa, f, m, l)
    }
}

/// Budget-based recursion control for `tr_introsort`.
/// Limits the total work to prevent O(n²) worst case on degenerate inputs.
struct TrBudget<I: SaIndex> {
    chance: i32,
    remain: I,
    incval: I,
    pub count: I,
}

impl<I: SaIndex> TrBudget<I> {
    const fn new(chance: i32, incval: I) -> Self {
        Self {
            chance,
            remain: incval,
            incval,
            count: I::ZERO,
        }
    }

    #[inline(always)]
    fn check(&mut self, size: I) -> bool {
        if size <= self.remain {
            self.remain -= size;
            return true;
        }
        if self.chance == 0 {
            self.count += size;
            return false;
        }
        self.remain += self.incval - size;
        self.chance -= 1;
        true
    }
}

/// Partitions [first..last) into three parts using v as pivot; returns the equal-range as [*pa..*pb).
fn tr_partition<I: SaIndex>(
    isad: &[I],
    sa: &mut [I],
    first: usize,
    middle: usize,
    last: usize,
    v: I,
) -> (usize, usize) {
    let mut b = middle;
    let mut x = I::ZERO;
    while b < last {
        x = isad[sa[b].to_usize()];
        if x != v {
            break;
        }
        b += 1;
    }
    let mut a = b;
    if a < last && x < v {
        b += 1;
        while b < last {
            x = isad[sa[b].to_usize()];
            if x > v {
                break;
            }
            if x == v {
                sa.swap(b, a);
                a += 1;
            }
            b += 1;
        }
    }
    // Right scan: for(c = last; (b < --c) && ((x = ISAd[*c]) == v);)
    let mut c = last;
    loop {
        if c == 0 {
            break;
        }
        c -= 1;
        if c <= b {
            break;
        } // b < --c failed
        x = isad[sa[c].to_usize()];
        if x != v {
            break;
        }
    }
    // d = c  (C: if((b < (d = c)) && (x > v)))
    let mut d = c;
    if b < c && x > v {
        loop {
            if c == 0 {
                break;
            }
            c -= 1;
            if c <= b {
                break;
            }
            x = isad[sa[c].to_usize()];
            if x < v {
                break;
            }
            if x == v {
                sa.swap(c, d);
                d -= 1;
            } // C: SWAP(*c,*d); --d
        }
    }
    // Main loop: for(; b < c;)
    while b < c {
        sa.swap(b, c);
        // Inner left: for(; (++b < c) && ((x = ISAd[*b]) <= v);)
        loop {
            b += 1;
            if b >= c {
                break;
            }
            x = isad[sa[b].to_usize()];
            if x > v {
                break;
            }
            if x == v {
                sa.swap(b, a);
                a += 1;
            }
        }
        // Inner right: for(; (b < --c) && ((x = ISAd[*c]) >= v);)
        loop {
            if c == 0 {
                break;
            }
            c -= 1;
            if c <= b {
                break;
            }
            x = isad[sa[c].to_usize()];
            if x < v {
                break;
            }
            if x == v {
                sa.swap(c, d);
                d -= 1;
            } // C: SWAP(*c,*d); --d
        }
    }

    if a <= d {
        // C: c = b - 1 (reset for blockswap)
        let c_new = b - 1;
        let s = (a - first).min(b - a);
        for k in 0..s {
            sa.swap(first + k, b - s + k);
        }
        // C uses signed: if((s = d - c) > (t = last - d - 1)) { s = t; }
        let dc = d as i64 - c_new as i64;
        let ldd = last as i64 - d as i64 - 1;
        let s2 = if dc > 0 && ldd > 0 {
            dc.min(ldd) as usize
        } else {
            0
        };
        for k in 0..s2 {
            sa.swap(b + k, last - s2 + k);
        }
        // C: first += (b - a), last -= (d - c) where c = b-1
        let new_first = first + (b - a);
        let new_last = (last as i64 - dc) as usize;
        return (new_first, new_last);
    }
    (first, last)
}

/// Copies sorted suffix positions from sa[a..b) back into the full range sa[first..last),
/// updating ISA group labels.
fn tr_copy<I: SaIndex>(
    isa: &mut [I],
    sa: &mut [I],
    first: usize,
    a: usize,
    b: usize,
    last: usize,
    depth: I,
) {
    let v = I::from_usize(b) - I::ONE; // b - SA - 1 where SA=0
    // C: for(c = first, d = a - 1; c <= d; ++c) { if(...) { *++d = s; ISA[s] = d - SA; } }
    // Rust: d starts at a (= C's d+1), write at d then d++; loop while c < d (= C's c <= d)
    let mut d = a;
    let mut c = first;
    while c < d {
        let s = sa[c] - depth;
        if s >= I::ZERO && isa[s.to_usize()] == v {
            sa[d] = s;
            isa[s.to_usize()] = I::from_usize(d);
            d += 1;
        }
        c += 1;
    }
    // C: for(c = last - 1, e = d + 1, d = b; e < d; --c) { if(...) { *--d = s; ISA[s] = d - SA; } }
    // C starts at d = b and pre-decrements, so the first write lands at b - 1.
    // Rust e = C's (d+1) from the first loop; the loop continues while e < d2.
    let e = d;
    let mut d2 = b;
    if last > 0 {
        let mut c2 = last - 1;
        loop {
            if e >= d2 {
                break;
            } // C condition: e < d
            let s = sa[c2] - depth;
            if s >= I::ZERO && isa[s.to_usize()] == v {
                d2 -= 1;
                sa[d2] = s;
                isa[s.to_usize()] = I::from_usize(d2);
            }
            if c2 == 0 {
                break;
            }
            c2 -= 1;
        }
    }
}

/// Partial copy variant of `tr_copy`, used when budget is exhausted (trlink == -1).
/// Creates new equal-group runs where exact ordering could not be determined.
fn tr_partialcopy<I: SaIndex>(
    isa: &mut [I],
    sa: &mut [I],
    first: usize,
    a: usize,
    b: usize,
    last: usize,
    depth: I,
) {
    let v = I::from_usize(b) - I::ONE;
    // C: for(c = first, d = a - 1; c <= d; ++c) — d extends dynamically
    let mut d = a;
    let mut lastrank = I::ZERO - I::ONE;
    let mut newrank = I::ZERO - I::ONE;
    let mut c = first;
    while c < d {
        let s = sa[c] - depth;
        if s >= I::ZERO && isa[s.to_usize()] == v {
            sa[d] = s;
            let rank = isa[(s + depth).to_usize()];
            if lastrank != rank {
                lastrank = rank;
                newrank = I::from_usize(d);
            }
            isa[s.to_usize()] = newrank;
            d += 1;
        }
        c += 1;
    }

    // C: for(e = d; first <= e; --e) — processes [first, d] inclusive (C's d = Rust's d-1)
    lastrank = I::ZERO - I::ONE;
    let mut e = d;
    while e > first {
        e -= 1;
        let rank = isa[sa[e].to_usize()];
        if lastrank != rank {
            lastrank = rank;
            newrank = I::from_usize(e);
        }
        if newrank != rank {
            isa[sa[e].to_usize()] = newrank;
        }
    }

    // C: for(c = last - 1, e = d + 1, d = b; e < d; --c)
    // C starts at d = b and pre-decrements, so the first write lands at b - 1.
    // Rust e2 = C's (d+1) from the first loop.
    lastrank = I::ZERO - I::ONE;
    let e2 = d;
    let mut d2 = b;
    if last > 0 {
        let mut c2 = last - 1;
        loop {
            if e2 >= d2 {
                break;
            } // C condition: e < d
            let s = sa[c2] - depth;
            if s >= I::ZERO && isa[s.to_usize()] == v {
                d2 -= 1;
                sa[d2] = s;
                let rank = isa[(s + depth).to_usize()];
                if lastrank != rank {
                    lastrank = rank;
                    newrank = I::from_usize(d2);
                }
                isa[s.to_usize()] = newrank;
            }
            if c2 == 0 {
                break;
            }
            c2 -= 1;
        }
    }
}

#[derive(Clone, Copy, Default)]
struct TrFrame {
    isad: usize,
    first: usize,
    last: usize,
    limit: i32,
    trlink: i32,
}

struct TrSortState<'a> {
    isad: &'a mut usize,
    first: &'a mut usize,
    last: &'a mut usize,
    limit: &'a mut i32,
    trlink: &'a mut i32,
}

/// Handles the tandem repeat partition case (limit == -1) in tr_introsort.
/// Returns `false` if tr_introsort should return (stack exhausted).
#[inline(always)]
fn tr_handle_tandem_partition<I: SaIndex>(
    isa: &mut [I],
    sa: &mut [I],
    stack: &mut FixedStack<TrFrame, { TR_STACKSIZE }>,
    state: &mut TrSortState<'_>,
    incr: usize,
) -> bool {
    // SAFETY: isa and sa are non-overlapping (split_at_mut(m) in sort_typebstar), and the
    // `&mut isa` is not used while `isad_tandem` is live, so the shared reborrow is not
    // aliased by a writer. tr_partition only reads from isad_tandem and swaps elements in sa.
    let isad_tandem: &[I] = unsafe {
        let offset = *state.isad - incr;
        core::slice::from_raw_parts(isa.as_ptr().add(offset), isa.len() - offset)
    };
    let (a, b) = tr_partition(
        isad_tandem,
        sa,
        *state.first,
        *state.first,
        *state.last,
        I::from_usize(*state.last) - I::ONE,
    );

    if a < *state.last {
        let v = I::from_usize(a) - I::ONE;
        for k in *state.first..a {
            isa[sa[k].to_usize()] = v;
        }
    }
    if b < *state.last {
        let v = I::from_usize(b) - I::ONE;
        for k in a..b {
            isa[sa[k].to_usize()] = v;
        }
    }

    if b - a > 1 {
        stack.push(TrFrame {
            isad: 0,
            first: a,
            last: b,
            limit: 0,
            trlink: 0,
        });
        stack.push(TrFrame {
            isad: *state.isad - incr,
            first: *state.first,
            last: *state.last,
            limit: -2,
            trlink: *state.trlink,
        });
        *state.trlink = (stack.len() as i32) - 2;
    }
    if a - *state.first <= *state.last - b {
        if a - *state.first > 1 {
            stack.push(TrFrame {
                isad: *state.isad,
                first: b,
                last: *state.last,
                limit: tr_ilg(*state.last - b),
                trlink: *state.trlink,
            });
            *state.last = a;
            *state.limit = tr_ilg(a - *state.first);
        } else if *state.last - b > 1 {
            *state.first = b;
            *state.limit = tr_ilg(*state.last - b);
        } else {
            return tr_pop_stack(stack, state);
        }
    } else if *state.last - b > 1 {
        stack.push(TrFrame {
            isad: *state.isad,
            first: *state.first,
            last: a,
            limit: tr_ilg(a - *state.first),
            trlink: *state.trlink,
        });
        *state.first = b;
        *state.limit = tr_ilg(*state.last - b);
    } else if a - *state.first > 1 {
        *state.last = a;
        *state.limit = tr_ilg(a - *state.first);
    } else {
        return tr_pop_stack(stack, state);
    }
    true
}

/// Handles the negate/scan/budget case (limit < -2) in tr_introsort.
/// Returns `false` if tr_introsort should return (stack exhausted).
#[inline(always)]
fn tr_handle_negate_scan<I: SaIndex>(
    isa: &mut [I],
    sa: &mut [I],
    stack: &mut FixedStack<TrFrame, { TR_STACKSIZE }>,
    state: &mut TrSortState<'_>,
    incr: usize,
    budget: &mut TrBudget<I>,
) -> bool {
    if sa[*state.first] >= I::ZERO {
        let mut a = *state.first;
        while a < *state.last && sa[a] >= I::ZERO {
            isa[sa[a].to_usize()] = I::from_usize(a);
            a += 1;
        }
        *state.first = a;
    }
    if *state.first >= *state.last {
        return tr_pop_stack(stack, state);
    }

    let first = *state.first;
    let last = *state.last;
    let isad = *state.isad;

    let mut a = first;
    loop {
        sa[a] = sa[a].bit_not();
        a += 1;
        if a >= last || sa[a] >= I::ZERO {
            break;
        }
    }
    let next = if a < last && isa[sa[a].to_usize()] != isa[isad + sa[a].to_usize()] {
        tr_ilg(a - first + 1)
    } else {
        -1
    };
    let run_end = if a < last {
        a += 1;
        if a < last {
            let v = I::from_usize(a) - I::ONE;
            for k in first..a {
                isa[sa[k].to_usize()] = v;
            }
        }
        a
    } else {
        last
    };

    if budget.check(I::from_usize(run_end - first)) {
        if run_end - first <= last - run_end {
            stack.push(TrFrame {
                isad,
                first: run_end,
                last,
                limit: -3,
                trlink: *state.trlink,
            });
            *state.isad += incr;
            *state.last = run_end;
            *state.limit = next;
        } else if last - run_end > 1 {
            stack.push(TrFrame {
                isad: isad + incr,
                first,
                last: run_end,
                limit: next,
                trlink: *state.trlink,
            });
            *state.first = run_end;
            *state.limit = -3;
        } else {
            *state.isad += incr;
            *state.last = run_end;
            *state.limit = next;
        }
    } else {
        if *state.trlink >= 0 {
            stack[*state.trlink as usize].limit = -1;
        }
        if last - run_end > 1 {
            *state.first = run_end;
            *state.limit = -3;
        } else {
            return tr_pop_stack(stack, state);
        }
    }
    true
}

/// Pops a frame from the stack into the state. Returns `false` if stack is empty.
#[inline(always)]
const fn tr_pop_stack(
    stack: &mut FixedStack<TrFrame, { TR_STACKSIZE }>,
    state: &mut TrSortState<'_>,
) -> bool {
    if let Some(f) = stack.pop() {
        *state.isad = f.isad;
        *state.first = f.first;
        *state.last = f.last;
        *state.limit = f.limit;
        *state.trlink = f.trlink;
        true
    } else {
        false
    }
}

/// Main introspective sort engine for tandem repeat sorting.
///
/// Uses quicksort with partition, falling back to heapsort when the recursion
/// limit is exhausted. Budget control prevents O(n²) worst case.
/// The `limit` state variable encodes the current phase:
///   - `limit >= 0`: normal introsort with remaining recursion budget
///   - `limit == -1`: tandem repeat partition phase
///   - `limit == -2`: copy/partial-copy phase
///   - `limit < -2`: negate/scan/budget-check phase
fn tr_introsort<I: SaIndex>(
    isa: &mut [I],
    isad_init: usize,
    sa: &mut [I],
    sa_first: usize,
    sa_last: usize,
    budget: &mut TrBudget<I>,
) {
    let incr = isad_init; // ISAd - ISA = isad_init (ISA is 0-based)
    let mut stack = FixedStack::<TrFrame, { TR_STACKSIZE }>::new();

    let mut isad = isad_init;
    let mut first = sa_first;
    let mut last = sa_last;
    let mut limit = tr_ilg(last - first);
    let mut trlink: i32 = -1;

    loop {
        if limit < 0 {
            if limit == -1 {
                let mut state = TrSortState {
                    isad: &mut isad,
                    first: &mut first,
                    last: &mut last,
                    limit: &mut limit,
                    trlink: &mut trlink,
                };
                if !tr_handle_tandem_partition(isa, sa, &mut stack, &mut state, incr) {
                    return;
                }
            } else if limit == -2 {
                let popped = stack.pop().unwrap();
                let a = popped.first;
                let b = popped.last;
                let d_flag = popped.limit;
                if d_flag == 0 {
                    tr_copy(isa, sa, first, a, b, last, I::from_usize(isad));
                } else {
                    if trlink >= 0 {
                        stack[trlink as usize].limit = -1;
                    }
                    tr_partialcopy(isa, sa, first, a, b, last, I::from_usize(isad));
                }
                if let Some(f) = stack.pop() {
                    isad = f.isad;
                    first = f.first;
                    last = f.last;
                    limit = f.limit;
                    trlink = f.trlink;
                } else {
                    return;
                }
            } else {
                let mut state = TrSortState {
                    isad: &mut isad,
                    first: &mut first,
                    last: &mut last,
                    limit: &mut limit,
                    trlink: &mut trlink,
                };
                if !tr_handle_negate_scan(isa, sa, &mut stack, &mut state, incr, budget) {
                    return;
                }
            }
            continue;
        }

        if last - first <= TR_INSERTIONSORT_THRESHOLD {
            tr_insertionsort(&isa[isad..], sa, first, last);
            limit = -3;
            continue;
        }

        limit -= 1;
        if limit == -1 {
            // limit was 0, now -1 after decrement → use heapsort
            // restore: C does `if(limit-- == 0)` so heapsort when original limit==0
            tr_heapsort(&isa[isad..], sa, first, last - first);
            // after heapsort, mark duplicates
            let mut a = last - 1;
            while a > first {
                let x = isa[isad + sa[a].to_usize()];
                let mut b = a;
                loop {
                    if b <= first || sa[b - 1] < I::ZERO || isa[isad + sa[b - 1].to_usize()] != x {
                        break;
                    }
                    b -= 1;
                    sa[b] = sa[b].bit_not();
                }
                if b == first {
                    break;
                }
                a = b - 1;
            }
            limit = -3;
            continue;
        }

        let pivot_idx = tr_pivot_idx(&isa[isad..], sa, first, last);
        sa.swap(first, pivot_idx);
        let v = isa[isad + sa[first].to_usize()];

        // SAFETY: Same aliasing argument as the isad_tandem case above: tr_partition reads from
        // isad_slice (= isa[isad..]) and swaps within sa; isa and sa are disjoint.
        // After tr_partition returns the slice is no longer live, so the subsequent reads
        // and writes to isa are safe with no aliased reference outstanding.
        let isad_slice: &[I] =
            unsafe { core::slice::from_raw_parts(isa.as_ptr().add(isad), isa.len() - isad) };
        let (a, b) = tr_partition_owned(isad_slice, sa, first, first + 1, last, v);

        if last - first != b - a {
            let next = if isa[sa[a].to_usize()] != v {
                tr_ilg(b - a)
            } else {
                -1
            };

            let v2 = I::from_usize(a) - I::ONE;
            for k in first..a {
                isa[sa[k].to_usize()] = v2;
            }
            if b < last {
                let v3 = I::from_usize(b) - I::ONE;
                for k in a..b {
                    isa[sa[k].to_usize()] = v3;
                }
            }

            if b - a > 1 && budget.check(I::from_usize(b - a)) {
                let mut state = TrSortState {
                    isad: &mut isad,
                    first: &mut first,
                    last: &mut last,
                    limit: &mut limit,
                    trlink: &mut trlink,
                };
                push5_and_continue(&mut stack, &mut state, incr, a, b, next);
            } else {
                if b - a > 1 && trlink >= 0 {
                    stack[trlink as usize].limit = -1;
                }
                if a - first <= last - b {
                    if a - first > 1 {
                        stack.push(TrFrame {
                            isad,
                            first: b,
                            last,
                            limit,
                            trlink,
                        });
                        last = a;
                    } else if last - b > 1 {
                        first = b;
                    } else if let Some(f) = stack.pop() {
                        isad = f.isad;
                        first = f.first;
                        last = f.last;
                        limit = f.limit;
                        trlink = f.trlink;
                    } else {
                        return;
                    }
                } else if last - b > 1 {
                    stack.push(TrFrame {
                        isad,
                        first,
                        last: a,
                        limit,
                        trlink,
                    });
                    first = b;
                } else if a - first > 1 {
                    last = a;
                } else if let Some(f) = stack.pop() {
                    isad = f.isad;
                    first = f.first;
                    last = f.last;
                    limit = f.limit;
                    trlink = f.trlink;
                } else {
                    return;
                }
            }
        } else if budget.check(I::from_usize(last - first)) {
            limit = tr_ilg(last - first);
            isad += incr;
        } else {
            if trlink >= 0 {
                stack[trlink as usize].limit = -1;
            }
            if let Some(f) = stack.pop() {
                isad = f.isad;
                first = f.first;
                last = f.last;
                limit = f.limit;
                trlink = f.trlink;
            } else {
                return;
            }
        }
    }
}

fn tr_partition_owned<I: SaIndex>(
    isad: &[I],
    sa: &mut [I],
    first: usize,
    middle: usize,
    last: usize,
    v: I,
) -> (usize, usize) {
    tr_partition(isad, sa, first, middle, last, v)
}

const fn push5_and_continue(
    stack: &mut FixedStack<TrFrame, { TR_STACKSIZE }>,
    state: &mut TrSortState<'_>,
    incr: usize,
    a: usize,
    b: usize,
    next: i32,
) {
    let af = *state.first;
    let al = *state.last;
    let lim = *state.limit;
    let tl = *state.trlink;

    if a - af <= al - b {
        if al - b <= b - a {
            if a - af > 1 {
                stack.push(TrFrame {
                    isad: *state.isad + incr,
                    first: a,
                    last: b,
                    limit: next,
                    trlink: tl,
                });
                stack.push(TrFrame {
                    isad: *state.isad,
                    first: b,
                    last: al,
                    limit: lim,
                    trlink: tl,
                });
                *state.last = a;
            } else if al - b > 1 {
                stack.push(TrFrame {
                    isad: *state.isad + incr,
                    first: a,
                    last: b,
                    limit: next,
                    trlink: tl,
                });
                *state.first = b;
            } else {
                *state.isad += incr;
                *state.first = a;
                *state.last = b;
                *state.limit = next;
            }
        } else if a - af <= b - a {
            stack.push(TrFrame {
                isad: *state.isad,
                first: b,
                last: al,
                limit: lim,
                trlink: tl,
            });
            if a - af > 1 {
                stack.push(TrFrame {
                    isad: *state.isad + incr,
                    first: a,
                    last: b,
                    limit: next,
                    trlink: tl,
                });
                *state.last = a;
            } else {
                *state.isad += incr;
                *state.first = a;
                *state.last = b;
                *state.limit = next;
            }
        } else {
            stack.push(TrFrame {
                isad: *state.isad,
                first: b,
                last: al,
                limit: lim,
                trlink: tl,
            });
            stack.push(TrFrame {
                isad: *state.isad,
                first: af,
                last: a,
                limit: lim,
                trlink: tl,
            });
            *state.isad += incr;
            *state.first = a;
            *state.last = b;
            *state.limit = next;
        }
    } else if a - af <= b - a {
        if al - b > 1 {
            stack.push(TrFrame {
                isad: *state.isad + incr,
                first: a,
                last: b,
                limit: next,
                trlink: tl,
            });
            stack.push(TrFrame {
                isad: *state.isad,
                first: af,
                last: a,
                limit: lim,
                trlink: tl,
            });
            *state.first = b;
        } else if a - af > 1 {
            stack.push(TrFrame {
                isad: *state.isad + incr,
                first: a,
                last: b,
                limit: next,
                trlink: tl,
            });
            *state.last = a;
        } else {
            *state.isad += incr;
            *state.first = a;
            *state.last = b;
            *state.limit = next;
        }
    } else if al - b <= b - a {
        stack.push(TrFrame {
            isad: *state.isad,
            first: af,
            last: a,
            limit: lim,
            trlink: tl,
        });
        if al - b > 1 {
            stack.push(TrFrame {
                isad: *state.isad + incr,
                first: a,
                last: b,
                limit: next,
                trlink: tl,
            });
            *state.first = b;
        } else {
            *state.isad += incr;
            *state.first = a;
            *state.last = b;
            *state.limit = next;
        }
    } else {
        stack.push(TrFrame {
            isad: *state.isad,
            first: af,
            last: a,
            limit: lim,
            trlink: tl,
        });
        stack.push(TrFrame {
            isad: *state.isad,
            first: b,
            last: al,
            limit: lim,
            trlink: tl,
        });
        *state.isad += incr;
        *state.first = a;
        *state.last = b;
        *state.limit = next;
    }
}

/// Sorts equal-group suffixes left by the B*-suffix sort using tandem repeat sorting.
///
/// `isa` and `sa` are the two halves of the working buffer (split at position m).
/// Called from `sort_typebstar` after sssort completes.
pub fn trsort<I: SaIndex>(isa: &mut [I], sa: &mut [I], n: usize, depth: usize) {
    let mut budget = TrBudget::new(tr_ilg(n) * 2 / 3, I::from_usize(n));
    let neg_n = I::ZERO - I::from_usize(n);

    let mut isad = depth;
    while sa[0] > neg_n {
        let mut first = 0usize;
        let mut skip = I::ZERO;
        let mut unsorted = I::ZERO;

        loop {
            let t = sa[first];
            if t < I::ZERO {
                first = (first as isize - t.to_isize()) as usize;
                skip += t;
            } else {
                if skip != I::ZERO {
                    // C: *(first + skip) = skip where skip is negative (signed pointer arithmetic)
                    sa[(first as isize + skip.to_isize()) as usize] = skip;
                    skip = I::ZERO;
                }
                let last = (isa[t.to_usize()] + I::ONE).to_usize();
                if last - first > 1 {
                    budget.count = I::ZERO;
                    tr_introsort(isa, isad, sa, first, last, &mut budget);
                    if budget.count != I::ZERO {
                        unsorted += budget.count;
                    } else {
                        skip = I::from_isize(first as isize - last as isize);
                    }
                } else if last - first == 1 {
                    skip = I::ZERO - I::ONE;
                }
                first = last;
            }
            if first >= n {
                break;
            }
        }
        if skip != I::ZERO {
            sa[(first as isize + skip.to_isize()) as usize] = skip;
        }
        if unsorted == I::ZERO {
            break;
        }
        isad += isad; // ISAd += ISAd - ISA → new_isad = isad + isad = 2*isad (since ISA=0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tr_ilg() {
        assert_eq!(tr_ilg(1), 0);
        assert_eq!(tr_ilg(2), 1);
        assert_eq!(tr_ilg(3), 1);
        assert_eq!(tr_ilg(4), 2);
        assert_eq!(tr_ilg(256), 8);
        assert_eq!(tr_ilg(255), 7);
    }

    #[test]
    fn test_trbudget_check() {
        let mut b = TrBudget::new(2, 10);
        assert!(b.check(5));
        assert_eq!(b.remain, 5);
        assert!(b.check(5));
        assert_eq!(b.remain, 0);
        // remain=0, chance=2: should use a chance
        assert!(b.check(3));
        assert_eq!(b.chance, 1);
        assert_eq!(b.remain, 7);
        // exhaust chances
        assert!(b.check(20));
        assert_eq!(b.chance, 0);
        assert!(!b.check(1));
        assert_eq!(b.count, 1);
    }

    #[test]
    fn test_tr_insertionsort_simple() {
        // ISAd = [3, 1, 4, 1, 5, 9, 2, 6]
        // SA = [0, 1, 2, 3, 4, 5, 6, 7] initially
        // after sort by ISAd values: order by ISAd[sa[i]]
        let isad = [3i32, 1, 4, 1, 5, 9, 2, 6];
        let mut sa = [1i32, 3, 0, 6, 2, 4, 7, 5];
        tr_insertionsort(&isad, &mut sa, 0, 8);
        // verify sorted by isad values (ignoring negative markers)
        for i in 1..sa.len() {
            let a = sa[i - 1];
            let b = sa[i];
            // negative values are duplicates of preceding
            if a >= 0 && b >= 0 {
                assert!(isad[a as usize] <= isad[b as usize]);
            }
        }
    }

    #[test]
    fn test_tr_heapsort_simple() {
        let isad = [3i32, 1, 4, 1, 5, 9, 2, 6];
        let mut sa = [1i32, 3, 0, 6, 2, 4, 7, 5];
        tr_heapsort(&isad, &mut sa, 0, 8);
        for i in 1..sa.len() {
            assert!(isad[sa[i - 1].to_usize()] <= isad[sa[i].to_usize()]);
        }
    }

    // Expected values below follow the C original (trsort.c): the right partition is
    // filled downwards starting at b - 1.
    #[test]
    fn test_tr_copy_fills_slot_before_b() {
        // depth = 1, middle group = sa[2..4), rank label v = b - 1 = 3.
        // Suffix 0 is found on the left side, suffix 1 on the right side.
        let mut isa = [3i32, 3, 5, 5, 5, 5];
        let mut sa = [1i32, 0, 9, 9, 0, 2];
        tr_copy(&mut isa, &mut sa, 0, 2, 4, 6, 1);
        assert_eq!(sa, [1, 0, 0, 1, 0, 2]);
        assert_eq!(&isa[..2], &[2, 3]);
    }

    #[test]
    fn test_tr_partialcopy_fills_slot_before_b() {
        // Same layout as above but with everything on the right side (first == a).
        let mut isa = [3i32, 3, 7, 5, 5, 5];
        let mut sa = [9i32, 9, 9, 9, 1, 2];
        tr_partialcopy(&mut isa, &mut sa, 2, 2, 4, 6, 1);
        assert_eq!(&sa[2..4], &[0, 1]);
        assert_eq!(&isa[..2], &[2, 3]);
    }
}
