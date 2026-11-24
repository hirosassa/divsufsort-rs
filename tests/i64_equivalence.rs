//! Differential tests: the `i64` index type must behave exactly like `i32` for every
//! public entry point, on inputs that both can represent.

use divsufsort_rs::{
    DivSufSortError, SaIndex, bw_transform, divbwt, divsufsort, inverse_bw_transform, sa_search,
    sa_simplesearch, sufcheck,
};

fn random(seed: u64, n: usize, alphabet: u8) -> Vec<u8> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as u8) % alphabet
        })
        .collect()
}

fn inputs() -> Vec<Vec<u8>> {
    let mut v = vec![
        random(1, 3000, 2),
        random(2, 20_000, 4),
        random(3, 50_000, 26),
        random(4, 20_000, 255),
        vec![7u8; 5000],
        b"abcabcabcabcabcabcabcabcabcabc".repeat(100),
        b"mississippi".to_vec(),
    ];
    for n in 0..=40usize {
        v.push(random(100 + n as u64, n, 3));
    }
    v
}

fn widen(sa: &[i32]) -> Vec<i64> {
    sa.iter().map(|&x| i64::from(x)).collect()
}

#[test]
fn divsufsort_i32_and_i64_agree() {
    for t in inputs() {
        let mut a = vec![0i32; t.len()];
        let mut b = vec![0i64; t.len()];
        divsufsort(&t, &mut a).unwrap();
        divsufsort(&t, &mut b).unwrap();
        assert_eq!(widen(&a), b, "n = {}", t.len());
        sufcheck(&t, &b, false).unwrap();
    }
}

#[test]
fn divbwt_i32_and_i64_agree() {
    for t in inputs() {
        let mut u32_out = vec![0u8; t.len()];
        let mut u64_out = vec![0u8; t.len()];
        let p32 = divbwt::<i32>(&t, &mut u32_out, None).unwrap();
        let p64 = divbwt::<i64>(&t, &mut u64_out, None).unwrap();
        assert_eq!((u32_out, i64::from(p32)), (u64_out, p64), "n = {}", t.len());
    }
}

#[test]
fn bw_transform_and_inverse_i32_and_i64_agree() {
    for t in inputs().into_iter().filter(|t| t.len() > 1) {
        let n = t.len();
        let mut sa32 = vec![0i32; n];
        let mut sa64 = vec![0i64; n];
        divsufsort(&t, &mut sa32).unwrap();
        divsufsort(&t, &mut sa64).unwrap();

        let (mut u32_out, mut u64_out) = (vec![0u8; n], vec![0u8; n]);
        let (mut i32_idx, mut i64_idx) = (0i32, 0i64);
        bw_transform(&t, &mut u32_out, Some(&mut sa32), &mut i32_idx).unwrap();
        bw_transform(&t, &mut u64_out, Some(&mut sa64), &mut i64_idx).unwrap();
        assert_eq!((&u32_out, i64::from(i32_idx)), (&u64_out, i64_idx));

        let (mut r32, mut r64) = (vec![0u8; n], vec![0u8; n]);
        inverse_bw_transform::<i32>(&u32_out, &mut r32, None, i32_idx).unwrap();
        inverse_bw_transform::<i64>(&u64_out, &mut r64, None, i64_idx).unwrap();
        assert_eq!(r32, t);
        assert_eq!(r64, t);
    }
}

#[test]
fn searches_i32_and_i64_agree() {
    let t = b"abracadabra abracadabra banana bandana".repeat(20);
    let mut sa32 = vec![0i32; t.len()];
    let mut sa64 = vec![0i64; t.len()];
    divsufsort(&t, &mut sa32).unwrap();
    divsufsort(&t, &mut sa64).unwrap();

    let patterns: [&[u8]; 6] = [
        b"abra",
        b"ana",
        b"ban",
        b"zzz",
        b"a",
        b"abracadabra abracadabra b",
    ];
    for p in patterns {
        let (c32, l32) = sa_search(&t, p, &sa32);
        let (c64, l64) = sa_search(&t, p, &sa64);
        assert_eq!(i64::from(c32), c64, "count for {p:?}");
        if c64 > 0 {
            assert_eq!(i64::from(l32), l64, "left for {p:?}");
        }
    }
    for c in *b"ab z" {
        let (c32, l32) = sa_simplesearch(&t, &sa32, c);
        let (c64, l64) = sa_simplesearch(&t, &sa64, c);
        assert_eq!(i64::from(c32), c64);
        if c64 > 0 {
            assert_eq!(i64::from(l32), l64);
        }
    }
}

#[test]
fn argument_validation_matches_for_both_index_types() {
    fn check<I: SaIndex>() {
        let t = b"banana";
        let mut short = vec![I::ZERO; 5];
        assert_eq!(
            divsufsort(t, &mut short),
            Err(DivSufSortError::InvalidArgument)
        );
        let mut u = [0u8; 5];
        assert_eq!(
            divbwt::<I>(t, &mut u, None),
            Err(DivSufSortError::InvalidArgument)
        );
        let mut u = [0u8; 6];
        let mut scratch = vec![I::ZERO; 5];
        assert_eq!(
            divbwt(t, &mut u, Some(&mut scratch)),
            Err(DivSufSortError::InvalidArgument)
        );
        assert_eq!(
            inverse_bw_transform(b"annbaa", &mut u, None, I::from_usize(7)),
            Err(DivSufSortError::InvalidArgument)
        );
    }
    check::<i32>();
    check::<i64>();
}
