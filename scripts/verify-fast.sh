#!/usr/bin/env bash
#
# verify-fast.sh - fast verification loop for Kooka Pictura.
#
# Format, lint, one test run, the non-goal guard, and spec validation. No Qt
# build and no app self-tests; use verify-full.sh for those.
#
# The test step runs nextest and the shared Qt executor through checks.toml.
# The full gate forwards configure and build so the executor stages the Qt build.
#
# Docs-only fast path: when the working diff (against the merge base with the
# default branch, plus untracked files) touches only docs/, openspec/, *.md, or
# .serena/, run the manifest's docs-only checks. An empty diff falls through
# to the full gate.

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/build-lock.sh
. "$script_dir/build-lock.sh"
build_lock_enter_managed "$script_dir/agent-build.py" bash "$script_dir/verify-fast.sh" "$@"
if [ -z "${PICTURA_CPU_BUDGET_APPLIED:-}" ]; then
    exec python3 "$script_dir/cpu-budget.py" exec -- \
        bash "$script_dir/$(basename "$0")" "$@"
fi
: "${PICTURA_JOBS:?cpu-budget marker set without PICTURA_JOBS; run the gate through scripts/cpu-budget.py}"
cd "$script_dir/.."

# Forward the Qt build options to the shared executor. The full gate asks for a
# configure and a staged build; the plain fast gate does neither.
export PICTURA_VERIFY_QT_CONFIGURE=0 PICTURA_VERIFY_QT_BUILD=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --configure) PICTURA_VERIFY_QT_CONFIGURE=1 ;;
    --build) PICTURA_VERIFY_QT_BUILD=1 ;;
    *)
      echo "verify-fast: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
  shift
done

changed_file=$(mktemp "${TMPDIR:-/tmp}/pictura-changed.XXXXXX")
python3 scripts/changed_files.py --output "$changed_file" >/dev/null

docs_only=0
# A requested build always runs: the docs-only fast path skips cmake.
if [ "$PICTURA_VERIFY_QT_CONFIGURE" = 0 ] && [ "$PICTURA_VERIFY_QT_BUILD" = 0 ] && [ -s "$changed_file" ]; then
  docs_only=1
  while IFS= read -r -d '' path; do
    case "$path" in
      docs/* | openspec/* | *.md | .serena/*) ;;
      *) docs_only=0; break ;;
    esac
  done < "$changed_file"
fi

if [ "$docs_only" -eq 1 ]; then
  echo "verify-fast: docs-only diff; skipping fmt, clippy, and tests"
  python3 scripts/gate_manifest.py run docs-only --changed "$changed_file"
  rm -f "$changed_file"
  echo "verify-fast: OK (docs-only)"
  exit 0
fi

build_lock_install_traps
build_lock_acquire || exit 1
trap 'rm -f "$changed_file"; build_lock_release' EXIT

case "${PICTURA_FULL_GATES:-0}" in
  0) placement=local-fast ;;
  1) placement=local-full ;;
  *) echo "verify-fast: PICTURA_FULL_GATES must be 0 or 1" >&2; exit 2 ;;
esac

echo "== checks (sequential) =="
python3 scripts/gate_manifest.py run "$placement" \
  --phase sequential --changed "$changed_file"

echo "== checks (parallel) =="
python3 scripts/gate_manifest.py run "$placement" \
  --phase parallel --parallel "$PICTURA_JOBS" --changed "$changed_file"

echo "verify-fast: OK"
