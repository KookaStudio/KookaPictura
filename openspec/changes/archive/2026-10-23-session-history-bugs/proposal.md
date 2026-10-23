# Proposal: Session and history data-loss bugs

## Why

Five session/history paths lose or misreport document state, and the
OpenSpec corpus currently pins the broken behavior in four places. The
behavioral contract in `docs/` already describes the intended model:

- `docs/10-workflow-io/document-lifecycle.md` says dirty state is driven by
  the undo stack revision and that Revert is one undoable history state.
- `docs/02-ui-ux/panels/history-panel.md` says `File > Revert` is added as a
  history state and that states and snapshots survive a revert.
- `docs/02-ui-ux/menus.md` says inapplicable commands are disabled.

`#827` (single session slot) and `#828` (edits in the transaction) have landed,
so these fixes build on the current session architecture.

## What changes

- #850 — `Image > Trim` crops the live document in place and records one
  `Trim` state on the existing history; it no longer replaces the document or
  discards prior states and named snapshots.
- #851 — `is_dirty()` follows the saved history state: undo, redo, and history
  jump mark the document modified unless they land on the saved state, and
  returning to the saved state clears the mark again. A snapshot restore appends
  a new state, so it marks the document modified.
- #852 — restoring a named snapshot appends one current history state, so Undo
  returns to the pre-restore state and Redo returns to the snapshot.
- #853 — a pending lifted transform refuses the direct document-editing
  commands (Edit > Fill/Stroke, filters, adjustments, image operations,
  clipboard edits), and the Layer/Image/Filter menus are disabled for any
  pending transform, so a commit or cancel can never discard an edit.
- #854 — `File > Revert` decodes before changing the document: a failure keeps
  the document, pixels, dirty state, path, history, and any live session; a
  success preserves the file identity (path, PDF password/options, source
  format), the existing history, and named snapshots, and appends one undoable
  `Revert` state.

## Capabilities

- Modified: `document/edit-history`
- Modified: `document/document-session`
- Modified: `document/document-lifecycle`
- Modified: `document/free-transform`

## Impact

- History/session: `crates/pictura-app/src/history.rs`,
  `cxxqt_object/document_session/session.rs`, `cxxqt_object/impl_history.rs`,
  `cxxqt_object/impl_core.rs`, `cxxqt_object/state.rs`,
  `cxxqt_object/impl_core/recovery.rs`.
- Revert/open: `cxxqt_object/open_jobs.rs`, `cxxqt_object/pdf_export.rs`,
  `cxxqt_object/impl_core.rs`, `cpp/frame.cpp`, `cpp/frame_menus.cpp`,
  `cpp/api/document_api.*`.
- Trim: `cxxqt_object/image_adjust/image_ops.rs`.
- Transform gate: `cxxqt_object/document_session/session.rs`,
  `cxxqt_object/state.rs`, `cxxqt_object/impl_transform/session.rs`,
  `cxxqt_object/impl_transform/dispatch.rs`,
  `cxxqt_object/paint_tools/fills.rs`, `cxxqt_object/impl_filters.rs`,
  `cxxqt_object/filter_tools/jobs.rs`, `cxxqt_object/clipboard.rs`,
  `cxxqt_object/impl_pictura_raw.rs`,
  `cxxqt_object/image_adjust/image_size.rs`,
  `cxxqt_object/image_adjust/image_mode.rs`,
  `cxxqt_object/impl_selection.rs`, `cpp/frame_transform_lock.cpp`,
  `cpp/tool_transform.cpp`.
- Tests: `selftest_session`, `tst_image_ops`, `tst_document_session`,
  `tst_document_api`, `tst_transform_lock`, `tst_recovery`, `tst_import_pdf`,
  `tst_save_pdf`, plus the Rust state tests in `pictura-app`.

## Non-goals

- No change to the 20-state / 10-snapshot bounds.
- No new dependency.
- No change to successful Open/New/Duplicate replacement semantics.
- No change to transform geometry, interpolation, or the existing Layer-menu
  lock; the change only adds commands to the locked set and leaves the Move
  tool's controls box unlocked.

## Issues

Closes #850, #851, #852, #853, #854.
