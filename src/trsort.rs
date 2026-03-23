use crate::constants::{FixedStack, LG_TABLE, TR_INSERTIONSORT_THRESHOLD, TR_STACKSIZE};

#[inline(always)]
fn tr_ilg(n: i32) -> i32 {
    if n & 0xffff0000u32 as i32 != 0 {
        if n & 0xff000000u32 as i32 != 0 {
            24 + LG_TABLE[((n >> 24) & 0xff) as usize]
        } else {
            16 + LG_TABLE[((n >> 16) & 0xff) as usize]
        }
    } else if n & 0x0000ff00 != 0 {
        8 + LG_TABLE[((n >> 8) & 0xff) as usize]
    } else {
        LG_TABLE[(n & 0xff) as usize]
    }
}

fn tr_insertionsort(isad: &[i32], sa: &mut [i32], first: usize, last: usize) {
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
        let mut r = 0i32;
        while b >= first_i {
            r = isad[t as usize] - isad[sa[b as usize] as usize];
            if r >= 0 {
                break;
            }
            // C: do { *(b+1) = *b; } while((first <= --b) && (*b < 0))
            sa[(b + 1) as usize] = sa[b as usize]; // shift non-negative element
            b -= 1;
            // skip (shift) contiguous negative elements
            while b >= first_i && sa[b as usize] < 0 {
                sa[(b + 1) as usize] = sa[b as usize];
                b -= 1;
            }
            // after: b < first_i OR sa[b] >= 0
        }
        if r == 0 && b >= 0 {
            sa[b as usize] = !sa[b as usize];
        }
        sa[(b + 1) as usize] = t;
        a += 1;
    }
}

