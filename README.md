# divsufsort-rs

Pure Rust port of [libdivsufsort](https://github.com/y-256/libdivsufsort) — a fast suffix array construction library based on the induced-sorting (IS) algorithm.

## What it does

Constructs the **suffix array** of a byte string in O(n log n) time. A suffix array is a sorted array of all suffixes of a string, and is a fundamental data structure for string search, data compression (BWT), and bioinformatics.

The implementation closely follows the original C library by Yuta Mori, including the same sssort / trsort refinement pipeline based on the induced-sorting (IS) algorithm described in:

> Nong, G., Zhang, S., & Chan, W. H. (2009). [Linear Suffix Array Construction by Almost Pure Induced-Sorting](https://www.researchgate.net/publication/221577802_Linear_Suffix_Array_Construction_by_Almost_Pure_Induced-Sorting). *Data Compression Conference (DCC 2009)*, pp. 193–202.

The B\*-bucket sorting step is parallelised with [rayon](https://github.com/rayon-rs/rayon).

> **Note:** This crate uses `unsafe` Rust internally for performance. Specifically, raw pointer aliasing is used to allow the suffix array and its read-only PA view to share the same allocation (mirroring the original C code), and bounds checks are elided in hot inner loops where invariants can be proven statically. The public API is fully safe.

## Usage

```toml
[dependencies]
divsufsort-rs = { path = "." }
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

Benchmarks compare this library against the original **C libdivsufsort** (compiled at `-O3`), run with [criterion](https://github.com/bheisler/criterion.rs) (`sample_size = 10`).

### Environment

| Item | Value |
|---|---|
| CPU | Apple M4 Max (16 logical cores) |
| OS | macOS 15.5 |
| Rust | 1.92.0 |
| Input size | 1,000,000 bytes |

### Corpora

| Name | Description |
|---|---|
| `random_binary` | LCG pseudo-random bytes (alphabet size 256) |
| `text_26` | LCG pseudo-random lowercase ASCII (alphabet size 26) |
| `fibonacci` | Fibonacci string over `{a, b}` (highly repetitive) |

### Results

| Corpus | Rust (this crate) | C libdivsufsort | Ratio |
|---|---|---|---|
| random_binary | 11.3 ms (84.7 MiB/s) | 14.0 ms (68.3 MiB/s) | **1.24× faster** |
| text_26 | 13.2 ms (72.0 MiB/s) | 24.0 ms (39.8 MiB/s) | **1.82× faster** |
| fibonacci | 30.1 ms (31.7 MiB/s) | 27.6 ms (34.6 MiB/s) | 0.97× (comparable) |

The parallel B\*-bucket sort drives the speedup for `random_binary` and `text_26`. For `fibonacci` the input produces only 1–2 non-trivial buckets, so parallelism provides no benefit and the result is comparable to C.

### Running the benchmarks

```sh
# Rust vs C comparison (bench_compare)
# requires the vendored C submodule — initialize it first:
git submodule update --init
cargo bench --bench bench_compare --features c-bench

# Lightweight benchmark — 3 corpora × 2 sizes, completes in ~2–3 minutes (bench_light)
cargo bench --bench bench_light

# Full benchmark — larger sizes and more corpora, takes significantly longer (bench)
cargo bench --bench bench
```
