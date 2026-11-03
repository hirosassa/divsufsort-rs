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

> [!NOTE]
> Since 0.7.0, the `std` feature no longer enables rayon. If you previously depended on `std` for parallelism and disabled default features, enable the `rayon` feature explicitly (it implies `std`):
>
> ```toml
> divsufsort-rs = { version = "...", default-features = false, features = ["rayon"] }
> ```

> [!IMPORTANT]
> This crate keeps `unsafe` to a minimum. Outside the `c-bench` FFI, it is limited to two shared reborrows of the `isa` slice in `trsort`. The parallel B\*-bucket sort is safe code: the suffix array is split into disjoint `&mut` chunks (`split_at_mut`), and hot loops avoid bounds checks through fixed-size bucket tables. The public API is fully safe.

## Usage

```toml
[dependencies]
divsufsort-rs = "0.7"
```

```rust
use divsufsort_rs::divsufsort;

fn main() {
    let text = b"banana";
    let mut sa = vec![0i32; text.len()];
    divsufsort(text, &mut sa).unwrap();
    // indices of sorted suffixes
    assert_eq!(sa, [5, 3, 1, 0, 4, 2]);
}
```

BWT construction is also available:

```rust
use divsufsort_rs::divbwt;

let text = b"banana";
let mut bwt = vec![0u8; text.len()];
let primary_index = divbwt(text, &mut bwt, None).unwrap();
assert_eq!(&bwt, b"annbaa");
assert_eq!(primary_index, 4);
```

## Benchmark

Benchmarks run with [criterion](https://github.com/bheisler/criterion.rs) (`sample_size = 10`).

### Environment

| Item | Value |
|---|---|
| CPU | Apple M4 Max (16 logical cores) |
| OS | macOS 26.7.1 |
| Rust | 1.99.1 |

### Corpora

| Name | Description |
|---|---|
| `random_binary` | LCG pseudo-random bytes (alphabet size 256) |
| `text_26` | LCG pseudo-random lowercase ASCII (alphabet size 26) |
| `fibonacci` | Fibonacci string over `{a, b}` (highly repetitive) |
| `repeat` | A short English sentence repeated (periodic input; `bench_light` only) |

### Results — Rust vs C libdivsufsort (1,000,000 bytes)

Compared against the original **C libdivsufsort** compiled at `-O3`.

| Corpus | Rust (this crate) | C libdivsufsort | Ratio |
|---|---|---|---|
| random_binary | 10.4 ms (91.3 MiB/s) | 13.3 ms (71.5 MiB/s) | **1.28× faster** |
| text_26 | 12.3 ms (77.6 MiB/s) | 22.7 ms (42.1 MiB/s) | **1.85× faster** |
| fibonacci | 28.9 ms (33.0 MiB/s) | 26.9 ms (35.4 MiB/s) | 0.93× |

The parallel B\*-bucket sort drives the speedup for `random_binary` and `text_26`. For `fibonacci` the input produces only 1–2 non-trivial buckets, so parallelism provides no benefit and C is slightly faster due to lower single-thread overhead.

### Results — `rayon` (parallel) vs serial

Shows the effect of rayon parallelism. The default feature set includes both `std` and `rayon`; disabling default features runs the serial path.

| Corpus | Size | `rayon` | serial | Ratio |
|---|---|---|---|---|
| random_binary | 100K | 1.21 ms | 1.32 ms | 1.10× |
| random_binary | 1M | 10.8 ms | 16.9 ms | **1.57×** |
| text_26 | 100K | 1.20 ms | 1.93 ms | **1.61×** |
| text_26 | 1M | 13.1 ms | 27.5 ms | **2.10×** |
| fibonacci | 100K | 2.34 ms | 2.32 ms | 0.99× |
| fibonacci | 1M | 30.0 ms | 30.0 ms | 1.00× |

For corpora with many distinct B\*-buckets (`random_binary`, `text_26`), rayon parallelism provides 1.6–2.1× speedup at 1M scale. Highly repetitive inputs (`fibonacci`) show no difference as they produce too few buckets to benefit from parallelism. For small inputs the thread-pool overhead dominates: at 1K bytes, `text_26` takes about 224 µs with `rayon` versus 88 µs serial, so consider disabling the `rayon` feature if you mostly sort small inputs.

### Running the benchmarks

```sh
# Rust vs C comparison (bench_compare)
# requires the vendored C submodule — initialize it first:
git submodule update --init
cargo bench --bench bench_compare --features c-bench

# Lightweight benchmark — 4 corpora × 4 sizes (1K–1M) plus a divbwt group (bench_light)
cargo bench --bench bench_light

# Same benchmark in serial mode (`no_std` + no Rayon)
cargo bench --bench bench_light --no-default-features

# Same benchmark with `std` but without Rayon
cargo bench --bench bench_light --no-default-features --features std

# Full benchmark — larger sizes and more corpora, takes significantly longer (bench)
cargo bench --bench bench
```

## Releasing

1. Move the entries under `## [Unreleased]` in `CHANGELOG.md` to a new `## [x.y.z] - YYYY-MM-DD` section and update the link references at the bottom.
2. Run `cargo release <level> --execute` to bump the version in `Cargo.toml`, commit, tag `vx.y.z` and push.
3. The `release` workflow validates that the tag matches `Cargo.toml` and that `CHANGELOG.md` has notes for the version, publishes to crates.io, and creates a GitHub Release whose body is that CHANGELOG section.
