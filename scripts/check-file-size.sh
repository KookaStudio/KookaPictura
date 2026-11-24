#!/usr/bin/env bash
#
# check-file-size.sh - enforce the LOC hard cap on crate sources.
#
# Code cap 1200, test cap 1400 (target <800 for both, AGENTS.md rule 9). Tests
# are detected by path: Rust under tests/ or named tests.rs, C++ *_test.{cpp,h}.
# Files over the cap must be listed in scripts/file-size-allowlist.txt with
# their current line count. An allowlist entry is a ceiling, not a licence: it
# may only shrink as the file is split.
#
# Usage: check-file-size.sh [--self-test] [--root DIR]
#   --self-test     build a sandbox and assert the pass and fail outcomes
#   --root DIR      scan DIR instead of the repository (used by --self-test)
#
# Line counts come from one batched `wc -l` and one awk pass, not one fork per
# file.

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
repo_root=$(cd "$script_dir/.." && pwd)

CODE_BUDGET=1200
TEST_BUDGET=1400

# Count every file in <paths-file> with one `wc -l`, and print
# "lines<TAB>path<TAB>budget" for the over-budget ones. The full wc output is
# kept in <counts> so the record count can be checked without a second run: wc
# quotes a name it cannot print as a plain word (one holding a newline) and skips
# an unreadable file, so a short batch must not silently drop a file from the
# check.
count_batch() { # <root> <paths-file> <counts-file>
  local root="$1" paths="$2" counts="$3"
  ( cd "$root" && xargs -0 -r wc -l < "$paths" ) > "$counts" 2>/dev/null || true
  awk -v code="$CODE_BUDGET" -v test="$TEST_BUDGET" '
    {
      lines = $1
      path = $0
      sub(/^[ \t]*/, "", path)
      sub(/^[0-9]+[ \t]+/, "", path)
      if (path == "total") next
      budget = (path ~ /\/tests\// || path ~ /\/tests\.rs$/ ||
                path ~ /_test\.(cpp|h|rs)$/) ? test : code
      if (lines > budget) print lines "\t" path "\t" budget
    }
  ' < "$counts"
}

check_tree() { # <root>
  local root="$1"
  local allowlist="$root/scripts/file-size-allowlist.txt"
  local -A allowed=()
  if [ -f "$allowlist" ]; then
    while read -r path loc _; do
      case "$path" in ''|\#*) continue ;; esac
      [ -n "${loc:-}" ] || continue
      allowed["$path"]="$loc"
    done < "$allowlist"
  fi

  local paths counts
  paths=$(mktemp "${TMPDIR:-/tmp}/pictura-file-size-paths.XXXXXX")
  counts=$(mktemp "${TMPDIR:-/tmp}/pictura-file-size-counts.XXXXXX")
  # shellcheck disable=SC2064
  trap "rm -f '$paths' '$counts'" RETURN
  ( cd "$root" && find crates -type f \( -name '*.rs' -o -name '*.cpp' -o -name '*.h' \) -print0 ) > "$paths"

  local over_budget
  over_budget=$(count_batch "$root" "$paths" "$counts")
  # wc adds a `total` line only for more than one file.
  local wanted counted
  wanted=$(tr -cd '\0' < "$paths" | wc -c)
  counted=$(awk '$2 != "total" { n++ } END { print n + 0 }' < "$counts")
  if [ "$counted" -ne "$wanted" ]; then
    echo "check-file-size: FAILED (wc counted $counted of $wanted files)" >&2
    return 2
  fi

  local violations=0 over=0 lines file budget
  while IFS=$'\t' read -r lines file budget; do
    [ -n "$file" ] || continue
    over=$((over + 1))
    if [ -z "${allowed[$file]:-}" ]; then
      echo "$file: $lines lines (budget $budget)"
      violations=$((violations + 1))
    elif [ "$lines" -gt "${allowed[$file]}" ]; then
      echo "$file: grew to $lines (allowed ${allowed[$file]})"
      violations=$((violations + 1))
    fi
  done <<< "$over_budget"

  if [ "$violations" -gt 0 ]; then
    echo "check-file-size: FAILED ($violations violation(s), $over file(s) over budget)"
    return 1
  fi
  echo "check-file-size: OK ($over files over budget, all allowlisted)"
  return 0
}

self_test() {
  local sandbox rc=0
  sandbox=$(mktemp -d "${TMPDIR:-/tmp}/pictura-file-size-check.XXXXXX") || exit 1
  # shellcheck disable=SC2064
  trap "rm -rf '$sandbox'" EXIT

  mkdir -p "$sandbox/crates/tests" "$sandbox/scripts"
  printf 'small\n' > "$sandbox/crates/small.rs"
  awk 'BEGIN { for (i = 0; i < 1250; i++) print "x" }' > "$sandbox/crates/tests/growing.rs"
  printf 'crates/tests/growing.rs 1300\ncrates/big.rs 1200\n' > "$sandbox/scripts/file-size-allowlist.txt"
  : > "$sandbox/crates/big.rs"

  # An over-budget file with no allowlist entry fails.
  awk 'BEGIN { for (i = 0; i < 1250; i++) print "x" }' > "$sandbox/crates/extra.rs"
  if check_tree "$sandbox" >/dev/null; then
    echo "check-file-size: self-test FAILED (an over-budget file passed)" >&2
    rc=1
  fi
  # A file at its allowlisted ceiling passes; one past it fails.
  awk 'BEGIN { for (i = 0; i < 1250; i++) print "x" }' > "$sandbox/crates/big.rs"
  if check_tree "$sandbox" >/dev/null; then
    echo "check-file-size: self-test FAILED (a grown allowlisted file passed)" >&2
    rc=1
  fi
  : > "$sandbox/crates/extra.rs"
  awk 'BEGIN { for (i = 0; i < 1200; i++) print "x" }' > "$sandbox/crates/big.rs"
  if ! check_tree "$sandbox" >/dev/null; then
    echo "check-file-size: self-test FAILED (an allowlisted ceiling was rejected)" >&2
    rc=1
  fi
  # 1250 lines under crates/tests is under the 1400 test budget.
  if ! check_tree "$sandbox" >/dev/null; then
    echo "check-file-size: self-test FAILED (the test budget was not applied)" >&2
    rc=1
  fi
  [ "$rc" -eq 0 ] && echo "check-file-size: self-test OK"
  return "$rc"
}

root="$repo_root"
while [ $# -gt 0 ]; do
  case "$1" in
    --root)
      [ $# -ge 2 ] || { echo "check-file-size: --root needs a directory" >&2; exit 2; }
      root="$2"
      shift 2
      ;;
    --self-test)
      self_test
      exit $?
      ;;
    *)
      echo "check-file-size: unknown option: $1" >&2
      exit 2
      ;;
  esac
done

check_tree "$root"
