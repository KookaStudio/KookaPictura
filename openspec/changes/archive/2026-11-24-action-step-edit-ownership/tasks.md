# Tasks

## 1. Regression coverage

- [x] 1.1 Add the import, edit, save, and reload tests to `crates/pictura-script/tests/atn.rs` for Play, Conditional, jsn-only Play, unknown descriptor items, JSON byte preservation, unrelated sidecars, and unsafe-sidecar fallback
- [x] 1.2 Add the no-op, structural, flag-only, and changed-payload tests to `crates/pictura-script/tests/atn.rs`
- [x] 1.3 Add `crates/pictura-script/tests/engine.rs::rejected_duplicate_rename_leaves_bytes_unchanged` and `crates/pictura-script/tests/atn.rs::renaming_an_action_leaves_unrelated_imported_steps_untouched`
- [x] 1.4 Run `python3 scripts/check-test-plan.py`; it passes

## 2. Edit ownership

- [x] 2.1 Make `ActionStep::kind` and `ActionStep::imported` private, add the constructor and accessors, and migrate every reader and constructor
- [x] 2.2 Add `set_kind`, `set_params`, and `rewrite_action_reference`; route parameter edits and Insert Path replacement through them

## 3. Reference rewrites

- [x] 3.1 Patch Play and Conditional imported descriptors with textual JSON span replacement, safe-sidecar matching, and re-authoring fallback
- [x] 3.2 Route `ActionLibrary::rename_action` through `rewrite_action_reference`

## 4. Adapter

- [x] 4.1 Update the actions-engine snapshot, library operations, and tool-recording test to the new accessors and constructor

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`; it passes
- [x] 5.2 Run the budgeted CMake configure and build; it passes after regenerating four stale zero-byte Qt MOC outputs, and the rebuild after the Clippy type-alias fix also passes
- [x] 5.3 Run `python3 scripts/check-test-plan.py`; it passes
- [x] 5.4 Run `openspec validate action-step-edit-ownership --strict`; it passes
- [x] 5.5 Run `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh`; fmt, Clippy, and build-tree checks pass, then the test report records 11,811 passed, 152 skipped, and one failure in `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`, so the guard phase is not reached
- [x] 5.6 Reproduce the GPU failure on clean `origin/master`; `tst_present_cache` fails on the same case with the same assertion
- [x] 5.7 Run the remaining `verify-fast` checks separately; all pass except `check-milestone-names.py --self-test`
- [x] 5.8 Reproduce the milestone-name self-test failure on clean `origin/master`; the same assertion fails
- [x] 5.9 Run `openspec validate --all --strict`; 529 specs pass
- [x] 5.10 Run `bash scripts/check-openspec-archived.sh`; it passes
- [x] 5.11 Run the MSVC string-limit and colour-literal self-tests; both pass

## 6. Archive and trackers

- [x] 6.1 Review `ROADMAP.md` and `PARITY.md`; Actions is already complete and this refactor changes no feature coverage, so neither tracker needs an edit
- [x] 6.2 Archive `action-step-edit-ownership` as `2026-11-24-action-step-edit-ownership`
