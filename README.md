# divsufsort-rs

[![Crates.io](https://img.shields.io/crates/v/divsufsort-rs.svg)](https://crates.io/crates/divsufsort-rs)
[![Documentation](https://docs.rs/divsufsort-rs/badge.svg)](https://docs.rs/divsufsort-rs)
[![build](https://github.com/hirosassa/divsufsort-rs/actions/workflows/test.yaml/badge.svg?branch=main)](https://github.com/hirosassa/divsufsort-rs/actions/workflows/test.yaml)
[![codecov](https://codecov.io/gh/hirosassa/divsufsort-rs/branch/main/graph/badge.svg?token=gSFzgTfVwv)](https://codecov.io/gh/hirosassa/divsufsort-rs)

Pure Rust port of [libdivsufsort](https://github.com/y-256/libdivsufsort) — a fast suffix array construction library based on the induced-sorting (IS) algorithm.

## What it does

Constructs the **suffix array** of a byte string in O(n log n) time and 5n + O(1) memory space. A suffix array is a sorted array of all suffixes of a string, and is a fundamental data structure for string search, data compression (BWT), and bioinformatics.

The implementation closely follows [the original C library](https://github.com/y-256/libdivsufsort) by Yuta Mori.

The B\*-bucket sorting step is parallelised with [rayon](https://github.com/rayon-rs/rayon) when the `rayon` feature is enabled (default).

### `no_std` support

This crate supports `no_std` environments (with `alloc`). Disable the default features:

```toml
[dependencies]
divsufsort-rs = { version = "...", default-features = false }
```

If you want `std` without Rayon, enable only `std`:

```toml
[dependencies]
divsufsort-rs = { version = "...", default-features = false, features = ["std"] }
```

When the `rayon` feature is disabled, B\*-bucket sorting runs sequentially.

> [!IMPORTANT]
> This crate uses `unsafe` Rust internally for performance. Specifically, raw pointer aliasing is used to allow the suffix array and its read-only PA view to share the same allocation (mirroring the original C code), and bounds checks are elided in hot inner loops where invariants can be proven statically. The public API is fully safe.

## Usage

```toml
[dependencies]
divsufsort-rs = "0.4"
```

```rust
use divsufsort_rs::divsufsort;

fn main() {
    let text = b"banana";
    let mut sa = vec![0i32; text.len()];
    divsufsort(text, &mut sa).unwrap();
    // sa == [5, 3, 1, 0, 4, 2]  (indices of sorted suffixes)
    println!("{:?}", sa);
}
```

BWT construction is also available:

```rust
use divsufsort_rs::divbwt;

let text = b"banana";
let mut bwt = vec![0u8; text.len()];
let primary_index = divbwt(text, &mut bwt, None).unwrap();
// bwt == b"nnbaaa", primary_index == 3
```

## Benchmark

Benchmarks run with [criterion](https://github.com/bheisler/criterion.rs) (`sample_size = 10`).

### Environment

| Item | Value |
|---|---|
| CPU | Apple M4 Max (16 logical cores) |
| OS | macOS 15.5 |
| Rust | 1.92.0 |

### Corpora

| Name | Description |
|---|---|
| `random_binary` | LCG pseudo-random bytes (alphabet size 256) |
| `text_26` | LCG pseudo-random lowercase ASCII (alphabet size 26) |
| `fibonacci` | Fibonacci string over `{a, b}` (highly repetitive) |

### Results — Rust vs C libdivsufsort (1,000,000 bytes)

Compared against the original **C libdivsufsort** compiled at `-O3`.

| Corpus | Rust (this crate) | C libdivsufsort | Ratio |
|---|---|---|---|
| random_binary | 11.2 ms (84.9 MiB/s) | 13.7 ms (69.4 MiB/s) | **1.22× faster** |
| text_26 | 13.2 ms (72.4 MiB/s) | 23.8 ms (40.1 MiB/s) | **1.80× faster** |
| fibonacci | 30.1 ms (31.7 MiB/s) | 27.4 ms (34.8 MiB/s) | 0.91× |

The parallel B\*-bucket sort drives the speedup for `random_binary` and `text_26`. For `fibonacci` the input produces only 1–2 non-trivial buckets, so parallelism provides no benefit and C is slightly faster due to lower single-thread overhead.

### Results — `rayon` (parallel) vs serial

Shows the effect of rayon parallelism. The default feature set includes both `std` and `rayon`; disabling default features runs the serial path.

| Corpus | Size | `rayon` | serial | Ratio |
|---|---|---|---|---|
| random_binary | 100K | 1.29 ms | 1.44 ms | 1.12× |
| random_binary | 1M | 11.8 ms | 17.8 ms | **1.51×** |
| text_26 | 100K | 1.29 ms | 2.07 ms | **1.60×** |
| text_26 | 1M | 14.0 ms | 28.3 ms | **2.02×** |
| fibonacci | 100K | 2.53 ms | 2.47 ms | 0.98× |
| fibonacci | 1M | 31.5 ms | 31.0 ms | 0.98× |

For corpora with many distinct B\*-buckets (`random_binary`, `text_26`), rayon parallelism provides 1.5–2× speedup at 1M scale. Highly repetitive inputs (`fibonacci`) show no difference as they produce too few buckets to benefit from parallelism.

### Running the benchmarks

```sh
# Rust vs C comparison (bench_compare)
# requires the vendored C submodule — initialize it first:
git submodule update --init
cargo bench --bench bench_compare --features c-bench

# Lightweight benchmark — 3 corpora × 2 sizes, completes in ~2–3 minutes (bench_light)
cargo bench --bench bench_light

# Same benchmark in serial mode (`no_std` + no Rayon)
cargo bench --bench bench_light --no-default-features

# Same benchmark with `std` but without Rayon
cargo bench --bench bench_light --no-default-features --features std

# Full benchmark — larger sizes and more corpora, takes significantly longer (bench)
cargo bench --bench bench
```
