#!/usr/bin/env bash
#
# verify-fast.sh - fast verification loop for Kooka Pictura.
#
# Format, lint, one test run, the non-goal guard, and spec validation. No Qt
# build and no app self-tests; use verify-full.sh for those.
#
# The test step routes through scripts/test-report.sh (nextest + doctests +
# the app self-tests when ./build/pictura exists), which prints the unified
# report. One nextest run per gate.
#
# Docs-only fast path: when the working diff (against the merge base with the
# default branch, plus untracked files) touches only docs/, openspec/, *.md, or
# .serena/, run just the guard and spec validation. An empty diff falls through
# to the full gate.

set -euo pipefail
cd "$(dirname "$0")/.."

# shellcheck source=scripts/build-lock.sh
. scripts/build-lock.sh

base=""
for ref in master main kooka/master kooka/main origin/master origin/main; do
  if git rev-parse --verify -q "$ref" >/dev/null 2>&1; then
    base=$(git merge-base HEAD "$ref" 2>/dev/null || true)
    [ -n "$base" ] && break
  fi
done

changed=$(
  {
    [ -n "$base" ] && git diff --name-only "$base" -- || true
    git status --porcelain --untracked-files=all | sed 's/^...//'
  } | sort -u
)

docs_only=0
if [ -n "$changed" ]; then
  docs_only=1
  while IFS= read -r path; do
    case "$path" in
      docs/* | openspec/* | *.md | .serena/*) ;;
      *) docs_only=0; break ;;
    esac
  done <<< "$changed"
fi

if [ "$docs_only" -eq 1 ]; then
  echo "verify-fast: docs-only diff; skipping fmt, clippy, and tests"
  echo "== guard =="
  bash scripts/guard.sh
  echo "== archive =="
  bash scripts/check-openspec-archived.sh
  echo "== test plan =="
  python3 scripts/check-test-plan.py
  echo "== openspec =="
  openspec validate --all --strict 2>&1 | tail -1
  echo "verify-fast: OK (docs-only)"
  exit 0
fi

build_lock_install_traps
build_lock_acquire || exit 1

echo "== fmt =="
cargo fmt --all --check
echo "fmt=ok"

echo "== clippy =="
clippy_errors=$(cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -cE '^error' || true)
echo "clippy_errors=$clippy_errors"
[ "$clippy_errors" -eq 0 ] || { echo "clippy: FAILED"; exit 1; }

echo "== build tree =="
# The gate runs the tests against ./build, so a tree that predates the sources
# would test something else. The full gate fails on this; the fast gate only
# reports it, because it does not own the build tree.
bash scripts/check-build-tree.sh --warn

echo "== test =="
bash scripts/test-report.sh never

checks_dir="$(mktemp -d "${TMPDIR:-/tmp}/pictura-verify.XXXXXX")"
trap 'rm -rf "$checks_dir"; build_lock_release' EXIT

check_names=()
check_pids=()

start_check() { # <name> <command...>
    local name="$1"; shift
    check_names+=("$name")
    ( set +e; "$@" >"$checks_dir/$name.log" 2>&1; printf '%s' "$?" >"$checks_dir/$name.rc" ) &
    check_pids+=("$!")
}

flatpak_check() {
    python3 scripts/check-flatpak-cargo-sources.py --self-test &&
        python3 scripts/check-flatpak-cargo-sources.py
}

archive_check() {
    bash scripts/check-openspec-archived.sh --self-test &&
        bash scripts/check-openspec-archived.sh
}

test_plan_check() {
    python3 scripts/check-test-plan.py --self-test &&
        python3 scripts/check-test-plan.py
}

openspec_check() {
    openspec validate --all --strict
}

echo "== checks (parallel) =="
start_check file-size bash scripts/check-file-size.sh
start_check flatpak flatpak_check
start_check worktree-hook bash scripts/check-worktree-hook.sh
start_check build-caching bash scripts/check-local-build-caching.sh
start_check build-lock bash scripts/check-build-lock.sh
start_check selftest-budget bash scripts/check-selftest-budget.sh
start_check archive archive_check
start_check icon-provenance python3 scripts/check-icon-provenance.py
start_check guard bash scripts/guard.sh
start_check test-plan test_plan_check
start_check openspec openspec_check

check_failed=0
for pid in "${check_pids[@]}"; do
    wait "$pid" || true
done
for name in "${check_names[@]}"; do
    rc=$(cat "$checks_dir/$name.rc" 2>/dev/null || printf 1)
    if [ "$rc" -eq 0 ]; then
        printf '%s=ok\n' "$name"
    else
        printf '%s=FAILED (rc=%s)\n' "$name" "$rc"
        sed 's/^/  /' "$checks_dir/$name.log" || true
        check_failed=1
    fi
done
[ "$check_failed" -eq 0 ] || { echo "verify-fast: FAILED"; exit 1; }

echo "verify-fast: OK"