// first is the absolute index of the heap root.
// child node absolute index: j = 2*(i - first) + 1 + first = 2*i - first + 1
fn tr_fixdown(isad: &[i32], sa: &mut [i32], first: usize, mut i: usize, size: usize) {
    let v = sa[i];
    let c = isad[v as usize];
    loop {
        let j = 2 * i - first + 1;
        if j >= size {
            break;
        }
        let mut k = j;
        let d_k = isad[sa[k] as usize];
        let j1 = j + 1;
        let d = if j1 < size {
            let e = isad[sa[j1] as usize];
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

fn tr_heapsort(isad: &[i32], sa: &mut [i32], first: usize, size: usize) {
    let mut m = size;
    if size.is_multiple_of(2) {
        m -= 1;
        if isad[sa[first + m / 2] as usize] < isad[sa[first + m] as usize] {
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
fn tr_median3_idx(isad: &[i32], sa: &[i32], mut v1: usize, mut v2: usize, v3: usize) -> usize {
    if isad[sa[v1] as usize] > isad[sa[v2] as usize] {
        std::mem::swap(&mut v1, &mut v2);
    }
    if isad[sa[v2] as usize] > isad[sa[v3] as usize] {
        if isad[sa[v1] as usize] > isad[sa[v3] as usize] {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

#[inline(always)]
fn tr_median5_idx(
    isad: &[i32],
    sa: &[i32],
    mut v1: usize,
    mut v2: usize,
    mut v3: usize,
    mut v4: usize,
    mut v5: usize,
) -> usize {
    if isad[sa[v2] as usize] > isad[sa[v3] as usize] {
        std::mem::swap(&mut v2, &mut v3);
    }
    if isad[sa[v4] as usize] > isad[sa[v5] as usize] {
        std::mem::swap(&mut v4, &mut v5);
    }
    if isad[sa[v2] as usize] > isad[sa[v4] as usize] {
        std::mem::swap(&mut v2, &mut v4);
        std::mem::swap(&mut v3, &mut v5);
    }
    if isad[sa[v1] as usize] > isad[sa[v3] as usize] {
        std::mem::swap(&mut v1, &mut v3);
    }
    if isad[sa[v1] as usize] > isad[sa[v4] as usize] {
        std::mem::swap(&mut v1, &mut v4);
        std::mem::swap(&mut v3, &mut v5);
    }
    if isad[sa[v3] as usize] > isad[sa[v4] as usize] {
        v4
    } else {
        v3
    }
}

fn tr_pivot_idx(isad: &[i32], sa: &[i32], first: usize, last: usize) -> usize {
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

struct TrBudget {
    chance: i32,
    remain: i32,
    incval: i32,
    pub count: i32,
}

impl TrBudget {
    const fn new(chance: i32, incval: i32) -> Self {
        Self {
            chance,
            remain: incval,
            incval,
            count: 0,
        }
    }

    #[inline(always)]
    const fn check(&mut self, size: i32) -> bool {
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
fn tr_partition(
    isad: &[i32],
    sa: &mut [i32],
    first: usize,
    middle: usize,
    last: usize,
    v: i32,
) -> (usize, usize) {
    let mut b = middle;
    let mut x = 0i32;
    while b < last {
        x = isad[sa[b] as usize];
        if x != v {
            break;
        }
        b += 1;
    }
    let mut a = b;
    if a < last && x < v {
        b += 1;
        while b < last {
            x = isad[sa[b] as usize];
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
        x = isad[sa[c] as usize];
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
            x = isad[sa[c] as usize];
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
            x = isad[sa[b] as usize];
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
            x = isad[sa[c] as usize];
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

fn tr_copy(
    isa: &mut [i32],
    sa: &mut [i32],
    first: usize,
    a: usize,
    b: usize,
    last: usize,
    depth: i32,
) {
    let v = (b as i32) - 1; // b - SA - 1 where SA=0
    // C: for(c = first, d = a - 1; c <= d; ++c) { if(...) { *++d = s; ISA[s] = d - SA; } }
    // Rust: d starts at a (= C's d+1), write at d then d++; loop while c < d (= C's c <= d)
    let mut d = a;
    let mut c = first;
    while c < d {
        let s = sa[c] - depth;
        if s >= 0 && isa[s as usize] == v {
            sa[d] = s;
            isa[s as usize] = d as i32;
            d += 1;
        }
        c += 1;
    }
    // C: for(c = last - 1, e = d + 1, d = b - 1; e < d; --c) { if(...) { *--d = s; ISA[s] = d - SA; } }
    // C initializes d = b-1 and uses *--d (pre-decrement). Rust e = C's (d+1) from first loop.
    // C initial check: e < b-1. Rust initial check: e >= d2. So d2 must start at b-1.
    let e = d;
    let mut d2 = b - 1;
    if last > 0 {
        let mut c2 = last - 1;
        loop {
            if e >= d2 {
                break;
            } // C condition: e < d
            let s = sa[c2] - depth;
            if s >= 0 && isa[s as usize] == v {
                d2 -= 1;
                sa[d2] = s;
                isa[s as usize] = d2 as i32;
            }
            if c2 == 0 {
                break;
            }
            c2 -= 1;
        }
    }
}

fn tr_partialcopy(
    isa: &mut [i32],
    sa: &mut [i32],
    first: usize,
    a: usize,
    b: usize,
    last: usize,
    depth: i32,
) {
    let v = (b as i32) - 1;
    // C: for(c = first, d = a - 1; c <= d; ++c) — d extends dynamically
    let mut d = a;
    let mut lastrank = -1i32;
    let mut newrank = -1i32;
    let mut c = first;
    while c < d {
        let s = sa[c] - depth;
        if s >= 0 && isa[s as usize] == v {
            sa[d] = s;
            let rank = isa[(s + depth) as usize];
            if lastrank != rank {
                lastrank = rank;
                newrank = d as i32;
            }
            isa[s as usize] = newrank;
            d += 1;
        }
        c += 1;
    }

    // C: for(e = d; first <= e; --e) — processes [first, d] inclusive (C's d = Rust's d-1)
    lastrank = -1;
    let mut e = d;
    while e > first {
        e -= 1;
        let rank = isa[sa[e] as usize];
        if lastrank != rank {
            lastrank = rank;
            newrank = e as i32;
        }
        if newrank != rank {
            isa[sa[e] as usize] = newrank;
        }
    }

    // C: for(c = last - 1, e = d + 1, d = b - 1; e < d; --c)
    // C initializes d = b-1 and uses *--d (pre-decrement). Rust e2 = C's (d+1) from first loop.
    lastrank = -1;
    let e2 = d;
    let mut d2 = b - 1;
    if last > 0 {
        let mut c2 = last - 1;
        loop {
            if e2 >= d2 {
                break;
            } // C condition: e < d (equivalent with d2 = b-1)
            let s = sa[c2] - depth;
            if s >= 0 && isa[s as usize] == v {
                d2 -= 1;
                sa[d2] = s;
                let rank = isa[(s + depth) as usize];
                if lastrank != rank {
                    lastrank = rank;
                    newrank = d2 as i32;
                }
                isa[s as usize] = newrank;
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
fn tr_handle_tandem_partition(
    isa: &mut [i32],
    sa: &mut [i32],
    stack: &mut FixedStack<TrFrame, { TR_STACKSIZE }>,
    state: &mut TrSortState<'_>,
    incr: usize,
) -> bool {
    // Safety: isa and sa are non-overlapping (split_at_mut(m) in sort_typebstar).
    // tr_partition only reads from isad_tandem and swaps elements in sa.
    let isad_tandem: &[i32] = unsafe {
        let offset = *state.isad - incr;
        std::slice::from_raw_parts(isa.as_ptr().add(offset), isa.len() - offset)
    };
    let (a, b) = tr_partition(
        isad_tandem,
        sa,
        *state.first,
        *state.first,
        *state.last,
        (*state.last as i32) - 1,
    );

    if a < *state.last {
        let v = (a as i32) - 1;
        for k in *state.first..a {
            isa[sa[k] as usize] = v;
        }
    }
    if b < *state.last {
        let v = (b as i32) - 1;
        for k in a..b {
            isa[sa[k] as usize] = v;
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
                limit: tr_ilg((*state.last - b) as i32),
                trlink: *state.trlink,
            });
            *state.last = a;
            *state.limit = tr_ilg((a - *state.first) as i32);
        } else if *state.last - b > 1 {
            *state.first = b;
            *state.limit = tr_ilg((*state.last - b) as i32);
        } else {
            return tr_pop_stack(stack, state);
        }
    } else if *state.last - b > 1 {
        stack.push(TrFrame {
            isad: *state.isad,
            first: *state.first,
            last: a,
            limit: tr_ilg((a - *state.first) as i32),
            trlink: *state.trlink,
        });
        *state.first = b;
        *state.limit = tr_ilg((*state.last - b) as i32);
    } else if a - *state.first > 1 {
        *state.last = a;
        *state.limit = tr_ilg((a - *state.first) as i32);
    } else {
        return tr_pop_stack(stack, state);
    }
    true
}

/// Handles the negate/scan/budget case (limit < -2) in tr_introsort.
/// Returns `false` if tr_introsort should return (stack exhausted).
#[inline(always)]
fn tr_handle_negate_scan(
    isa: &mut [i32],
    sa: &mut [i32],
    stack: &mut FixedStack<TrFrame, { TR_STACKSIZE }>,
    state: &mut TrSortState<'_>,
    incr: usize,
    budget: &mut TrBudget,
) -> bool {
    if sa[*state.first] >= 0 {
        let mut a = *state.first;
        while a < *state.last && sa[a] >= 0 {
            isa[sa[a] as usize] = a as i32;
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
        sa[a] = !sa[a];
        a += 1;
        if a >= last || sa[a] >= 0 {
            break;
        }
    }
    let next = if a < last && isa[sa[a] as usize] != isa[isad + sa[a] as usize] {
        tr_ilg((a - first + 1) as i32)
    } else {
        -1
    };
    let run_end = if a < last {
        a += 1;
        if a < last {
            let v = (a as i32) - 1;
            for k in first..a {
                isa[sa[k] as usize] = v;
            }
        }
        a
    } else {
        last
    };

    if budget.check((run_end - first) as i32) {
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

fn tr_introsort(
    isa: &mut [i32],
    isad_init: usize,
    sa: &mut [i32],
    sa_first: usize,
    sa_last: usize,
    budget: &mut TrBudget,
) {
    let incr = isad_init; // ISAd - ISA = isad_init (ISA is 0-based)
    let mut stack = FixedStack::<TrFrame, { TR_STACKSIZE }>::new();

    let mut isad = isad_init;
    let mut first = sa_first;
    let mut last = sa_last;
    let mut limit = tr_ilg((last - first) as i32);
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
                    tr_copy(isa, sa, first, a, b, last, isad as i32);
                } else {
                    if trlink >= 0 {
                        stack[trlink as usize].limit = -1;
                    }
                    tr_partialcopy(isa, sa, first, a, b, last, isad as i32);
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
                let x = isa[isad + sa[a] as usize];
                let mut b = a;
                loop {
                    if b <= first || sa[b - 1] < 0 || isa[isad + sa[b - 1] as usize] != x {
                        break;
                    }
                    b -= 1;
                    sa[b] = !sa[b];
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
        let v = isa[isad + sa[first] as usize];

        // Same aliasing argument as the isad_tandem case above: tr_partition reads from
        // isad_slice (= isa[isad..]) and swaps within sa; isa and sa are disjoint.
        // After tr_partition returns the slice is no longer live, so the subsequent reads
        // and writes to isa are safe with no aliased reference outstanding.
        let isad_slice: &[i32] =
            unsafe { std::slice::from_raw_parts(isa.as_ptr().add(isad), isa.len() - isad) };
        let (a, b) = tr_partition_owned(isad_slice, sa, first, first + 1, last, v);

        if last - first != b - a {
            let next = if isa[sa[a] as usize] != v {
                tr_ilg((b - a) as i32)
            } else {
                -1
            };

            let v2 = (a as i32) - 1;
            for k in first..a {
                isa[sa[k] as usize] = v2;
            }
            if b < last {
                let v3 = (b as i32) - 1;
                for k in a..b {
                    isa[sa[k] as usize] = v3;
                }
            }

            if b - a > 1 && budget.check((b - a) as i32) {
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
        } else if budget.check((last - first) as i32) {
            limit = tr_ilg((last - first) as i32);
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

fn tr_partition_owned(
    isad: &[i32],
    sa: &mut [i32],
    first: usize,
    middle: usize,
    last: usize,
    v: i32,
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

pub fn trsort(isa: &mut [i32], sa: &mut [i32], n: i32, depth: i32) {
    let n = n as usize;
    let mut budget = TrBudget::new(tr_ilg(n as i32) * 2 / 3, n as i32);

    let mut isad = depth as usize;
    while sa[0] > -(n as i32) {
        let mut first = 0usize;
        let mut skip = 0i32;
        let mut unsorted = 0i32;

        loop {
            let t = sa[first];
            if t < 0 {
                first = (first as i32 - t) as usize;
                skip += t;
            } else {
                if skip != 0 {
                    // C: *(first + skip) = skip where skip is negative (signed i32 pointer arithmetic)
                    sa[(first as i32 + skip) as usize] = skip;
                    skip = 0;
                }
                let last = (isa[t as usize] + 1) as usize;
                if last - first > 1 {
                    budget.count = 0;
                    tr_introsort(isa, isad, sa, first, last, &mut budget);
                    if budget.count != 0 {
                        unsorted += budget.count;
                    } else {
                        skip = first as i32 - last as i32;
                    }
                } else if last - first == 1 {
                    skip = -1;
                }
                first = last;
            }
            if first >= n {
                break;
            }
        }
        if skip != 0 {
            sa[(first as i32 + skip) as usize] = skip;
        }
        if unsorted == 0 {
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
            assert!(isad[sa[i - 1] as usize] <= isad[sa[i] as usize]);
        }
    }
}
