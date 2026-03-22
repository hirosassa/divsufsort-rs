#[cfg(feature = "c-bench")]
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
#[cfg(feature = "c-bench")]
use divsufsort_rs::divsufsort as divsufsort_rust;
#[cfg(feature = "c-bench")]
use divsufsort_rs::ffi::divsufsort_c;

#[cfg(feature = "c-bench")]
fn lcg_next(seed: &mut u64) -> u8 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

#[cfg(feature = "c-bench")]
fn corpus_random_binary(size: usize) -> Vec<u8> {
    let mut seed = 0x517cc1b727220a95u64;
    (0..size).map(|_| lcg_next(&mut seed)).collect()
}

#[cfg(feature = "c-bench")]
fn corpus_text(size: usize) -> Vec<u8> {
    let mut seed = 0x517cc1b727220a95u64;
    (0..size)
        .map(|_| b'a' + (lcg_next(&mut seed) % 26))
        .collect()
}

#[cfg(feature = "c-bench")]
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

#[cfg(feature = "c-bench")]
fn bench_compare(c: &mut Criterion) {
    const SIZE: usize = 1_000_000;

    let corpora: &[(&str, fn(usize) -> Vec<u8>)] = &[
        ("random_binary", corpus_random_binary),
        ("text_26", corpus_text),
        ("fibonacci", corpus_fibonacci),
    ];

    for &(name, func) in corpora {
        let data = func(SIZE);

        let mut group = c.benchmark_group(name);
        group.sample_size(10);
        group.throughput(Throughput::Bytes(SIZE as u64));

        group.bench_with_input(BenchmarkId::new("rust", SIZE), &data, |b, data| {
            b.iter(|| {
                let mut sa = vec![0i32; data.len()];
                divsufsort_rust(data, &mut sa).unwrap();
            })
        });

        group.bench_with_input(BenchmarkId::new("c", SIZE), &data, |b, data| {
            b.iter(|| {
                let mut sa = vec![0i32; data.len()];
                divsufsort_c(data, &mut sa);
            })
        });

        group.finish();
    }
}

#[cfg(feature = "c-bench")]
criterion_group!(benches, bench_compare);
#[cfg(feature = "c-bench")]
criterion_main!(benches);

#[cfg(not(feature = "c-bench"))]
fn main() {
    eprintln!("This benchmark requires the c-bench feature.");
    eprintln!("Run: cargo bench --bench bench_compare --features c-bench");
}
