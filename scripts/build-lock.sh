#!/usr/bin/env bash
#
# build-lock.sh - opted-in serialization for the heavy local verify scripts.
#
# Sourced by verify-fast.sh and verify-full.sh. Enable with
# PICTURA_VERIFY_LOCK=1 in the process environment or the user-global env file
# (${KOOKA_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env}).
# PICTURA_VERIFY_LOCK_TIMEOUT bounds the wait in seconds (default 3600;
# 0 = no limit). See DEVELOPING.md, "Worktrees".
#
# The lock is a mode-700 directory created with atomic mkdir. It holds an owner
# record and is reclaimed when the recorded PID is not live. The primitive is
# portable across Linux, macOS, and git-bash: no flock, /proc, GNU stat, or
# abstract sockets.

_build_lock_owned=0
_build_lock_dir=
_build_lock_self_pid=

build_lock_env_file() {
    if [ -n "${KOOKA_ENV_FILE:-}" ]; then
        printf '%s' "$KOOKA_ENV_FILE"
    elif [ -n "${XDG_CONFIG_HOME:-}" ]; then
        printf '%s/kooka-pictura/env' "$XDG_CONFIG_HOME"
    elif [ -n "${HOME:-}" ]; then
        printf '%s/.config/kooka-pictura/env' "$HOME"
    fi
}

build_lock_has_setting() { # <NAME>
    local name=$1 file
    if [ "${!name+x}" = x ]; then
        return 0
    fi
    file=$(build_lock_env_file)
    [ -n "$file" ] && [ -r "$file" ] || return 1
    grep -qE "^[[:space:]]*(export[[:space:]]+)?${name}=" "$file" 2>/dev/null
}

build_lock_setting() { # <NAME>
    local name=$1 file
    if [ "${!name+x}" = x ]; then
        printf '%s' "${!name}"
        return 0
    fi
    file=$(build_lock_env_file)
    [ -n "$file" ] && [ -r "$file" ] || return 0
    sed -nE "s/^[[:space:]]*(export[[:space:]]+)?${name}=//p" "$file" 2>/dev/null \
        | tail -1 \
        | sed -E "s/\r$//; s/^[[:space:]]*[\"']?//; s/[\"']?[[:space:]]*(#.*)?$//"
}

build_lock_enabled() {
    [ "$(build_lock_setting PICTURA_VERIFY_LOCK)" = "1" ]
}

build_lock_timeout() {
    local raw
    if build_lock_has_setting PICTURA_VERIFY_LOCK_TIMEOUT; then
        raw=$(build_lock_setting PICTURA_VERIFY_LOCK_TIMEOUT)
    else
        raw=3600
    fi
    case "$raw" in
        0) printf '0' ;;
        '' | *[!0-9]* | 0*)
            printf 'build-lock: invalid PICTURA_VERIFY_LOCK_TIMEOUT=%s (want 0 or a positive integer)\n' "$raw" >&2
            return 1
            ;;
        *) printf '%s' "$raw" ;;
    esac
}

build_lock_dir_path() {
    printf '%s/kooka-pictura-build-%s.lock' \
        "${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}" "$(id -u)"
}

build_lock_branch() {
    local branch
    branch=$(git branch --show-current 2>/dev/null || true)
    if [ -z "$branch" ]; then
        branch=$(git rev-parse --short HEAD 2>/dev/null || true)
    fi
    printf '%s' "${branch:-unknown}"
}

_build_lock_pid_valid() { # <pid>
    case "$1" in
        '' | *[!0-9]* | 0*) return 1 ;;
    esac
    return 0
}

_build_lock_owner_pid() { # <dir>
    sed -n '1s/^pid=//p' "$1/owner" 2>/dev/null
}

_build_lock_owner_text() {
    printf 'pid=%s\nworktree=%s\nbranch=%s\nscript=%s\nsince=%s\n' \
        "$1" "$PWD" "$(build_lock_branch)" "${0##*/}" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
}

_build_lock_reap() { # <dir>; 0 = gone/removed, 1 = still present
    local dir=$1 reap="$1.reap" pid stale
    if [ -d "$reap" ] && [ -z "$(find "$reap" -prune -mmin -1 2>/dev/null)" ]; then
        rmdir -- "$reap" 2>/dev/null || true
    fi
    mkdir -m 700 "$reap" 2>/dev/null || return 1
    if [ ! -d "$dir" ]; then
        rmdir -- "$reap" 2>/dev/null || rm -rf -- "$reap"
        return 0
    fi
    pid=$(_build_lock_owner_pid "$dir")
    if [ -z "$pid" ]; then
        sleep 1
        pid=$(_build_lock_owner_pid "$dir")
    fi
    if [ -z "$pid" ] || ! _build_lock_pid_valid "$pid" || ! kill -0 "$pid" 2>/dev/null; then
        stale="$dir.stale.$$"
        if mv -- "$dir" "$stale" 2>/dev/null; then
            rm -rf -- "$stale"
            rmdir -- "$reap" 2>/dev/null || rm -rf -- "$reap"
            return 0
        fi
    fi
    rmdir -- "$reap" 2>/dev/null || rm -rf -- "$reap"
    return 1
}

