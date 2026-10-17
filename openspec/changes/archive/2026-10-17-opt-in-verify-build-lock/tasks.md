# Tasks

## 1. Build-lock check and helper

- [x] 1.1 Write `scripts/check-build-lock.sh`: a helper-level, sandboxed check (temporary `HOME`, `XDG_CONFIG_HOME`, `XDG_RUNTIME_DIR`, `TMPDIR`, and `KOOKA_ENV_FILE`; `PICTURA_VERIFY_LOCK*` and the nesting marker cleared) covering `bl_off`, `bl_envfile` (including CRLF/quotes/inline comments and the default path), `bl_precedence` (both variables), `bl_notracked`, `bl_queue`, `bl_nested`, `bl_nested_release`, `bl_release_ownership`, `bl_symlink`, `bl_docs_only` (static ordering and marker wiring), `bl_timeout`, `bl_timeout_override`, `bl_invalid_timeout` (`abc`/`00`/`-1`), `bl_stale`, `bl_invalid_pid`, `bl_empty_owner`, `bl_reclaim_race`, `bl_slow_git`, and `bl_signal` (`TERM`/`INT` plus child termination); verify it runs and reports the missing helper before 1.2 lands
- [x] 1.2 Implement `scripts/build-lock.sh`: sourceable `build_lock_enabled` / `build_lock_acquire` / `build_lock_release` with set-versus-unset process-env-then-env-file resolution, validated timeout, atomic `mkdir -m 700` lock directory with ownership/symlink checks, atomically published owner record (`pid`, worktree, branch, script, since), bounded wait, empty-owner grace, reap-lock stale reclaim, idempotent acquire, and ownership-scoped release; verify `bash scripts/check-build-lock.sh` exits 0
- [x] 1.3 Wire `scripts/check-build-lock.sh` into the full gate in `scripts/verify-fast.sh`; verify `rg -n check-build-lock scripts/verify-fast.sh` shows the call and a full `verify-fast.sh` run reaches it

## 2. Wire the verification scripts

- [x] 2.1 In `scripts/verify-fast.sh`, source the helper after the `cd`, acquire only after the docs-only early exit, and release via `EXIT`/`INT`/`TERM` traps; verify a docs-only run in a throwaway docs-only worktree creates no lock directory and a full run acquires and removes it
- [x] 2.2 In `scripts/verify-full.sh`, acquire before CMake configure/build, export `PICTURA_VERIFY_LOCK_HELD=1` only when it actually acquired, and release via traps so the nested `verify-fast.sh` neither re-acquires nor releases; verify `PICTURA_VERIFY_LOCK=1 bash scripts/verify-full.sh` completes and leaves no lock directory behind

## 3. Configuration and developer documentation

- [x] 3.1 Add commented `PICTURA_VERIFY_LOCK=1` and `PICTURA_VERIFY_LOCK_TIMEOUT=3600` under the “Project opt-ins” section of `.env.example`; verify both names match the helper's lookups, `rg -n PICTURA_VERIFY_LOCK .env.example` shows both, and the example still enables nothing as shipped
- [x] 3.2 Add a short opt-in note to the “Worktrees” section of `DEVELOPING.md` (off by default, enabling and timeout variables, default timeout, and that it covers CMake plus cargo work across worktrees); verify the names and default match the scripts

## 4. Verification and archive

- [x] 4.1 Run `bash -n` on `scripts/build-lock.sh`, `scripts/check-build-lock.sh`, `scripts/verify-fast.sh`, and `scripts/verify-full.sh`, then `bash scripts/check-build-lock.sh`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`; verify all exit 0
- [x] 4.2 After 1.1–4.1 are checked off, archive with `openspec archive opt-in-verify-build-lock --yes`; verify `openspec list --json` reports no active change and the delta has merged into `openspec/specs/verification/verification-harness/spec.md`
- [x] 4.3 Post-archive, run `bash scripts/check-openspec-archived.sh`, `bash scripts/verify-fast.sh`, and `PICTURA_VERIFY_LOCK=1 bash scripts/verify-full.sh`; verify all exit 0
- [x] 4.4 Verify the queue and docs-only behavior through `scripts/check-build-lock.sh` (`bl_queue`, `bl_docs_only`); a literal two-process `verify-fast.sh` run and a throwaway docs-only worktree run are not part of the local gate
- [x] 4.5 List the test plan's `manual:` rows in the pull request body as a reviewer checklist; verify every `manual:` row is present
