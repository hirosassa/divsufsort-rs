# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Suffix array entries can now be `i64`: the public API is generic over the new sealed `SaIndex` trait (implemented for `i32` and `i64`), lifting the input length limit from `i32::MAX` to `i64::MAX`. `i64` needs 9n bytes (`i32`: 5n); see the README
- `bench_light` has an `i64` group; tests run for both index types, with `i32`/`i64` equivalence tests and `#[ignore]`d multi-gigabyte tests (`tests/large_input.rs`)

### Changed
- **Breaking:** `divsufsort`, `divbwt`, `sufcheck`, `sa_search`, `sa_simplesearch`, `bw_transform` and `inverse_bw_transform` are generic over `I: SaIndex`. `divsufsort(t, &mut sa)` with an `i32` slice infers as before, but `divbwt(t, u, None)` no longer infers `I`: write `divbwt::<i32>(t, u, None)`. The primary index returned by `divbwt` and used by `bw_transform`/`inverse_bw_transform`, and the `(count, left)` returned by the search functions, now have type `I`
- `divbwt` allocates its scratch buffer with `try_reserve_exact` and returns `AllocationFailure` instead of aborting; the variant was previously unused
- The `i32` code path is monomorphised and executes the same instructions as before (measured within ±2% wall time, see the PR)

### Fixed
- `divsufsort` and `divbwt` now return `InvalidArgument` for inputs longer than the index type's maximum (`i32::MAX` for `i32`) instead of silently truncating lengths internally
- `inverse_bw_transform` now returns `InvalidArgument` for inputs longer than `I::MAX`
- `bw_transform` now returns `InvalidArgument` instead of panicking when a provided suffix array has an out-of-range entry or does not contain exactly one `0`
- `inverse_bw_transform` now returns `InvalidArgument` when the output buffer `u` is shorter than the input, instead of silently truncating the result (or panicking for a one-byte input)

## [0.7.0] - 2026-11-03

### Changed
- **Breaking:** the `std` feature no longer enables `rayon`; with default features disabled, enable `rayon` explicitly. `rayon` is now a separate feature (enabled by default); the `cc` build dependency is gated behind the `c-bench` feature (#18)
- The `no_std` setup now follows the Cargo book convention (`std` feature) (#18)
- Sort directly in the caller's buffer and move B* suffixes in place, roughly halving peak memory (#24)
- Use fixed-size bucket tables to drop bounds checks (#23)
- Update GitHub Actions, add an actions lockfile and group Dependabot updates (#19)

### Fixed
- `divbwt` and `bw_transform` now return `InvalidArgument` instead of panicking when the output buffer `u` (or the provided suffix array in `bw_transform`) is shorter than the input
- Panic in `ss_swapmerge` on periodic inputs (#21)
- Wrong suffix array on periodic inputs caused by `tr_copy`/`tr_partialcopy` filling the right partition from the wrong position (#22)
- Parallel `sssort` aliased `&mut` references to the suffix array; it now splits the array into disjoint chunks (#25)
- Match the C implementation in the `ss_inplacemerge` flagged-group skip and the `sssort` buffer borrow (#26)
- Clippy errors with newer toolchains (#20)

### Added
- Tests pinning SA/BWT output checksums to guard performance refactorings (#23)
- Small-`n` and repetitive-corpus benchmarks, `divbwt` benchmark (#23)

## [0.6.0] - 2026-03-29

### Changed
- Version bump only; no functional changes since 0.5.0

## [0.5.0] - 2026-03-29

### Added
- `no_std` support (#17)

### Changed
- Rewrite safety comments in the idiomatic `// SAFETY:` style (#16)

## [0.4.0] - 2026-03-24

### Changed
- Readability refactorings with no intended performance change: share the constant tables, split long functions, extract the stack and the skip-negatives logic, fix bit-flag handling, introduce type aliases, and improve docs (#8–#15)

## [0.3.0] - 2026-03-23

### Fixed
- Documentation and reference fixes (#6, #7)

## [0.2.0] - 2026-03-23

### Added
- MIT license (#5)
- Dependabot configuration for Cargo updates (#3, #4)

### Fixed
- Release workflow fixes (#2)
- Reduce function complexity (#1)

[Unreleased]: https://github.com/hirosassa/divsufsort-rs/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/hirosassa/divsufsort-rs/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/hirosassa/divsufsort-rs/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/hirosassa/divsufsort-rs/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/hirosassa/divsufsort-rs/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/hirosassa/divsufsort-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/hirosassa/divsufsort-rs/releases/tag/v0.2.0
