# Proposal

## Why

The History panel's three remaining CS6 commands are transcribed but disabled:
`History Options…`, `Clear History`, and `New Document` (`panel_group_menu.cpp`;
parity gap PAN-6, #514). The history engine already supports bounded undo,
snapshots, and non-destructive state materialization, but the app exposes no
option state, no clear-vs-purge distinction, and no way to turn a retained state
or snapshot into a document.

## What Changes

- Add a **History Options** dialog with the five CS6 toggles and persist them in
  the UI session store:
  - Automatically Create First Snapshot (default on)
  - Automatically Create New Snapshot When Saving (default off)
  - Allow Non-Linear History (default off)
  - Show New Snapshot Dialog By Default (default off)
  - Make Layer Visibility Changes Undoable (default off)
- Wire each option to behavior: the initial snapshot on document add, a snapshot
  on a successful non-copy save, non-linear append instead of redo truncation,
  the Create New Snapshot button's prompt/automatic mode, and whether layer
  visibility edits capture a history state.
- Enable **Clear History**: plain clear drops the undo states but keeps named
  snapshots; `Alt`/`Option` clear drops both. Neither is undoable.
- Enable **New Document**: materialize the selected state or named snapshot into
  a new untitled document whose history contains exactly one `Duplicate State`
  state; the source document is untouched.
- Update the History panel menu table, the panel command ids, and the command
  catalogue, and keep the dialog/panel command enabled only when a document
  exists (except History Options, which is always available).

Out of scope: snapshot `From` (Full Document / Merged Layers / Current Layer),
snapshot rename/delete, dragging a state onto an existing document, and the
History Log preference pane.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/history-panel`: the enabled History Options, Clear History, and New
  Document affordances, the snapshot button's prompt option, and their
  enablement.
- `document/edit-history`: non-linear capture, the visibility-undo option, the
  two automatic-snapshot options, state materialization for New Document, and
  the clear/purge split.

## Impact

- `crates/pictura-app/src/history.rs` and `history/`: a `non_linear` flag,
  non-linear capture, state materialization, and the clear/purge split already
  provided by `purge_states`/`purge`.
- `crates/pictura-app/src/cxxqt_object/state.rs` and
  `impl_history/purge.rs`: per-view option fields and the option/new-document
  bridge functions.
- `crates/pictura-app/src/cxxqt_object/edit.rs`, `impl_layers.rs`: an
  untracked edit path for visibility when the option is off.
- `crates/pictura-app/src/cxxqt_object/impl_core/save_jobs/imp.rs`: the save
  snapshot hook.
- `crates/pictura-app/cpp/panels/history_panel.{h,cpp}`: the options dialog,
  clear/new-document actions, selected-row target, and session persistence.
- `crates/pictura-app/cpp/panels/panel_group_menu.cpp`,
  `menu_panel_commands.cpp`, `frame_canvas.cpp`, `session.{h,cpp}`,
  `api/history_api.{h,cpp}`: menu/command wiring, initial snapshot, option
  persistence, and the bridge.
- `crates/pictura-script/src/command_catalogue/` and the generated
  `command_ids.h`: three panel command ids.
- Tests: Rust history/save unit tests and a new Qt Test suite
  `tst_history_options`.
- Docs (separate `TASK-ALLOWS-DOCS` commit): `ROADMAP.md`, `PARITY.md`,
  `docs/dev/doc-code-parity-gaps.md` PAN-6, `docs/dev/m42-panel-menus.md`, and
  `docs/dev/STATE.md`.
