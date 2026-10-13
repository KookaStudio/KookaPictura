#!/usr/bin/env bash
#
# check-worktree-hook.sh - prove .githooks/post-checkout end to end.
#
# Builds a throwaway git repo in mktemp, commits the real hook from this
# checkout, creates a linked worktree, and asserts the wiring the
# meta/worktree-hooks spec requires: both symlinks, idempotent second
# checkout, and the missing-fixtures skip path. Never touches the real
# checkout; removes the sandbox on exit. Exits non-zero on any failure.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
hook_src="$repo_root/.githooks/post-checkout"
if [ ! -f "$hook_src" ]; then
    echo "check-worktree-hook: missing $hook_src" >&2
    exit 1
fi

sandbox=$(mktemp -d "${TMPDIR:-/tmp}/pictura-hook-check.XXXXXX") || exit 1
cleanup() { rm -rf "$sandbox"; }
trap cleanup EXIT

fail() { echo "check-worktree-hook: FAIL: $*" >&2; exit 1; }

main="$sandbox/main"
wt="$sandbox/worktree"
mkdir -p "$main"

git -C "$main" init -q
git -C "$main" config user.name "Hook Check"
git -C "$main" config user.email "hook-check@localhost"
git -C "$main" config core.hooksPath .githooks
mkdir -p "$main/.githooks" "$main/reference-fixtures"
cp "$hook_src" "$main/.githooks/post-checkout"
chmod 755 "$main/.githooks/post-checkout"
echo seed >"$main/reference-fixtures/marker"
# Use the repo's real .gitignore so the /reference-fixtures pattern itself is
# under test (a regression to the directory-only form breaks the remove below).
cp "$repo_root/.gitignore" "$main/.gitignore"
git -C "$main" add -A
git -C "$main" commit -qm "seed"

git -C "$main" worktree add -q -b scratch "$wt" || fail "git worktree add"

[ -L "$wt/target" ] || fail "target is not a symlink"
[ "$(readlink "$wt/target")" = "../.target" ] || fail "target does not point at ../.target"
[ -d "$wt/target" ] || fail "target symlink does not resolve to a directory"
[ -L "$wt/reference-fixtures" ] || fail "reference-fixtures is not a symlink"
[ -e "$wt/reference-fixtures/marker" ] || fail "reference-fixtures symlink does not resolve"

before=$(ls -A "$wt" | sort)
git -C "$wt" checkout -q HEAD || fail "second checkout errored"
after=$(ls -A "$wt" | sort)
[ "$before" = "$after" ] || fail "second checkout changed the worktree (not idempotent)"

# A checkout in the main tree must be left untouched (no symlinks appear).
git -C "$main" checkout -q HEAD || fail "main checkout errored"
[ ! -e "$main/target" ] && [ ! -L "$main/target" ] \
    || fail "main checkout gained a target link"
[ ! -L "$main/reference-fixtures" ] || fail "main checkout fixtures link replaced the directory"

git -C "$main" worktree remove "$wt" || fail "worktree remove without --force"
rm -rf "$main/reference-fixtures" || fail "remove fixtures seed"
git -C "$main" worktree add -q -b scratch2 "$wt" || fail "second git worktree add"
[ -L "$wt/target" ] || fail "target missing when fixtures absent"
[ ! -e "$wt/reference-fixtures" ] && [ ! -L "$wt/reference-fixtures" ] \
    || fail "fixtures link created though source is absent"
git -C "$main" worktree remove "$wt" || fail "second worktree remove"

echo "check-worktree-hook: OK"
