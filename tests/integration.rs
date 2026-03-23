use divsufsort_rs::{divbwt, divsufsort, inverse_bw_transform, sufcheck};

// Deterministic pseudo-random number generator using a simple LCG
fn lcg_next(state: &mut u64) -> u8 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 33) & 0xff) as u8
}

fn lcg_bytes(seed: u64, n: usize) -> Vec<u8> {
    let mut state = seed;
    (0..n).map(|_| lcg_next(&mut state)).collect()
}

// Helper: build a suffix array and verify it with sufcheck
fn assert_sufcheck(t: &[u8]) {
    let mut sa = vec![0i32; t.len()];
    divsufsort(t, &mut sa).unwrap();
    sufcheck(t, &sa, false)
        .unwrap_or_else(|e| panic!("sufcheck failed for input {:?}: {}", t, e.message));
}

// Helper: verify BWT round-trip (transform then inverse)
fn assert_bwt_roundtrip(t: &[u8]) {
    let n = t.len();
    let mut bwt = vec![0u8; n];
    let pidx = divbwt(t, &mut bwt, None).unwrap();
    let mut restored = vec![0u8; n];
    inverse_bw_transform(&bwt, &mut restored, None, pidx).unwrap();
    assert_eq!(restored, t, "BWT roundtrip failed for input {:?}", t);
}

fn corpus_fibonacci(size: usize) -> Vec<u8> {
    let mut s0: Vec<u8> = vec![b'b'];
    let mut s1: Vec<u8> = vec![b'a'];
    while s1.len() < size {
        let mut s2 = s1.clone();
        s2.extend_from_slice(&s0);
        s0 = s1;
        s1 = s2;
    }
    s1.truncate(size);
    s1
}

// ---- Long strings ----

#[test]
fn test_sufcheck_long_abcde_repeat() {
    // "abcde" repeated 100 times
    let t: Vec<u8> = b"abcde".iter().cycle().take(500).copied().collect();
    assert_sufcheck(&t);
}

#[test]
fn test_sufcheck_long_fibonacci_string() {
    // Fibonacci string F_12 ≈ 144 chars
    let mut s = b"a".to_vec();
    let mut prev = b"b".to_vec();
    for _ in 0..12 {
        let next = [prev.clone(), s.clone()].concat();
        s = prev;
        prev = next;
    }
    assert_sufcheck(&prev);
}

/// Uses a large Fibonacci string over a binary alphabet ('a','b'), which concentrates
/// nearly all B*-suffixes into a single bucket that exceeds the external buffer size.
/// This exercises the code path in ss_mergebackward where the next unsorted block is
/// used as the temporary merge buffer (curbufsize > bufsize), validating the correctness
/// of the cycle step `sa[X] = sa[a]` (a prior bug used `sa[a+1]`, causing duplicates).
#[test]
fn test_sufcheck_fibonacci_100k() {
    let t = corpus_fibonacci(100_000);
    assert_sufcheck(&t);
}

// ---- All-same character ----

#[test]
fn test_sufcheck_all_same_small() {
    for n in [1, 2, 3, 4, 5, 8] {
        let t = vec![b'x'; n];
        assert_sufcheck(&t);
    }
}

#[test]
fn test_sufcheck_all_same_large() {
    for n in [64, 128, 256, 1000] {
        let t = vec![b'a'; n];
        assert_sufcheck(&t);
    }
}

// ---- Binary alphabet ----

#[test]
fn test_sufcheck_binary_alphabet_short() {
    let cases: &[&[u8]] = &[
        b"ab",
        b"ba",
        b"aab",
        b"abb",
        b"aabb",
        b"abab",
        b"abba",
        b"aabbaabb",
    ];
    for &t in cases {
        assert_sufcheck(t);
    }
}

#[test]
fn test_sufcheck_binary_alphabet_long() {
    // alternating: ababab...
    let t: Vec<u8> = (0..200)
        .map(|i| if i % 2 == 0 { b'a' } else { b'b' })
        .collect();
    assert_sufcheck(&t);
}

#[test]
fn test_sufcheck_binary_alphabet_long2() {
    // aaa...bbb...
    let mut t = vec![b'a'; 100];
    t.extend(vec![b'b'; 100]);
    assert_sufcheck(&t);
}

// ---- Repeated patterns ----

#[test]
fn test_sufcheck_repeat_pattern_abc() {
    let t: Vec<u8> = b"abc".iter().cycle().take(300).copied().collect();
    assert_sufcheck(&t);
}

#[test]
fn test_sufcheck_repeat_pattern_single_char_variants() {
    // repeat "aaab"
    let t: Vec<u8> = b"aaab".iter().cycle().take(400).copied().collect();
    assert_sufcheck(&t);
}

// ---- Pseudo-random stress tests ----

