#!/usr/bin/env bash
# Print the body of one version's section from a Keep a Changelog file.
# Usage: extract-changelog.sh <version> [changelog-path]
set -euo pipefail

version="${1:?usage: extract-changelog.sh <version> [changelog-path]}"
version="${version#v}"
file="${2:-CHANGELOG.md}"

body="$(
  awk -v ver="$version" '
    /^## \[/ {
      if (found) exit
      if (index($0, "## [" ver "]") == 1) found = 1
      next
    }
    /^\[[^]]+\]: / { if (found) exit }
    found { print }
  ' "$file"
)"

# Trim leading/trailing blank lines.
body="$(printf '%s\n' "$body" | sed -e '/./,$!d' | sed -e ':a' -e '/^\n*$/{$d;N;ba' -e '}')"

if [ -z "$body" ]; then
  echo "error: no non-empty section for version $version in $file" >&2
  exit 1
fi

printf '%s\n' "$body"
