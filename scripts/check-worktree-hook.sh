#!/usr/bin/env bash
#
# check-worktree-hook.sh - prove .githooks/post-checkout end to end.
#
# Builds a throwaway git repo in mktemp, commits the real hook from this
# checkout, creates a linked worktree, and asserts the wiring the
# meta/worktree-hooks spec requires: both symlinks, idempotent second
# checkout, the missing-fixtures skip path, and the opt-in cross-worktree
# sccache config sourced from the user-global env file / process env. Never
# touches the real checkout or the real ~/.config; a sandbox HOME and a stubbed
# sccache keep it hermetic. Exits non-zero on any failure.

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

# Hermetic HOME and a stub sccache: the hook must never touch the real config,
# and the stub records the daemon calls it makes.
export HOME="$sandbox/home"
export XDG_CONFIG_HOME="$HOME/.config"
unset SCCACHE_CONF KOOKA_SCCACHE_SHARE KOOKA_ENV_FILE || true
conf="$XDG_CONFIG_HOME/sccache/config"
kooka_env="$XDG_CONFIG_HOME/kooka-pictura/env"
log="$sandbox/sccache.log"
mkdir -p "$HOME" "$sandbox/bin"
cat > "$sandbox/bin/sccache" <<'STUB'
#!/usr/bin/env bash
echo "$*" >> "${SCCACHE_STUB_LOG:?}"
exit 0
STUB
chmod 755 "$sandbox/bin/sccache"
export SCCACHE_STUB_LOG="$log"
export PATH="$sandbox/bin:$PATH"

write_env_file() { mkdir -p "$(dirname "$kooka_env")"; printf 'KOOKA_SCCACHE_SHARE=1\n' > "$kooka_env"; }
rm_env_file() { rm -f "$kooka_env"; }

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

# Opt-in off: no env file, no process env -> no config, hook still wires symlinks.
git -C "$main" worktree add -q -b scratch "$wt" || fail "git worktree add"
[ -L "$wt/target" ] || fail "target is not a symlink"
[ "$(readlink "$wt/target")" = "../.target" ] || fail "target does not point at ../.target"
[ -d "$wt/target" ] || fail "target symlink does not resolve to a directory"
[ -L "$wt/reference-fixtures" ] || fail "reference-fixtures is not a symlink"
[ -e "$wt/reference-fixtures/marker" ] || fail "reference-fixtures symlink does not resolve"
[ ! -e "$conf" ] || fail "config written though opt-in is off"

# Opt-in via the user-global env file (process env unset).
write_env_file
git -C "$wt" checkout -q HEAD || fail "checkout with env-file opt-in errored"
[ -f "$conf" ] || fail "env-file opt-in did not write the config"
sed -n '1p' "$conf" | grep -q '^# managed by kooka-pictura' || fail "config lacks the managed sentinel"
grep -q 'hash_working_directory = false' "$conf" || fail "config lacks hash_working_directory = false"
grep -qF "\"$main\"" "$conf" || fail "config basedirs missing the main root"
grep -qF "\"$wt\"" "$conf" || fail "config basedirs missing the worktree root"
grep -q -- '--stop-server' "$log" || fail "daemon was not restarted after the initial write"

# No-op: an unchanged config is not rewritten and the daemon is not restarted.
before=$(ls -A "$wt" | sort)
conf_before=$(cksum "$conf")
stops_before=$(grep -c -- '--stop-server' "$log")
git -C "$wt" checkout -q HEAD || fail "second checkout errored"
after=$(ls -A "$wt" | sort)
[ "$before" = "$after" ] || fail "second checkout changed the worktree (not idempotent)"
[ "$(cksum "$conf")" = "$conf_before" ] || fail "config was rewritten though unchanged"
[ "$(grep -c -- '--stop-server' "$log")" = "$stops_before" ] || fail "daemon restarted though config unchanged"

# Opt-in via the process environment alone (no env file).
rm_env_file
rm -f "$conf"
KOOKA_SCCACHE_SHARE=1 git -C "$wt" checkout -q HEAD || fail "checkout with process-env opt-in errored"
[ -f "$conf" ] || fail "process-env opt-in did not write the config"

# A repository-local env file cannot enable the opt-in.
rm_env_file
rm -f "$conf"
printf 'KOOKA_SCCACHE_SHARE=1\n' > "$wt/.env"
git -C "$wt" checkout -q HEAD || fail "checkout with repo-local env errored"
[ ! -e "$conf" ] || fail "repo-local .env enabled the opt-in"
rm -f "$wt/.env"

# No-clobber: a config without the sentinel is left untouched.
write_env_file
printf 'manifest-ignore = true\n' > "$conf"
git -C "$wt" checkout -q HEAD || fail "checkout with unmanaged config errored"
[ "$(cat "$conf")" = "manifest-ignore = true" ] || fail "unmanaged config was modified"
rm -f "$conf"

# Opt-off: opt-in removed -> nothing written.
rm_env_file
git -C "$wt" checkout -q HEAD || fail "checkout with opt-in off errored"
[ ! -e "$conf" ] || fail "config written though opt-in is off"

# sccache-absent: opt-in on, no sccache on PATH -> nothing written, no failure.
write_env_file
# A PATH where sccache is absent but every other tool (git!) stays reachable: a
# sccache-bearing directory is replaced by a shim of symlinks to its other
# entries, so hiding sccache does not also hide the toolchain (Fedora/Homebrew
# keep sccache in the same bin as git).
shim="$sandbox/no-sccache"; mkdir -p "$shim"
no_sccache_path=""
IFS=':' read -ra path_dirs <<< "$PATH"
for d in "${path_dirs[@]}"; do
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
        no_sccache_path="${no_sccache_path:+$no_sccache_path:}$masked"
    else
        no_sccache_path="${no_sccache_path:+$no_sccache_path:}$d"
    fi
done
PATH="$no_sccache_path" git -C "$wt" checkout -q HEAD || fail "checkout without sccache errored"
[ ! -e "$conf" ] || fail "config written though sccache is absent"

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
