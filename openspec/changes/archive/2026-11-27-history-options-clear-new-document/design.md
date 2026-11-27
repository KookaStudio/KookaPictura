# Design

## Context

The history engine materializes one state at the cursor and stores every other
state as metadata plus tile deltas (`crates/pictura-app/src/history.rs`). It
already has:

- `materialize(target)` — rebuilds any retained state without moving the cursor.
- `purge_states()` — collapses to the current state, keeping named snapshots.
- `purge()` — collapses to the current state and drops named snapshots.
- `snapshot(i)` — clones a named snapshot.

The panel (`crates/pictura-app/cpp/panels/history_panel.cpp`) lists snapshots and
states, supports jump/restore/brush source, and creates a snapshot through a
`QInputDialog`. The three missing menu rows are disabled in
`panels/panel_group_menu.cpp`.

## Goals / Non-Goals

**Goals:**

- Five working History Options toggles, persisted across restarts.
- Clear History with a plain/Alt split that keeps or drops snapshots.
- New Document from the selected state or snapshot, with a one-state history.
- Non-linear history that appends rather than truncates.
- Automatic initial and save snapshots.
- Layer visibility history gated by its option.

**Non-Goals:**

- Snapshot `From` modes, rename/delete, or drag-to-document.
- A retained-but-hidden memory pool for plain Clear History. The current engine
  releases the dropped states' tiles; the observable difference is that plain
  Clear keeps snapshots while Alt clear drops them.
- Per-document history option overrides; the options are application-wide.

## Decisions

### Options live in the UI session store, behavior flags live on the view

The five booleans are panel/application state, like the Layers panel options, so
they belong in `SessionState` (`session.{h,cpp}`) rather than the TOML preference
file. `HistoryPanel` owns the values, loads them in its constructor, writes them
when the dialog is accepted, and exposes getters for the frame and the command
handlers.

The two flags the Rust engine must see immediately are copied to
`PictureViewRust`:

- `history_allow_non_linear`
- `history_visibility_undoable`
- `history_auto_snapshot_on_save`

`HistoryPanel::setView` pushes them through a new bridge whenever the active view
changes, and `openHistoryOptions` pushes them when the dialog is accepted.
`replace_document` re-applies `allow_non_linear` to the new `History`, because
open/new/duplicate install a fresh history. The initial-snapshot flag stays in
C++ and is read by `PicturaMainWindow::wireDocument`.

### Non-linear capture appends a full anchor

Linear capture truncates `states[cursor + 1..]` and stores a delta against the
materialized cursor state. Non-linear capture must keep the later states, so it:

1. takes the materialized cursor state,
2. `fill_anchor`s it so the cursor's segment keeps a full copy of that state,
3. stores the new document as a **full anchor** at the end, and
4. leaves `current` at the new state.

Undo from the appended state crosses back into the previous segment, which
re-materializes the previous recorded state from its own anchor. This is the
documented non-linear behavior: states append in recording order and undo/redo
follow that order, not a branch tree. The cost is one full snapshot per
non-linear branch capture; the linear path remains delta-based. A `ponytail:`
comment records that ceiling.

### Visibility edits choose tracked or untracked capture

`PictureView::edit` always captures. Add an `edit_untracked` sibling that runs the
same transaction and presentation but skips the history capture. The visibility
methods call it when `history_visibility_undoable` is off; the pixel change,
dirty flag, composite refresh, and `layerVisibilityChanged` signal are unchanged.
A later tracked edit diffs against the still-current history state, so an
untracked visibility change becomes part of the next recorded state, matching
CS6's "not separately undoable" behavior.

### New Document uses the engine's materialization

Add `History::state_snapshot(i) -> Option<Snapshot>` wrapping `materialize(i)`.
A new free bridge `history_new_document(source, target, snapshot, index)`:

- resolves a named snapshot through `snapshot(i)` or a state through
  `state_snapshot(i)`;
- installs the snapshot document on the target with an empty history and no
  path/selection;
- selects the bottom layer as `duplicate_into` does;
- records exactly one `Duplicate State` state with `Edit::Full`.

The source view is borrowed immutably, so the new document cannot disturb it.
The target is a fresh `PictureView` created by the command handler; the handler
adds it to the document registry. The initial-snapshot hook may then add the
normal initial snapshot to the new document, which is consistent with any other
new document.

### Clear History split

- Plain `Clear History` calls `purge_states()`: the undo list collapses to the
  current state, named snapshots survive.
- `Alt`/`Option` `Clear History` calls `purge()`: the undo list collapses and
  named snapshots are dropped.
- Both are outside the undo system.

The menu action checks `QApplication::keyboardModifiers()` at trigger time.
`performPanelMenuAction("clearHistoryPurge")` is also accepted as an explicit
test seam.

### Automatic snapshots

- First snapshot: `PicturaMainWindow::wireDocument` adds `Snapshot 1` when the
  option is on, the view has a document, history count is at most one, and no
  snapshot exists. This covers synchronous adds and the async open's
  `finishLoadingDocument` path.
- Save snapshot: `save_finish_rust` adds `Snapshot N` after a successful
  non-copy save when the view's flag is set. It uses `named_snapshot()` so the
  snapshot shares tiles with the document.
- Snapshot labels are `Snapshot <n>` where `n` is the next count, matching the
  automatic naming in CS6.

### Snapshot button

`Show New Snapshot Dialog By Default` controls the bottom `Create Snapshot`
button only. When off, the button adds `Snapshot N` immediately; when on, it
prompts. The menu `New Snapshot…` always prompts. `Alt`-click snapshot options
remain out of scope.

## Risks

- Existing tests that assert absolute snapshot counts after `frame.newDocument`
  will see the initial `Snapshot 1`; update them to count relative to the
  initial snapshot or to clear it.
- Non-linear history is new storage behavior; the Rust tests must prove undo
  from an appended state lands on the previously recorded state and that the
  later states remain.
- The save-snapshot hook runs inside the save finish path; it must not run for
  copies or failed/uncertain saves.
