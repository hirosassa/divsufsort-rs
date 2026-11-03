#!/usr/bin/env bats

setup() {
  SCRIPT="$BATS_TEST_DIRNAME/../extract-changelog.sh"
  WORK="$(mktemp -d)"
}

teardown() {
  rm -rf "$WORK"
}

@test "extracts only the requested version section" {
  cat > "$WORK/CHANGELOG.md" <<'MD'
# Changelog

## [Unreleased]

## [0.2.0] - 2026-01-02

### Fixed
- b

## [0.1.0] - 2026-01-01

### Added
- a
MD
  run "$SCRIPT" 0.2.0 "$WORK/CHANGELOG.md"
  [ "$status" -eq 0 ]
  [ "$output" = "### Fixed
- b" ]
}

@test "accepts a leading v in the version" {
  printf '## [0.1.0] - 2026-01-01\n\n### Added\n- a\n' > "$WORK/CHANGELOG.md"
  run "$SCRIPT" v0.1.0 "$WORK/CHANGELOG.md"
  [ "$status" -eq 0 ]
  [ "$output" = "### Added
- a" ]
}

@test "does not match a version that is only a prefix" {
  printf '## [0.10.0] - 2026-01-01\n\n- x\n' > "$WORK/CHANGELOG.md"
  run "$SCRIPT" 0.1.0 "$WORK/CHANGELOG.md"
  [ "$status" -ne 0 ]
}

@test "fails when the version is missing" {
  printf '## [0.1.0] - 2026-01-01\n\n- a\n' > "$WORK/CHANGELOG.md"
  run "$SCRIPT" 9.9.9 "$WORK/CHANGELOG.md"
  [ "$status" -ne 0 ]
}

@test "fails when the section is empty" {
  printf '## [0.2.0] - 2026-01-02\n\n## [0.1.0] - 2026-01-01\n\n- a\n' > "$WORK/CHANGELOG.md"
  run "$SCRIPT" 0.2.0 "$WORK/CHANGELOG.md"
  [ "$status" -ne 0 ]
}

@test "stops at the link reference definitions at the end of the file" {
  printf '## [0.1.0] - 2026-01-01\n\n- a\n\n[0.1.0]: https://example.com\n' > "$WORK/CHANGELOG.md"
  run "$SCRIPT" 0.1.0 "$WORK/CHANGELOG.md"
  [ "$status" -eq 0 ]
  [ "$output" = "- a" ]
}
