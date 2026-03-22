use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use divsufsort_rs::divsufsort;

// Simple LCG for deterministic pseudo-random generation without external deps.
fn lcg_next(seed: &mut u64) -> u8 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

/// Uniform random bytes over the full 256-alphabet (mimics binary/executable files).
fn corpus_random_binary(size: usize) -> Vec<u8> {
    let mut seed = 0x517cc1b727220a95u64;
    (0..size).map(|_| lcg_next(&mut seed)).collect()
}

/// Uniform random lowercase ASCII (mimics natural-language text files).
fn corpus_text(size: usize) -> Vec<u8> {
    let mut seed = 0x517cc1b727220a95u64;
    (0..size)
        .map(|_| b'a' + (lcg_next(&mut seed) % 26))
        .collect()
}

/// Single repeated character — hardest case for many SA algorithms.
fn corpus_repetitive(size: usize) -> Vec<u8> {
    vec![b'a'; size]
}

/// Uniform random {A, C, G, T} (mimics DNA/protein corpus).
fn corpus_dna(size: usize) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ACGT";
    let mut seed = 0x517cc1b727220a95u64;
    (0..size)
        .map(|_| ALPHABET[(lcg_next(&mut seed) % 4) as usize])
        .collect()
}

/// Fibonacci word over {a, b} — structured repetition that stresses many algorithms.
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

fn bench_divsufsort(c: &mut Criterion) {
    const SIZES: &[usize] = &[100_000, 1_000_000, 10_000_000];

    let corpora: &[(&str, fn(usize) -> Vec<u8>)] = &[
        ("random_binary", corpus_random_binary),
        ("text_26",       corpus_text),
        ("repetitive",    corpus_repetitive),
        ("dna",           corpus_dna),
        ("fibonacci",     corpus_fibonacci),
    ];

    for &(name, func) in corpora {
        let mut group = c.benchmark_group(name);
        group.sample_size(10);

        for &size in SIZES {
            let data = func(size);
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                &data,
                |b, data| {
                    b.iter(|| {
                        let mut sa = vec![0i32; data.len()];
                        divsufsort(data, &mut sa).unwrap();
                    })
                },
            );
        }

        group.finish();
    }
}

criterion_group!(benches, bench_divsufsort);
criterion_main!(benches);
