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
  echo "== test plan =="
  python3 scripts/check-test-plan.py
  echo "== openspec =="
  openspec validate --all --strict 2>&1 | tail -1
  echo "verify-fast: OK (docs-only)"
  exit 0
fi

echo "== fmt =="
cargo fmt --all --check
echo "fmt=ok"

echo "== clippy =="
clippy_errors=$(cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -cE '^error' || true)
echo "clippy_errors=$clippy_errors"
[ "$clippy_errors" -eq 0 ] || { echo "clippy: FAILED"; exit 1; }

echo "== test =="
bash scripts/test-report.sh never

echo "== file-size =="
bash scripts/check-file-size.sh

echo "== worktree hook =="
bash scripts/check-worktree-hook.sh

echo "== selftest budget =="
bash scripts/check-selftest-budget.sh

echo "== icon provenance =="
python3 scripts/check-icon-provenance.py

echo "== guard =="
bash scripts/guard.sh

echo "== test plan =="
python3 scripts/check-test-plan.py --self-test
python3 scripts/check-test-plan.py

echo "== openspec =="
openspec validate --all --strict 2>&1 | tail -1

echo "verify-fast: OK"
