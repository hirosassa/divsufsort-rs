use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use divsufsort_rs::{divbwt, divsufsort};

fn lcg_next(seed: &mut u64) -> u8 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

fn corpus_random_binary(size: usize) -> Vec<u8> {
    let mut seed = 0x517cc1b727220a95u64;
    (0..size).map(|_| lcg_next(&mut seed)).collect()
}

fn corpus_text(size: usize) -> Vec<u8> {
    let mut seed = 0x517cc1b727220a95u64;
    (0..size)
        .map(|_| b'a' + (lcg_next(&mut seed) % 26))
        .collect()
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

fn corpus_repeat(size: usize) -> Vec<u8> {
    const UNIT: &[u8] = b"the quick brown fox jumps over the lazy dog. ";
    UNIT.iter().copied().cycle().take(size).collect()
}

type Corpus = (&'static str, fn(usize) -> Vec<u8>);

fn bench_divsufsort_light(c: &mut Criterion) {
    const SIZES: &[usize] = &[1_000, 10_000, 100_000, 1_000_000];

    let corpora: &[Corpus] = &[
        ("random_binary", corpus_random_binary),
        ("text_26", corpus_text),
        ("fibonacci", corpus_fibonacci),
        ("repeat", corpus_repeat),
    ];

    for &(name, func) in corpora {
        let mut group = c.benchmark_group(name);
        group.sample_size(10);

        for &size in SIZES {
            let data = func(size);
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
                // Allocate the suffix array outside the timed section.
                b.iter_batched_ref(
                    || vec![0i32; data.len()],
                    |sa| divsufsort(data, sa).unwrap(),
                    BatchSize::LargeInput,
                )
            });
        }

        group.finish();
    }
}

fn bench_divbwt_light(c: &mut Criterion) {
    const SIZES: &[usize] = &[1_000, 10_000, 100_000, 1_000_000];

    let mut group = c.benchmark_group("divbwt/text_26");
    group.sample_size(10);

    for &size in SIZES {
        let data = corpus_text(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter_batched_ref(
                || vec![0u8; data.len()],
                |u| divbwt(data, u, None).unwrap(),
                BatchSize::LargeInput,
            )
        });
    }

    group.finish();
}

criterion_group!(benches, bench_divsufsort_light, bench_divbwt_light);
criterion_main!(benches);
