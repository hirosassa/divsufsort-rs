# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
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
