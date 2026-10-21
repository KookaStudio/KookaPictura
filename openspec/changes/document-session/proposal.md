# Proposal

## Why

Document-session logic and Qt signal emission share `impl qobject::PictureView`
methods behind `Pin<&mut PictureView>`. `PictureViewRust` is a large backing
struct with dozens of `Option` session and cache slots, and a live gesture is
spread across independent fields with no shared lifecycle. Rust tests therefore
reach history and session behaviour only by calling private helpers, while the
observable paths are checked through the Qt binary.

The stale-state defect is a class, not one field. `reset_edit_state` clears 17
direct fields plus `reset_pyramid`, but not `deselected_selection`,
`selection_move_origin`, `heal_stroke`, `mask_stroke`, `filter_preview`,
`mask_filter_preview`, `mode_preview`, `path_gesture`, `mask_edit_target`,
`pending_lasso`, `edge_map`, `active_path`, `pdf_options`, `pdf_password`,
`ruler`, `opacity_preview_changed`, `fill_preview_changed`,
`mask_refine_preview`, `canvas_proxy`, or an in-flight filter job. The leak is
most visible on the in-place paths — Revert and the same-view `open`/`pdf_open`
paths reuse the view, as do Undo/Jump — because File > Open/New/Open Image/Open
As Smart Object/Duplicate and recovery create a fresh `PictureView`. After an
in-place restore, Reselect can restore the previous selection, a healing or mask
gesture can commit into the restored document, a mode-preview cancel can
overwrite an undone state, and a pending opacity/fill preview can record a
spurious history state. Undo cancels only a stroke (paint or mask), so a lifted
transform, filter preview, or mode preview can outlive the restore. The C++ tool
handlers keep their own `pressView_`, gesture, polygon, selection-move,
whole-layer move/clone, and typing state and can commit into the restored
document on the next release.

## What Changes

- Add one live-session slot, `ActiveSession`, on the backing state in phase 1,
  then move it with the document state into a plain-Rust `DocumentSession` in
  phase 2. The enum covers paint and mask strokes, healing, lasso and magnetic
  selection, selection move, transform, path gesture, filter preview, and mode
  preview. It has no Qt types and is exercised directly by `cargo test`.
- Replace the scattered live-gesture fields with the one slot and route every
  existing begin/cancel/commit entry point through it. Begins of a different
  kind are refused; update-style kinds (filter preview, mode preview, the
  magnetic/lasso pair) update the live session; Free Transform keeps its
  documented same-mode no-op and different-target/mode replacement.
- Make document replacement and history navigation cancel the live session and
  clear pending previews before installing or restoring state. Replacement
  clears the outgoing document-scoped pointers, pending preview flags, canvas
  proxy validity, and in-flight result guard, bumps the document generation,
  and then the entry point installs the incoming document, its PDF password
  when applicable, and the caller-built Open/Revert/New/Duplicate history. Undo,
  Redo, Jump, and snapshot restore cancel first and force a full display rebuild
  when the cancel changed the display.
- Add a `session_cancelled` notification and a `ToolHandler::onSessionCancelled`
  hook so every C++ handler resets its local gesture state on in-place
  Revert and Undo/Jump, not only on a tool switch. Type and whole-layer
  Move/Alt-clone stay C++-owned; the hook clears them without moving them into
  the Rust slot.
- End the live session on a tool switch according to the tool's existing
  contract: cancellable gestures (healing, lasso, magnetic, transform,
  selection move) are cancelled; tools that commit on deactivate (Brush, Pen,
  Path Selection, Type) keep committing. The slot is then cleared. This avoids
  changing documented commit-on-switch behaviour.
- Return a session change report that wraps #739's transaction `Effect` with the
  affected regions (optionally carrying the session-composited pixels) and a
  notification bit, and let the QObject adapter turn it into
  `changed`/`region_blitted`. Preserve the existing signal counts, including
  the region path's "blit without `changed`" contract and the region-plus-
  `changed` cases.
- Extract a plain-Rust `ViewFrame` for the level-0 frame, view pyramid, solo-mask
  planes, damage account, canvas/frame revisions, and the pixel-buffer side of
  cropping. The QObject keeps only `QImage` conversion, signals, interop, async
  jobs, and tool/preference state.
- Fix the reselect-after-replace defect and the other stale-slot leaks as a
  structural side effect, with regression tests, including a same-size
  replacement that proves the prior-selection memory is cleared.

Implementation is staged so each phase leaves the build green: session lifecycle
first, then document-state ownership and the edit transaction, then the display
frame. The change builds on #739 (`edit-transaction`), which must land first;
it uses #739's current `Edit::{None, Meta, Record, Regions(Vec<PsdRect>), Full,
Structural}` and `Effect::{Nothing, Notify, Blit, Rebuild}` and adds the
notification bit only at the session boundary.

## Capabilities

### New Capabilities

- `document/document-session`: the app-side contract for a document session —
  document replacement and history navigation cancel every live session, at
  most one live session exists, cancellation restores the pre-session document
  and selection, document-scoped state and pending previews are cleared on
  replacement, in-place restores reset C++ tool-handler state, tool switching
  ends the session according to its contract, stale async installs are
  discarded, and the change report drives display and notification.

### Modified Capabilities

None. `document/free-transform` already requires switching documents to cancel
the active transform, `document/edit-history` already requires undo to restore
bit-identically, and `tools/select-menu` already requires Reselect to be
disabled when nothing has been deselected since the document opened. This
change makes the code honour those existing requirements; no requirement text
changes.

## Impact

- `crates/pictura-app/src/cxxqt_object/`: new `document_session` and
  `view_frame` modules (under `cxxqt_object/` so existing `pub(super)` types
  remain reachable); `state.rs`, `impl_core`, `impl_history`, `impl_paint`,
  `impl_transform`, `impl_selection`, `paths`, `healing`, `filter_tools`,
  `image_adjust`, `open_jobs`, and `filter_tools/jobs` route through them.
- `crates/pictura-app/cpp/tools.cpp` and the Brush/Pen/Path Selection/Type/
  Move/Lasso/Healing handlers: the tool switch keeps `onDeactivate` and the
  C++ cleanup blocks, then clears the Rust slot; the new `session_cancelled`
  notification resets handler-local gesture state on in-place restore through
  `ToolHandler::onSessionCancelled`.
- Bridge: one new invokable (`cancel_active_session`) and one new signal
  (`session_cancelled`).
- Tests: new Rust `#[test]`s on `DocumentSession`/`ViewFrame`; new Qt Test cases
  for tool switching, region/notification counts, Revert/Undo and Path Selection
  mid-gesture, and replacement entry points; a pure Rust stale-async guard test;
  existing Qt suites and self-test tokens/exit codes are the behavioural oracle.
- No new dependencies. No `docs/` change. No change to the PSD format, history
  format, rendered pixels, or self-test exit codes.
