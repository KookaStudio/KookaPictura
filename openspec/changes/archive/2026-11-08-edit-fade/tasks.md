# Tasks

## 1. Spec and test plan

- [x] 1.1 Write the `imaging/filter-fade` ADDED delta for the compositor, capture, dialog, and command wiring
- [x] 1.2 Pair it with `test-plan.md`, one row per scenario
- [x] 1.3 Add the named Rust and Qt tests before the implementation they cover

## 2. Engine

- [x] 2.1 Move the W3C blend function to `pictura_core::blend` and re-export it from `pictura-render`
- [x] 2.2 Add `pictura-filters::fade` with `Fade` and `fade_layer`, including the source-over alpha composite and the 0/100 fast paths
- [x] 2.3 Add the engine unit tests for lerp, blend, alpha, non-color channels, and the union rectangle

## 3. Session state and capture

- [x] 3.1 Add `FadeState` to `PictureViewRust` and the fade module helpers (`available`, `name`, `preview`, `apply`, `cancel`, `mode_allowed`)
- [x] 3.2 Clear the record in `apply_edit_with` and `impl_history::restore`
- [x] 3.3 Capture original/effected layers in the filter and adjustment worker commit and install the record in `commit_finish`
- [x] 3.4 Capture in the synchronous filter and adjustment paths
- [x] 3.5 Capture paint/erase at `begin_paint`/`end_paint` and Magic Eraser
- [x] 3.6 Add the cxx-qt bridge functions for availability, label, prefill, preview, apply, cancel, and Lab mode gating

## 4. UI

- [x] 4.1 Add `FadeDialog` with Opacity, Mode, Preview, OK/Cancel and Lab-disabled modes; register it in `CMakeLists.txt`
- [x] 4.2 Mark `edit.fade` implemented, generate `command_ids::EditFade`, and wire the handler, enabled provider, and label provider
- [x] 4.3 Add `tst_fade` and register it in the tests `CMakeLists.txt`

## 5. Docs and bookkeeping

- [x] 5.1 Tick `ROADMAP.md`, update `docs/dev/STATE.md`, and remove SYS-3 from `docs/dev/doc-code-parity-gaps.md`
- [x] 5.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`
- [x] 5.3 Build once, run the affected Rust and Qt tests, and run `verify-fast.sh`
- [x] 5.4 Archive the change with `openspec archive edit-fade`
