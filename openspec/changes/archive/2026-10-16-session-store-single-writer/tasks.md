# Tasks: session-store-single-writer

## 1. Guard

- [x] 1.1 `session.cpp`: per-launch `instanceToken`/`instanceStartMs` statics;
  `saveSession` refuses a store another launch wrote at or after this launch's
  start and stamps `writerToken`/`writtenAt` on every write.
- [x] 1.2 `frame_session.cpp`: skip `snapshotActiveWorkspace()` when the session
  write is refused.
- [x] 1.3 `tst_session_store`: own writes round-trip; a newer foreign write is
  refused and the store is unchanged; a foreign write from before the launch is
  adopted and re-stamped.

## 2. Self-test hardening

- [x] 2.1 `main.cpp`: an invalid self-test temp dir prints a FAIL token and
  returns 199 (the next free code) instead of falling back to the real store.
- [x] 2.2 `selftest.cpp`: drop the redundant second `XDG_STATE_HOME` redirect.

## 3. Verification

- [x] 3.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo nextest run --workspace`, `ctest --test-dir build -R
  '^tst_'`, both `--headless --self-test` invocations, `bash
  scripts/verify-fast.sh`, `openspec validate --all --strict`. The unified
  report's only failure is the pre-existing local ImageMagick fixture mismatch
  (`imagemagick_fixtures_reproduce`), which fails on master too.
