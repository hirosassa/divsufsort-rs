//! Pins the exact suffix array / BWT output for deterministic inputs, so that
//! performance refactorings can be shown to leave the output unchanged.

use divsufsort_rs::{divbwt, divsufsort};

const fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

fn fnv1a(values: impl Iterator<Item = u64>) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for v in values {
        h ^= v;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn sa_checksum(t: &[u8]) -> u64 {
    let mut sa = vec![0i32; t.len()];
    divsufsort(t, &mut sa).unwrap();
    fnv1a(sa.iter().map(|&x| x as u64))
}

fn bwt_checksum(t: &[u8]) -> u64 {
    let mut u = vec![0u8; t.len()];
    let idx = divbwt(t, &mut u, None).unwrap();
    fnv1a(u.iter().map(|&x| x as u64).chain([idx as u64]))
}

fn random(seed: u64, n: usize, alphabet: u64) -> Vec<u8> {
    let mut s = seed;
    (0..n)
        .map(|_| b'a' + (lcg(&mut s) % alphabet) as u8)
        .collect()
}

fn fibonacci(n: usize) -> Vec<u8> {
    let (mut s0, mut s1) = (vec![b'b'], vec![b'a']);
    while s1.len() < n {
        let mut s2 = s1.clone();
        s2.extend_from_slice(&s0);
        s0 = s1;
        s1 = s2;
    }
    s1.truncate(n);
    s1
}

fn periodic(unit: &[u8], n: usize) -> Vec<u8> {
    unit.iter().copied().cycle().take(n).collect()
}

/// (name, input) pairs covering random, skewed, repetitive and tiny inputs.
fn corpora() -> Vec<(&'static str, Vec<u8>)> {
    let mut v = vec![
        ("binary_50k", random(1, 50_000, 2)),
        ("dna_50k", random(2, 50_000, 4)),
        ("text26_50k", random(3, 50_000, 26)),
        ("byte256_50k", {
            let mut s = 4u64;
            (0..50_000).map(|_| lcg(&mut s) as u8).collect()
        }),
        ("fibonacci_100k", fibonacci(100_000)),
        (
            "repeat_sentence_100k",
            periodic(b"the quick brown fox jumps over the lazy dog. ", 100_000),
        ),
        (
            "period36_binary",
            periodic(b"babbaabbbbabbbaababaabaabaabbbbbbabb", 20_000),
        ),
        ("all_same_20k", vec![b'a'; 20_000]),
    ];
    for n in 2..=64usize {
        v.push(("tiny", random(n as u64, n, 3)));
    }
    v
}

#[test]
fn sa_and_bwt_checksums_are_stable() {
    let mut sa_sums = Vec::new();
    let mut bwt_sums = Vec::new();
    for (_, t) in corpora() {
        sa_sums.push(sa_checksum(&t));
        bwt_sums.push(bwt_checksum(&t));
    }
    let combined = fnv1a(sa_sums.iter().chain(&bwt_sums).copied());
    assert_eq!(combined, EXPECTED, "SA/BWT output changed");
}

const EXPECTED: u64 = 58_659_796_404_886_630;