build_lock_acquire() {
    local dir timeout start now elapsed self_pid owner_text tmp pid seen=0 strikes=0 mkdir_err parent

    if [ "${_build_lock_owned:-0}" = "1" ] && [ -n "${_build_lock_dir:-}" ] \
        && [ -d "$_build_lock_dir" ] && [ ! -L "$_build_lock_dir" ] \
        && [ "$(_build_lock_owner_pid "$_build_lock_dir")" = "${_build_lock_self_pid:-}" ]; then
        return 0
    fi
    _build_lock_owned=0
    _build_lock_dir=

    build_lock_enabled || return 0
    if [ "${PICTURA_VERIFY_LOCK_HELD:-}" = "1" ]; then
        return 0
    fi

    dir=$(build_lock_dir_path)
    timeout=$(build_lock_timeout) || return 1
    self_pid=${BASHPID:-$$}
    owner_text=$(_build_lock_owner_text "$self_pid")
    start=$(date +%s)

    while :; do
        now=$(date +%s)
        elapsed=$((now - start))
        if [ "$timeout" -gt 0 ] && [ "$elapsed" -ge "$timeout" ]; then
            printf 'build-lock: timed out after %ss waiting for the holder\n' "$timeout" >&2
            if [ -f "$dir/owner" ]; then
                sed 's/^/  /' "$dir/owner" >&2 2>/dev/null || true
            fi
            return 1
        fi

        if mkdir_err=$(mkdir -m 700 "$dir" 2>&1); then
            if ! printf 'pid=%s\n' "$self_pid" >"$dir/owner" 2>/dev/null; then
                if [ ! -d "$dir" ]; then
                    continue
                fi
                rmdir -- "$dir" 2>/dev/null || true
                printf 'build-lock: could not publish the owner record in %s\n' "$dir" >&2
                return 1
            fi
            tmp="$dir/.owner.$$"
            if printf '%s\n' "$owner_text" >"$tmp" 2>/dev/null; then
                mv -f "$tmp" "$dir/owner" 2>/dev/null || rm -f "$tmp" 2>/dev/null || true
            fi
            _build_lock_owned=1
            _build_lock_dir=$dir
            _build_lock_self_pid=$self_pid
            return 0
        fi

        if [ ! -e "$dir" ] && [ ! -L "$dir" ]; then
            parent=${dir%/*}
            if [ ! -d "$parent" ]; then
                printf 'build-lock: cannot create %s: %s is not a directory\n' "$dir" "$parent" >&2
                return 1
            fi
            if [ ! -w "$parent" ]; then
                printf 'build-lock: cannot create %s: %s is not writable\n' "$dir" "$parent" >&2
                return 1
            fi
            strikes=$((strikes + 1))
            if [ "$strikes" -ge 5 ]; then
                printf 'build-lock: cannot create %s: %s\n' "$dir" "${mkdir_err:-unknown error}" >&2
                return 1
            fi
            sleep 0.2
            continue
        fi
        strikes=0

        if [ ! -d "$dir" ] || [ -L "$dir" ]; then
            printf 'build-lock: cannot use %s (not a directory or a symlink)\n' "$dir" >&2
            return 1
        fi
        if [ ! -O "$dir" ]; then
            printf 'build-lock: %s is not owned by the current user; refusing to reclaim it\n' "$dir" >&2
            return 1
        fi

        pid=$(_build_lock_owner_pid "$dir")
        if [ -z "$pid" ]; then
            sleep 1
            pid=$(_build_lock_owner_pid "$dir")
            if [ -z "$pid" ]; then
                if _build_lock_reap "$dir"; then
                    sleep 0.2
                    continue
                fi
                sleep 0.5
                continue
            fi
        fi

        if ! _build_lock_pid_valid "$pid" || ! kill -0 "$pid" 2>/dev/null; then
            if _build_lock_reap "$dir"; then
                sleep 0.2
                continue
            fi
            sleep 0.5
            continue
        fi

        if [ "$seen" -eq 0 ]; then
            printf 'build-lock: waiting for another build to finish:\n' >&2
            sed 's/^/  /' "$dir/owner" >&2 2>/dev/null || true
            seen=1
        fi
        sleep 0.5
    done
}

build_lock_release() {
    if [ "${_build_lock_owned:-0}" = "1" ] && [ -n "${_build_lock_dir:-}" ]; then
        if [ -O "$_build_lock_dir" ] && [ ! -L "$_build_lock_dir" ] \
            && [ "$(_build_lock_owner_pid "$_build_lock_dir")" = "${_build_lock_self_pid:-}" ]; then
            rm -rf -- "$_build_lock_dir" || true
        fi
    fi
    _build_lock_owned=0
    _build_lock_dir=
    _build_lock_self_pid=
    return 0
}

_build_lock_descendants() { # <pid>
    local parent=$1 p pp table
    table=$(ps -eo pid=,ppid= 2>/dev/null || ps -axo pid=,ppid= 2>/dev/null) || return 0
    printf '%s\n' "$table" | while read -r p pp; do
        [ -n "$p" ] || continue
        [ "$pp" = "$parent" ] || continue
        printf '%s\n' "$p"
    done
}

_build_lock_signal_tree() { # <pid> <signal>
    local parent=$1 sig=$2 child
    for child in $(_build_lock_descendants "$parent"); do
        _build_lock_signal_tree "$child" "$sig"
        kill -"$sig" "$child" 2>/dev/null || true
    done
}

_build_lock_on_signal() { # <exit-code>
    local root=${BASHPID:-$$}
    trap - INT TERM
    _build_lock_signal_tree "$root" TERM
    sleep 1
    _build_lock_signal_tree "$root" KILL
    build_lock_release
    exit "$1"
}

build_lock_install_traps() {
    trap 'build_lock_release' EXIT
    trap '_build_lock_on_signal 130' INT
    trap '_build_lock_on_signal 143' TERM
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    printf 'build-lock.sh is sourced by the verify scripts; run scripts/check-build-lock.sh\n' >&2
    exit 2
fi
