# Tasks

## 1. Tests first

- [x] 1.1 Add Rust history tests for non-linear append/undo, state materialization, clear-vs-purge, and the save snapshot hook.
- [x] 1.2 Add `tst_history_options` Qt Test cases for the dialog, persistence, clear/Alt clear, New Document from a state/snapshot, automatic first/save snapshots, visibility gating, and the snapshot button.
- [x] 1.3 Register `tst_history_options` in `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 2. History engine

- [x] 2.1 Add `non_linear` to `History`, `set_non_linear`, and the append-a-full-anchor capture path.
- [x] 2.2 Add `state_snapshot(i)` wrapping `materialize(i)`.
- [x] 2.3 Add per-view option fields and the `set_history_options` bridge; re-apply non-linear on `replace_document`.
- [x] 2.4 Add the untracked edit path and gate layer visibility capture.
- [x] 2.5 Add the successful non-copy save snapshot hook.

## 3. Panel and commands

- [x] 3.1 Add the five session options and load/save them.
- [x] 3.2 Add the History Options dialog and push accepted values to the view.
- [x] 3.3 Add Clear History (plain and Alt purge), selected-row target, New Document bridge, and the snapshot button's automatic/prompt behavior.
- [x] 3.4 Add `history_new_document` and `set_history_options` to `history_api` and the Rust bridge.
- [x] 3.5 Add the three panel command ids to the catalogue/golden files and regenerate `command_ids.h`.
- [x] 3.6 Enable the three menu rows and bind their command handlers; add enablement providers.
- [x] 3.7 Add the initial-snapshot hook to `wireDocument`.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all`, clippy, and the Rust tests.
- [x] 4.2 Run the Qt test suite and the full fast gate.
- [x] 4.3 Update `ROADMAP.md`, `PARITY.md`, the parity gap register, the M42 history table, and `docs/dev/STATE.md`.
- [x] 4.4 Archive the OpenSpec change.
