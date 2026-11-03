//! Small inputs that exercise the parallel B*-suffix sort path (several non-trivial
//! buckets, and one bucket spanning several SS_BLOCKSIZE blocks). Sized to stay fast
//! under `cargo +nightly miri test --test miri_smoke`.
use divsufsort_rs::{divsufsort, sufcheck};

fn assert_sufcheck(t: &[u8]) {
    let mut sa = vec![0i32; t.len()];
    divsufsort(t, &mut sa).unwrap();
    sufcheck(t, &sa, false).unwrap_or_else(|e| panic!("sufcheck failed: {}", e.message));
}

fn lcg_text(seed: u64, n: usize, alphabet: u64) -> Vec<u8> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            b'a' + ((state >> 33) % alphabet) as u8
        })
        .collect()
}

#[test]
fn miri_smoke_many_buckets() {
    assert_sufcheck(&lcg_text(1, 1500, 4));
    assert_sufcheck(&lcg_text(2, 1500, 26));
}

#[test]
fn miri_smoke_bucket_spanning_blocks() {
    let t: Vec<u8> = b"ab".iter().copied().cycle().take(2400).collect();
    assert_sufcheck(&t);
}
