//! Deterministic fuzz / property tests for suffix array construction.
//!
//! Inputs come from a self-contained xorshift generator (no extra dependencies), so every
//! failure is reproducible from the case label printed in the assertion message.
//! The generators favour structures that stress the array-rewriting kernels
//! (`ss_mergebackward`, `tr_copy`, ...): periodic strings, Fibonacci / Thue-Morse words and
//! small-alphabet random text.
//!
//! Properties checked for every input:
//! 1. `divsufsort` output passes `sufcheck`.
//! 2. For `n <= NAIVE_MAX_LEN` it equals the naive comparison-sorted suffix array.
//! 3. The `i32` and `i64` index types produce the same array.
//! 4. `divbwt` followed by `inverse_bw_transform` restores the input.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreadable_literal,
    reason = "test code"
)]

use divsufsort_rs::{divbwt, divsufsort, inverse_bw_transform, sufcheck};

/// Naive suffix sorting is O(n^2 log n) worst case on periodic text, so keep it small.
const NAIVE_MAX_LEN: usize = 2000;

struct XorShift64(u64);

impl XorShift64 {
    const fn new(seed: u64) -> Self {
        // xorshift has an all-zero fixed point.
        Self(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }

    const fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    const fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn byte_below(&mut self, bound: usize) -> u8 {
        u8::try_from(self.below(bound)).unwrap()
    }
}

/// `period`-periodic text of length `n`; the last period is cut short (a "tail fragment").
fn periodic(rng: &mut XorShift64, period: usize, n: usize, alphabet: usize) -> Vec<u8> {
    let unit: Vec<u8> = (0..period).map(|_| rng.byte_below(alphabet)).collect();
    (0..n).map(|i| unit[i % period]).collect()
}

fn fibonacci(n: usize) -> Vec<u8> {
    let mut prev = vec![b'b'];
    let mut cur = vec![b'a'];
    while cur.len() < n {
        let next = [cur.as_slice(), prev.as_slice()].concat();
        prev = cur;
        cur = next;
    }
    cur.truncate(n);
    cur
}

fn thue_morse(n: usize) -> Vec<u8> {
    (0..n).map(|i| b'a' + (i.count_ones() % 2) as u8).collect()
}

fn random_text(rng: &mut XorShift64, n: usize, alphabet: usize) -> Vec<u8> {
    (0..n).map(|_| rng.byte_below(alphabet)).collect()
}

fn naive_suffix_array(t: &[u8]) -> Vec<i32> {
    let mut sa: Vec<i32> = (0..i32::try_from(t.len()).unwrap()).collect();
    sa.sort_by(|&a, &b| t[a as usize..].cmp(&t[b as usize..]));
    sa
}

fn check_all_properties(label: &str, t: &[u8]) {
    let n = t.len();

    let mut sa32 = vec![0i32; n];
    divsufsort(t, &mut sa32).unwrap_or_else(|e| panic!("{label}: divsufsort i32 failed: {e}"));
    sufcheck(t, &sa32, false).unwrap_or_else(|e| panic!("{label} (n={n}): sufcheck failed: {e:?}"));

    if n <= NAIVE_MAX_LEN {
        assert_eq!(
            sa32,
            naive_suffix_array(t),
            "{label} (n={n}): differs from naive SA"
        );
    }

    let mut sa64 = vec![0i64; n];
    divsufsort(t, &mut sa64).unwrap_or_else(|e| panic!("{label}: divsufsort i64 failed: {e}"));
    let widened: Vec<i64> = sa32.iter().copied().map(i64::from).collect();
    assert_eq!(widened, sa64, "{label} (n={n}): i32 and i64 SA differ");

    let mut bwt = vec![0u8; n];
    let idx =
        divbwt::<i32>(t, &mut bwt, None).unwrap_or_else(|e| panic!("{label}: divbwt failed: {e}"));
    let mut restored = vec![0u8; n];
    inverse_bw_transform::<i32>(&bwt, &mut restored, None, idx)
        .unwrap_or_else(|e| panic!("{label}: inverse_bw_transform failed: {e}"));
    assert_eq!(restored, t, "{label} (n={n}): BWT round trip mismatch");
}

fn run_periodic(seed: u64, lengths: &[usize], periods: &[usize]) {
    let mut rng = XorShift64::new(seed);
    for &n in lengths {
        for &period in periods {
            let alphabet = 1 + rng.below(4);
            let t = periodic(&mut rng, period, n, alphabet);
            check_all_properties(&format!("periodic(p={period}, alphabet={alphabet})"), &t);
        }
    }
}

/// Found by this harness: `tr_partition` computed `c = b - 1` (C pointer arithmetic that may
/// legitimately reach -1 when `first == 0`) in `usize`, panicking with a subtraction overflow.
#[test]
fn regression_tr_partition_block_at_index_zero() {
    check_all_properties(
        "tr_partition regression",
        &[0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1],
    );
}

#[test]
fn empty_and_tiny_inputs() {
    let mut rng = XorShift64::new(1);
    for n in 0..=16 {
        for alphabet in 1..=3 {
            let t = random_text(&mut rng, n, alphabet);
            check_all_properties(&format!("tiny(alphabet={alphabet})"), &t);
        }
    }
}

#[test]
fn periodic_strings_with_every_period_up_to_64() {
    let periods: Vec<usize> = (1..=64).collect();
    // Lengths are not multiples of the period so the tail fragment is exercised.
    run_periodic(2, &[67, 203, 1000, 1999], &periods);
}

#[test]
fn periodic_strings_medium() {
    let periods = [1, 2, 3, 5, 8, 13, 21, 34, 55, 64];
    run_periodic(3, &[5000, 12_345], &periods);
}

#[test]
fn fibonacci_words() {
    for n in [1, 2, 3, 5, 8, 13, 100, 233, 1000, 1999, 6765, 10_946] {
        check_all_properties("fibonacci", &fibonacci(n));
    }
}

#[test]
fn thue_morse_words() {
    for n in [1, 2, 3, 4, 7, 16, 100, 256, 1000, 1999, 4096, 10_000] {
        check_all_properties("thue_morse", &thue_morse(n));
    }
}

#[test]
fn small_alphabet_random_text() {
    let mut rng = XorShift64::new(4);
    for alphabet in 2..=4 {
        for n in [10, 100, 777, 2000, 9000, 25_000] {
            for _ in 0..3 {
                let t = random_text(&mut rng, n, alphabet);
                check_all_properties(&format!("random(alphabet={alphabet})"), &t);
            }
        }
    }
}

#[test]
fn random_lengths_up_to_a_few_thousand() {
    let mut rng = XorShift64::new(5);
    for _ in 0..200 {
        let n = rng.below(3000);
        let alphabet = 2 + rng.below(3);
        let t = if rng.below(2) == 0 {
            random_text(&mut rng, n, alphabet)
        } else {
            let period = 1 + rng.below(64);
            periodic(&mut rng, period, n, alphabet)
        };
        check_all_properties(&format!("mixed(alphabet={alphabet})"), &t);
    }
}

#[test]
#[ignore = "large inputs; run with `cargo test --release -- --ignored`"]
fn large_structured_inputs() {
    let n = 100_000;
    check_all_properties("fibonacci-100k", &fibonacci(n));
    check_all_properties("thue_morse-100k", &thue_morse(n));
    let mut rng = XorShift64::new(6);
    for period in [1, 2, 7, 31, 64] {
        let t = periodic(&mut rng, period, n + 13, 3);
        check_all_properties(&format!("periodic-100k(p={period})"), &t);
    }
    for alphabet in 2..=4 {
        let t = random_text(&mut rng, n, alphabet);
        check_all_properties(&format!("random-100k(alphabet={alphabet})"), &t);
    }
}

#[test]
#[ignore = "large inputs; run with `cargo test --release -- --ignored`"]
fn many_random_periodic_cases() {
    let mut rng = XorShift64::new(7);
    for _ in 0..300 {
        let n = rng.below(50_000);
        let period = 1 + rng.below(64);
        let alphabet = 1 + rng.below(4);
        let t = periodic(&mut rng, period, n, alphabet);
        check_all_properties(
            &format!("periodic-fuzz(p={period}, alphabet={alphabet})"),
            &t,
        );
    }
}
