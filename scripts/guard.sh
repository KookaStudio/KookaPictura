#!/usr/bin/env bash
#
# guard.sh - non-goal artifact guard for Kooka Pictura (task M0-C).
#
# Fails when tracked code drifts into a declared non-goal:
#   1. crates/ source referencing Adobe .8bf plugin binaries (wrong ABI, non-goal).
#   2. an artboard* file appearing under crates/ (artboards are post-CS6 CC 2015).
#   3. a milestone name (m<NN>/M<NN>) in a crates/ identifier or string literal
#      (milestones belong in comments, docs, and specs only).
#   4. docs/ modified without a TASK-ALLOWS-DOCS marker.
#   5. an encumbered Adobe preset/binary asset (.abr/.pat/.grd/.asl/...).
#   6. an Adobe-tool creator string inside a tracked binary asset.
#   7. a tracked reference-fixtures/ file, or tracked code depending on it.
#   8. a missing required legal artifact (LICENSE, notices, deny.toml, CONTRIBUTING).
#   9. a release-please version marker dropped from a version file.
#
# Runnable locally with no setup:   bash scripts/guard.sh
#
# Overrides:
#   GUARD_BASE=<git-ref>     diff docs/ against this ref (CI/PR use). Without it,
#                            only uncommitted docs/ changes are checked, so the
#                            guard is green on a clean checkout of already-reviewed history.
#   TASK_ALLOWS_DOCS=1       allow docs/ changes without the commit-message marker

set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
echo "guard: repo $(pwd)"

# --- 1. .8bf plugin-binary references in tracked crates/ sources ---------------
echo "check: crates/ references .8bf plugin binaries"
hits=$(git grep -n -i -e '8bf' -- 'crates/*' || true)
if [ -n "$hits" ]; then
    echo "FAIL: .8bf reference under crates/ (Adobe plugin ABI is a non-goal):"
    echo "$hits"
    fail=1
fi

# --- 2. artboard files under crates/ ------------------------------------------
echo "check: no artboard* files under crates/"
artboards=$(find crates \( -name target -o -name .git \) -prune -o -iname 'artboard*' -print 2>/dev/null)
if [ -n "$artboards" ]; then
    echo "FAIL: artboard artifact under crates/ (post-CS6 feature):"
    echo "$artboards"
    fail=1
fi

# --- 2b. encumbered Adobe preset/binary assets --------------------------------
echo "check: no encumbered Adobe preset/binary assets"
encumbered=$(git ls-files | grep -Ei '\.(abr|pat|grd|asl|acv|atn|csh|8bf|aco|acb)$' || true)
if [ -n "$encumbered" ]; then
    echo "FAIL: encumbered Adobe preset/binary asset tracked (do not bundle Adobe assets):"
    echo "$encumbered"
    fail=1
fi

# --- 2c. Adobe-tool creator strings inside tracked binary assets --------------
echo "check: no Adobe-tool strings in tracked binary assets"
adtool=$(git grep -a -l -e 'Adobe Photoshop' -e 'Adobe Illustrator' -e 'xmp:CreatorTool' \
    -- '*.psd' '*.psb' '*.bin' '*.icc' '*.icm' '*.ttf' '*.otf' '*.jpg' '*.jpeg' '*.png' '*.tif' '*.tiff' || true)
if [ -n "$adtool" ]; then
    echo "FAIL: asset carries an Adobe-tool creator string (generate a synthetic asset instead):"
    echo "$adtool"
    fail=1
fi

# --- 2d. reference-fixtures stay untracked and unreferenced -------------------
echo "check: reference-fixtures/ stay untracked"
tracked_ref=$(git ls-files | grep '^reference-fixtures/' || true)
if [ -n "$tracked_ref" ]; then
    echo "FAIL: reference-fixtures/ is tracked (must stay gitignored, never distributed):"
    echo "$tracked_ref"
    fail=1
fi
# check-worktree-hook.sh seeds its own throwaway copy; it does not depend on
# the local reference-fixtures/ being present (design D7 of auto-wire-worktrees).
ref_use=$(git grep -n -e 'reference-fixtures/' -- 'crates/*' 'scripts/*' ':!scripts/guard.sh' ':!scripts/check-worktree-hook.sh' || true)
if [ -n "$ref_use" ]; then
    echo "FAIL: tracked code depends on reference-fixtures/ (absent in clones/CI):"
    echo "$ref_use"
    fail=1
fi

# --- 2e. required legal artifacts present -------------------------------------
echo "check: required legal artifacts present"
for f in LICENSE THIRD-PARTY-LICENSES LICENSES/GPL-3.0-or-later.txt deny.toml CONTRIBUTING.md; do
    if [ ! -f "$f" ]; then
        echo "FAIL: required legal artifact missing: $f"
        fail=1
    fi
done

# --- 3. milestone names in crates/ identifiers or strings ---------------------
echo "check: no milestone names in crates/ code"
if ! python3 "$(dirname "$0")/check-milestone-names.py"; then
    fail=1
fi

# --- 4. docs/ modified without a TASK-ALLOWS-DOCS marker ----------------------
echo "check: docs/ changes carry a TASK-ALLOWS-DOCS marker"
base="${GUARD_BASE:-}"

allow=0
if [ -n "${TASK_ALLOWS_DOCS:-}" ]; then
    allow=1
fi
if [ -n "$base" ]; then
    msgs=$(git log "$base"..HEAD --format='%B' 2>/dev/null || true)
    case "$msgs" in
        *TASK-ALLOWS-DOCS*) allow=1 ;;
    esac
fi

committed=""
if [ -n "$base" ]; then
    committed=$(git diff --name-only "$base"..HEAD -- docs/ || true)
fi
uncommitted=$(git status --porcelain -- docs/ || true)

if [ -n "$committed$uncommitted" ] && [ "$allow" -ne 1 ]; then
    echo "FAIL: docs/ modified without a TASK-ALLOWS-DOCS marker:"
    if [ -n "$committed" ]; then echo "$committed"; fi
    if [ -n "$uncommitted" ]; then echo "$uncommitted"; fi
    fail=1
fi
if [ -z "$base" ]; then
    echo "note: GUARD_BASE unset; checked uncommitted docs/ changes only"
fi

# --- 5. release-please version markers present --------------------------------
# The Generic updater only rewrites annotated lines, so a dropped marker silently
# stops that file's version bump (release process in DEVELOPING.md).
echo "check: release-please version markers present"
for f in Cargo.toml CMakeLists.txt; do
    if ! grep -q 'x-release-please-version' "$f"; then
        echo "FAIL: $f is missing its x-release-please-version marker"
        fail=1
    fi
done

if [ "$fail" -ne 0 ]; then
    echo "guard: FAILED"
    exit 1
fi
echo "guard: OK"
