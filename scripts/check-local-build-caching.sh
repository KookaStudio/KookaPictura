#!/usr/bin/env bash
#
# check-local-build-caching.sh - prove the CMake default wires sccache.
#
# Default mode: configure the project into throwaway build directories and
# assert the Ninja LAUNCHER build-rule variable (sccache by default, an explicit
# override wins, none when sccache is absent). --rebuild mode (with optional
# --dir <build-dir>): build the pictura target, delete its CMake object files,
# rebuild, and assert the second build hits the cache. Never touches the real
# build/ unless --dir names it, and restores the repo-root compile_commands.json
# symlink. Self-skips when cmake or qmake6 is absent.

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
trap 'restore_compile_db; rm -rf "$sandbox"' EXIT

fail() { echo "check-local-build-caching: FAIL: $*" >&2; exit 1; }

configure() { # <build-dir> [extra cmake args...]
    local dir="$1"; shift
    cmake -S "$repo_root" -B "$dir" -G Ninja \
        -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld "$@" >/dev/null
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
    find "$dir" -path '*/CMakeFiles/*' \( -name '*.o' -o -name '*.obj' \) -delete
    build_dir "$dir" --target pictura >/dev/null
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