#[test]
fn test_sufcheck_random_short() {
    for seed in 0..20u64 {
        for n in [3, 5, 7, 10, 15, 20] {
            let t = lcg_bytes(seed * 100 + n as u64, n);
            assert_sufcheck(&t);
        }
    }
}

#[test]
fn test_sufcheck_random_medium() {
    for seed in 0..10u64 {
        let t = lcg_bytes(seed, 500);
        assert_sufcheck(&t);
    }
}

#[test]
fn test_sufcheck_random_large() {
    for seed in 0..5u64 {
        let t = lcg_bytes(seed * 997 + 1, 5000);
        assert_sufcheck(&t);
    }
}

// Random bytes restricted to a 4-symbol alphabet
#[test]
fn test_sufcheck_random_small_alphabet() {
    for seed in 0..10u64 {
        let t: Vec<u8> = lcg_bytes(seed, 300).iter().map(|&b| b'a' + b % 4).collect();
        assert_sufcheck(&t);
    }
}

// ---- BWT round-trip ----

#[test]
fn test_bwt_roundtrip_known_strings() {
    let cases: &[&[u8]] = &[
        b"banana",
        b"mississippi",
        b"abracadabra",
        b"aaaaaa",
        b"abcdefghij",
    ];
    for &t in cases {
        assert_bwt_roundtrip(t);
    }
}

#[test]
fn test_bwt_roundtrip_random_short() {
    for seed in 0..20u64 {
        for n in [2, 5, 10, 20] {
            let t = lcg_bytes(seed * 50 + n as u64, n);
            assert_bwt_roundtrip(&t);
        }
    }
}

#[test]
fn test_bwt_roundtrip_random_large() {
    for seed in 0..3u64 {
        let t = lcg_bytes(seed * 1234 + 5, 2000);
        assert_bwt_roundtrip(&t);
    }
}

#[test]
fn test_bwt_roundtrip_all_same() {
    for n in [1, 2, 10, 100] {
        let t = vec![b'z'; n];
        assert_bwt_roundtrip(&t);
    }
}

#[test]
fn test_bwt_roundtrip_binary_repeat() {
    let t: Vec<u8> = (0..200)
        .map(|i| if i % 3 == 0 { b'a' } else { b'b' })
        .collect();
    assert_bwt_roundtrip(&t);
}

// ---- Full byte-value coverage ----

#[test]
fn test_sufcheck_all_256_bytes() {
    // string containing all byte values 0..=255
    let t: Vec<u8> = (0u8..=255u8).collect();
    assert_sufcheck(&t);
}

#[test]
fn test_bwt_roundtrip_all_256_bytes() {
    let t: Vec<u8> = (0u8..=255u8).collect();
    assert_bwt_roundtrip(&t);
}

#[test]
fn test_sufcheck_text26_1m() {
    fn lcg_corpus_text(size: usize) -> Vec<u8> {
        let mut seed = 0x517cc1b727220a95u64;
        (0..size)
            .map(|_| {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                b'a' + ((seed >> 33) as u8 % 26)
            })
            .collect()
    }
    let t = lcg_corpus_text(1_000_000);
    assert_sufcheck(&t);
}

fn brute_force_sa(t: &[u8]) -> Vec<i32> {
    let n = t.len();
    let mut sa: Vec<i32> = (0..n as i32).collect();
    sa.sort_by(|&a, &b| t[a as usize..].cmp(&t[b as usize..]));
    sa
}

#[test]
fn test_text26_small_correctness() {
    fn lcg_corpus_text(size: usize) -> Vec<u8> {
        let mut seed = 0x517cc1b727220a95u64;
        (0..size)
            .map(|_| {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                b'a' + ((seed >> 33) as u8 % 26)
            })
            .collect()
    }
    for size in [100, 500, 1000, 5000, 10000, 50000, 100000] {
        let t = lcg_corpus_text(size);
        let mut sa = vec![0i32; t.len()];
        divsufsort(&t, &mut sa).unwrap();
        let expected = brute_force_sa(&t);
        assert_eq!(sa, expected, "wrong SA at size={}", size);
    }
}

#[test]
fn test_text26_large_sufcheck() {
    fn lcg_corpus_text(size: usize) -> Vec<u8> {
        let mut seed = 0x517cc1b727220a95u64;
        (0..size)
            .map(|_| {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                b'a' + ((seed >> 33) as u8 % 26)
            })
            .collect()
    }
    for size in [200000, 500000, 1000000] {
        let t = lcg_corpus_text(size);
        let mut sa = vec![0i32; t.len()];
        divsufsort(&t, &mut sa).unwrap();
        sufcheck(&t, &sa, false).unwrap_or_else(|e| {
            panic!("WRONG at size={}: {}", size, e.message);
        });
    }
}
