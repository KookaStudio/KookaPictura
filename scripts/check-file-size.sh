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
# Usage: check-file-size.sh [--changed FILE] [--full] [--self-test] [--root DIR]
#   --changed FILE  scan only the crate sources named in FILE (the NUL-separated
#                   list from scripts/changed_files.py); an empty list falls back
#                   to the whole tree, so an unobtainable change set cannot
#                   silence it
#   --full          scan the whole tree (the default, and what CI uses)
#   --self-test     build a sandbox and assert the scoped and full outcomes
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

check_tree() { # <root> <changed-list or empty>
  local root="$1" changed_list="$2"
  local allowlist="$root/scripts/file-size-allowlist.txt"
  # The allowlist exempts files by path, so a change to it can affect a file the
  # changed set does not name: scan the whole tree then.
  if [ -n "$changed_list" ] && tr '\0' '\n' < "$changed_list" |
      grep -qx 'scripts/file-size-allowlist.txt'; then
    changed_list=""
  fi
  local -A allowed=()
  if [ -f "$allowlist" ]; then
    while read -r path loc _; do
      case "$path" in ''|\#*) continue ;; esac
      [ -n "${loc:-}" ] || continue
      allowed["$path"]="$loc"
    done < "$allowlist"
  fi

  if [ -n "$changed_list" ] && [ -s "$changed_list" ] &&
     [ "$(tr -cd '\0' < "$changed_list" | wc -c)" -eq 0 ]; then
    # A non-empty list with no NUL record is a hand-written newline list this
    # reader cannot split safely, so treat it as unreadable rather than silently
    # scanning nothing.
    echo "check-file-size: changed list has no NUL-separated entry: $changed_list" >&2
    return 2
  fi

  local paths
  paths=$(mktemp "${TMPDIR:-/tmp}/pictura-file-size-paths.XXXXXX")
  # shellcheck disable=SC2064
  trap "rm -f '$paths'" RETURN
  if [ -n "$changed_list" ] && [ -s "$changed_list" ]; then
    # The helper writes a NUL-separated list, so a name with a newline survives.
    while IFS= read -r -d '' rel; do
      case "$rel" in
        crates/*.rs | crates/*.cpp | crates/*.h) ;;
        *) continue ;;
      esac
      [ -f "$root/$rel" ] || continue
      printf '%s\0' "$rel"
    done < "$changed_list" > "$paths"
  else
    ( cd "$root" && find crates -type f \( -name '*.rs' -o -name '*.cpp' -o -name '*.h' \) -print0 ) > "$paths"
  fi

  local counts
  counts=$(mktemp "${TMPDIR:-/tmp}/pictura-file-size-counts.XXXXXX")
  # shellcheck disable=SC2064
  trap "rm -f '$paths' '$counts'" RETURN
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
  printf 'small\n' > "$sandbox/crates/with space.rs"
  awk 'BEGIN { for (i = 0; i < 1250; i++) print "x" }' > "$sandbox/crates/big.rs"
  awk 'BEGIN { for (i = 0; i < 1250; i++) print "x" }' > "$sandbox/crates/tests/growing.rs"
  printf 'crates/big.rs 1200\ncrates/tests/growing.rs 1300\n' > "$sandbox/scripts/file-size-allowlist.txt"

  # The helper writes a NUL-separated list, so a name with a space must survive.
  printf 'crates/small.rs\0crates/with space.rs\0' > "$sandbox/small.txt"
  printf 'crates/small.rs\0crates/big.rs\0' > "$sandbox/both.txt"
  : > "$sandbox/empty.txt"

  # The full scan sees the allowlist growth.
  if check_tree "$sandbox" "" >/dev/null; then
    echo "check-file-size: self-test FAILED (the full scan missed an over-budget file)" >&2
    rc=1
  fi
  # A scoped scan that excludes the violation passes...
  if ! check_tree "$sandbox" "$sandbox/small.txt" >/dev/null; then
    echo "check-file-size: self-test FAILED (the scoped scan flagged an unchanged file)" >&2
    rc=1
  fi
  # ...but one that includes it fails.
  if check_tree "$sandbox" "$sandbox/both.txt" >/dev/null; then
    echo "check-file-size: self-test FAILED (the scoped scan missed a changed violation)" >&2
    rc=1
  fi
  # An empty changed set falls back to the full scan instead of passing.
  if check_tree "$sandbox" "$sandbox/empty.txt" >/dev/null; then
    echo "check-file-size: self-test FAILED (an empty changed list skipped the scan)" >&2
    rc=1
  fi
  # A file wc cannot count (here: a name holding a newline) must fail, not pass.
  mkdir -p "$sandbox/crates/odd"
  awk 'BEGIN { for (i = 0; i < 1250; i++) print "x" }' > "$sandbox/crates/odd/we
ird.rs"
  printf 'crates/odd/we\nird.rs\0' > "$sandbox/odd.txt"
  if check_tree "$sandbox" "$sandbox/odd.txt" >/dev/null 2>&1; then
    echo "check-file-size: self-test FAILED (a name wc could not count passed)" >&2
    rc=1
  fi
  # A non-empty list with no NUL record is a usage error, not a silent pass.
  printf 'crates/big.rs\n' > "$sandbox/newline.txt"
  if check_tree "$sandbox" "$sandbox/newline.txt" >/dev/null 2>&1; then
    echo "check-file-size: self-test FAILED (a newline-only list passed silently)" >&2
    rc=1
  fi
  # A change to the allowlist widens the scan to the whole tree, so the
  # unallowlisted violation is still found.
  printf 'scripts/file-size-allowlist.txt\0' > "$sandbox/allowlist.txt"
  if check_tree "$sandbox" "$sandbox/allowlist.txt" >/dev/null; then
    echo "check-file-size: self-test FAILED (an allowlist change did not widen the scan)" >&2
    rc=1
  fi
  # The test budget is the higher 1400: 1250 lines under crates/tests is fine.
  printf 'crates/tests/growing.rs\0' > "$sandbox/test_only.txt"
  if ! check_tree "$sandbox" "$sandbox/test_only.txt" >/dev/null; then
    echo "check-file-size: self-test FAILED (the test budget was not applied)" >&2
    rc=1
  fi
  [ "$rc" -eq 0 ] && echo "check-file-size: self-test OK"
  return "$rc"
}

root="$repo_root"
changed_list=""
while [ $# -gt 0 ]; do
  case "$1" in
    --changed)
      [ $# -ge 2 ] || { echo "check-file-size: --changed needs a file" >&2; exit 2; }
      changed_list="$2"
      shift 2
      ;;
    --full)
      changed_list=""
      shift
      ;;
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

if [ -n "$changed_list" ] && [ ! -f "$changed_list" ]; then
  echo "check-file-size: cannot read the changed list: $changed_list" >&2
  exit 2
fi

check_tree "$root" "$changed_list"
