#!/usr/bin/env bash
#
# check-local-build-caching.sh - prove the CMake default wires sccache.
#
# Default mode: configure the project into throwaway build directories and
# assert the Ninja LAUNCHER build-rule variable (sccache by default, an explicit
# override wins, none when sccache is absent). --rebuild mode (with optional
# --dir <build-dir>): build the pictura target, delete one of its shell object
# files, rebuild, and assert the second build hits the cache. The three
# throwaway configures share a user-level FetchContent base so cxx-qt-cmake and
# Corrosion are fetched once per run and reused later; only the build directory
# named by --dir is ever built. Never touches the real build/ unless --dir names
# it, and restores the repo-root compile_commands.json symlink. Self-skips when
# cmake or qmake6 is absent.

set -euo pipefail

rebuild=0
rebuild_dir=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --rebuild) rebuild=1 ;;
        --dir) shift; rebuild_dir="${1:-}" ;;
        *) echo "check-local-build-caching: unknown argument '$1'" >&2; exit 2 ;;
    esac
    shift
done

if ! command -v cmake >/dev/null 2>&1 || ! command -v qmake6 >/dev/null 2>&1; then
    echo "check-local-build-caching: SKIP (needs cmake and qmake6)"
    exit 0
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

sandbox=$(mktemp -d "${TMPDIR:-/tmp}/pictura-sccache-check.XXXXXX")

# CMakeLists.txt repoints the repo-root compile_commands.json at each configured
# build directory, so preserve the original (symlink or file) and restore it.
compile_db="$repo_root/compile_commands.json"
restore_compile_db() {
    rm -f "$compile_db"
    if [ -e "$sandbox/compile_db.bak" ] || [ -L "$sandbox/compile_db.bak" ]; then
        cp -a "$sandbox/compile_db.bak" "$compile_db"
    fi
    return 0
}
if [ -e "$compile_db" ] || [ -L "$compile_db" ]; then
    cp -a "$compile_db" "$sandbox/compile_db.bak"
fi

# One user-level FetchContent base per machine. Worktrees share it, so the
# dependency sources are cloned once and reused; the check itself only builds in
# the directory it was asked to inspect. A second concurrent run falls back to
# its own sandbox base instead of writing the shared one.
fetchcontent_cache="${XDG_CACHE_HOME:-${HOME:-/tmp}/.cache}/kooka-pictura/fetchcontent"
fetchcontent_lock="$fetchcontent_cache.lock"
fetchcontent_base="$fetchcontent_cache"
lock_owned=0
mkdir -p "$(dirname "$fetchcontent_cache")"

cleanup() {
    if [ "$lock_owned" -eq 1 ]; then
        rm -rf "$fetchcontent_lock"
    fi
    restore_compile_db
    rm -rf "$sandbox"
}
trap cleanup EXIT

fail() { echo "check-local-build-caching: FAIL: $*" >&2; exit 1; }

if mkdir "$fetchcontent_lock" 2>/dev/null; then
    lock_owned=1
elif [ -r "$fetchcontent_lock/pid" ] \
    && ! kill -0 "$(cat "$fetchcontent_lock/pid" 2>/dev/null)" 2>/dev/null; then
    rm -rf "$fetchcontent_lock"
    if mkdir "$fetchcontent_lock" 2>/dev/null; then
        lock_owned=1
    fi
fi
if [ "$lock_owned" -eq 1 ]; then
    printf '%s' "$$" >"$fetchcontent_lock/pid"
else
    fetchcontent_base="$sandbox/fetchcontent"
fi

configure() { # <build-dir> [extra cmake args...]
    local dir="$1"; shift
    local log="$sandbox/configure.log"
    if [ -f "$dir/CMakeCache.txt" ]; then
        if ! cmake -S "$repo_root" -B "$dir" \
            -DFETCHCONTENT_BASE_DIR="$fetchcontent_base" "$@" >"$log" 2>&1; then
            cat "$log" >&2
            return 1
        fi
    elif ! cmake -S "$repo_root" -B "$dir" -G Ninja \
        -DFETCHCONTENT_BASE_DIR="$fetchcontent_base" "$@" >"$log" 2>&1; then
        cat "$log" >&2
        return 1
    fi
}

launcher() { # <build-dir> -> the Ninja LAUNCHER value, or empty
    local line
    line="$(grep -m1 -E '^[[:space:]]*LAUNCHER =' "$1/build.ninja" 2>/dev/null || true)"
    [ -n "$line" ] || return 0
    printf '%s' "${line#*= }" | sed -E 's/[[:space:]]*$//'
}

build_dir() { # <build-dir> [extra cmake --build args...]
    local dir="$1"; shift
    if [ -n "${CMAKE_BUILD_PARALLEL_LEVEL:-}" ]; then
        cmake --build "$dir" "$@" --parallel "$CMAKE_BUILD_PARALLEL_LEVEL"
    else
        cmake --build "$dir" "$@" --parallel
    fi
}

