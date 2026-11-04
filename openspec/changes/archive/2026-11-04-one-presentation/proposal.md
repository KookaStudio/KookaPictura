# Proposal

## Why

Several change-report shapes — `Edit`/`Effect`/`Applied`, `SessionChange`, `Restored`, `Refresh`/`Landed`, and `AdjustRefresh` — each carry a different shape and each has its own presenter. The rule "apply or blit regions, then emit at most one `changed()`" is copied across them, so the ordering invariant lives in folklore: a missed or reordered emission ships a stale canvas, and no single test can prove the rule for all producers.

This is candidate 2 of #1159 and the recommended first child because it has the highest locality: one value and one presenter concentrate the ordering rule without touching the composite engine or the C++ shell.

## What Changes

- Add one `Presentation` value: the regions whose pixels the producer already folded into level 0 and the pyramid, a rebuild kind, a notify flag, the dirty value to apply, a content-revision flag, and (for the committed edit transaction) the normalized edit declaration.
- Add one presenter, `PictureView::present`, backed by a Rust `present_core` seam, that owns the order: apply the dirty and revision bookkeeping, emit every region, perform at most one rebuild, then at most one notification.
- Make the five change producers return `Presentation`: the edit transaction, session cancellation, history restore, filter commit, and image-adjust preview poll.
- Delete the duplicated dialects: `Effect` and `effect_of`, `SessionChange`, `RegionBlit`, `SessionChange::merge`, `changed_display`, `Applied`, `Landed`, and `AdjustRefresh`. The report types are gone; the private worker payloads `WorkerRefresh` and `WorkerAdjustRefresh` survive because they carry worker-computed pixels into the presenter, and the rebuild kind replaces the old `Refresh` effect.
- Keep `Edit` as the mutation declaration and `Restored` as the history layer's report; convert both to `Presentation` at the app boundary. The edit transaction's presentation carries the normalized `Edit`, so callers keep the outcome they use today.
- Collapse `preview_layers_opacity` / `preview_layers_fill` and their two commits into one Rust helper returning `(i32, Option<Presentation>, Option<DisplayPreview>)`, keeping the Qt wrappers' signatures; the wrapper restores the pre-gesture preview after `present()`; route the display-preview fast path through the same presenter.
- Split `composite_full` into "make the document composite exact" (`composite_into_doc`) and the display reset (`reset_pyramid`), so producers never reset the display and the presenter owns the rebuild.
- Preserve observable canvas behaviour: the same regions, the same notification count, the same dirty and content-revision values, and the same dirty-star timing.

## Capabilities

### New Capabilities

- `document/presentation`: one change-report value and one presenter for the five document change producers, specifying bookkeeping-before-blit, region-before-notify ordering, one rebuild kind, and explicit dirty/revision bookkeeping.

### Modified Capabilities

- `document/document-session`: the change report becomes one `Presentation` with no pixel buffers, and the presenter replaces the adapter as the emitter.
- `document/edit-transaction`: the transaction returns the normalized outcome and presents through the one presenter, which owns the dirty and content-revision writes; the presentation contract moves to `document/presentation`.

## Impact

- `crates/pictura-app/src/cxxqt_object.rs` (module registration)
- `crates/pictura-app/src/cxxqt_object/presentation.rs` (new)
- `crates/pictura-app/src/cxxqt_object/presentation_tests.rs` (new)
- `crates/pictura-app/src/cxxqt_object/edit.rs`
- `crates/pictura-app/src/cxxqt_object/document_session/{session,adapter,active,tests}.rs`
- `crates/pictura-app/src/cxxqt_object/impl_history.rs`
- `crates/pictura-app/src/cxxqt_object/impl_core.rs`
- `crates/pictura-app/src/cxxqt_object/filter_tools/jobs/imp.rs`
- `crates/pictura-app/src/cxxqt_object/image_adjust/jobs/imp.rs`
- `crates/pictura-app/src/cxxqt_object/impl_layers/opacity_fill.rs`
- `crates/pictura-app/src/cxxqt_object/display_preview.rs`
- `crates/pictura-app/src/cxxqt_object/adjustment_edit/imp.rs`
- `crates/pictura-app/src/cxxqt_object/impl_paint.rs`
- `crates/pictura-app/src/cxxqt_object/impl_paint/tool_recording/imp.rs`
- `crates/pictura-app/src/cxxqt_object/impl_selection.rs`
- `crates/pictura-app/src/cxxqt_object/layer_arrange/imp.rs`
- `crates/pictura-app/src/cxxqt_object/paths/imp.rs`
- Rust tests in `edit.rs`, `document_session/tests.rs`, `filter_tools/jobs/imp/tests.rs`, `damage_policy_tests.rs`, `impl_paint/tests.rs`, `metrics_tests.rs`, `tool_recording_tests.rs`, `picture.rs`, `tests_profiles/*`, and `display_preview.rs`
- New MODIFIED deltas: `specs/document/document-session/spec.md`, `specs/document/edit-transaction/spec.md`
- The C++ shell is unchanged; the existing Qt tests remain the behavioural oracle.