sccache_hits() {
    sccache --show-stats | awk '$1=="Cache" && $2=="hits" && $3 ~ /^[0-9]+$/ {print $3}'
}

# A PATH where sccache is absent but every other tool stays reachable. A
# sccache-bearing directory (Fedora and Homebrew put it in the same bin as cmake
# and git) is replaced by a shim of symlinks to its other entries, so hiding
# sccache does not also hide the toolchain.
path_without_sccache() {
    local out="" d masked b f
    local shim="$sandbox/no-sccache"
    local -a dirs
    mkdir -p "$shim"
    IFS=':' read -ra dirs <<< "$PATH"
    for d in "${dirs[@]}"; do
        [ -n "$d" ] || continue
        if [ -x "$d/sccache" ]; then
            masked="$shim/${d//\//_}"
            mkdir -p "$masked"
            for f in "$d"/*; do
                [ -e "$f" ] || continue
                b=${f##*/}
                [ "$b" = "sccache" ] && continue
                ln -sf "$f" "$masked/$b"
            done
            out="${out:+$out:}$masked"
        else
            out="${out:+$out:}$d"
        fi
    done
    printf '%s' "$out"
}

if [ "$rebuild" -eq 1 ]; then
    command -v sccache >/dev/null 2>&1 || fail "--rebuild needs sccache on PATH"
    dir="$rebuild_dir"
    if [ -z "$dir" ]; then
        dir="$sandbox/rebuild"
        configure "$dir"
    elif [ ! -f "$dir/build.ninja" ]; then
        configure "$dir"
    fi
    build_dir "$dir" --target pictura >/dev/null
    hits_before="$(sccache_hits)"
    test_objects_before=$(find "$dir" -path '*/CMakeFiles/tst_*.dir/*' \( -name '*.o' -o -name '*.obj' \) | wc -l | tr -d ' ')
    probe=""
    while IFS= read -r obj; do
        probe="$obj"
        break
    done < <(find "$dir" -path '*/CMakeFiles/pictura_shell.dir/*' \( -name '*.o' -o -name '*.obj' \))
    [ -n "$probe" ] || fail "rebuild: no pictura_shell object found under $dir"
    rm -f "$probe"
    build_dir "$dir" --target pictura_shell >/dev/null
    test_objects_after=$(find "$dir" -path '*/CMakeFiles/tst_*.dir/*' \( -name '*.o' -o -name '*.obj' \) | wc -l | tr -d ' ')
    if [ "${test_objects_after:-0}" -lt "${test_objects_before:-0}" ]; then
        fail "rebuild: test objects disappeared (${test_objects_before:-0} -> ${test_objects_after:-0})"
    fi
    hits_after="$(sccache_hits)"
    if [ "${hits_after:-0}" -le "${hits_before:-0}" ]; then
        fail "rebuild: cache hits did not increase (${hits_before:-0} -> ${hits_after:-0})"
    fi
    echo "check-local-build-caching: rebuild hit the cache OK (${hits_before:-0} -> ${hits_after:-0})"
    echo "check-local-build-caching: OK"
    exit 0
fi

# Case 1: default - sccache on PATH, no override -> launcher is sccache.
if command -v sccache >/dev/null 2>&1; then
    configure "$sandbox/default"
    got="$(launcher "$sandbox/default")"
    case "$got" in
        *sccache*) ;;
        *) fail "default case: expected sccache launcher, got '${got}'" ;;
    esac
    echo "check-local-build-caching: default -> sccache OK"
else
    echo "check-local-build-caching: default case skipped (sccache not on PATH)"
fi

# Case 2: an explicit launcher wins over the sccache default.
configure "$sandbox/override" -DCMAKE_CXX_COMPILER_LAUNCHER=ccache
got="$(launcher "$sandbox/override")"
case "$got" in
    *sccache*) fail "override case: default leaked in, got '${got}'" ;;
    *ccache*) ;;
    *) fail "override case: explicit launcher not honoured, got '${got}'" ;;
esac
echo "check-local-build-caching: override -> ccache OK"

# Case 3: sccache absent -> no launcher and no unresolved launcher command.
PATH="$(path_without_sccache)" configure "$sandbox/absent"
got="$(launcher "$sandbox/absent")"
[ -z "$got" ] || fail "absent case: expected no launcher, got '${got}'"
if grep -q 'SCCACHE_EXECUTABLE-NOTFOUND' "$sandbox/absent/build.ninja"; then
    fail "absent case: unresolved launcher command generated"
fi
echo "check-local-build-caching: absent -> no launcher OK"

echo "check-local-build-caching: OK"
