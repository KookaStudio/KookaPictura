# Proposal

## Why

The rule "a document has at most one live session" is enforced at runtime, not
represented in the types. `active_session_kind()`
(`crates/pictura-app/src/cxxqt_object/document_session/session.rs:255`) infers
the live kind from eleven independent `Option` fields on the 66-field
`PictureViewRust`, so the chain's order silently decides the winner when two
are set, and nothing in the type system forbids it. The C++ `ToolController`
keeps its own copy of the transform drag state (`transformDragging_`,
`transformHandle_`, 27 references across `tools.h`, `tools.cpp`, and
`tool_transform.cpp`), reset by hand at eight sites.

This is the half of #740 that was left behind. The archived change
`2026-10-21-document-session` designed the owning slot — "`ActiveSession` is an
enum whose variants own their data" — and then deferred it: "the live session
is a rule, not a nested type … the fields move in phase 2, together with the
document state they belong to". Phase 2 moved the document state into
`DocumentSession` and the per-kind begin rules onto the plain-Rust state —
`slot_allows` and `active_session_kind` are `PictureViewRust` methods — while
the live-gesture fields stayed flat on `PictureViewRust`.
`docs/dev/STATE.md` still records the deferred decision.

## What Changes

- Add `ActiveSession`: one owned value for the live session, held by
  `DocumentSession`, with a variant per session family — `Idle`, `Stroke`,
  `Heal`, `Selection`, `SelectionMove`, `Transform`, `Path`, `FilterPreview`,
  `ModePreview` — each variant holding that session's payload. The stroke
  variant covers the paint stroke and the mask stroke; the stroke's target
  decides which of the two kinds it reports.
- Move the 21 live-gesture fields out of `PictureViewRust` into the session
  module: the stroke family (the CPU stroke, the mask stroke, the GPU stroke
  and its placer, the reduced-level preview, the dirty tiles, the
  pending-present frame, the present-due flag, the pre-stroke document, the
  history label), the healing gesture, the lasso and magnetic selection, the
  selection move, the transform session, the path gesture, the filter preview
  (layer and mask), and the mode preview. The types a variant owns move with
  it.
- Read the session kind from the slot instead of inferring it:
  `active_session_kind()` becomes a read of the owned value, and
  `cancel_active_sessions` matches the one live variant instead of cancelling
  every field that might be set.
- Keep the per-kind begin rules in `slot_allows`. Only a begin takes the slot;
  a commit that does not own a session leaves the slot untouched, so a filter
  can still be committed while a transform is live.
- Make the C++ controller read the transform drag handle from the session over
  the bridge, and delete `transformDragging_`, `transformHandle_`, and their
  eight hand resets.

No edit rule changes: the begin, commit, and cancel rules, the history, the
pixels, the signals, and the self-test tokens stay as they are. The one
lifecycle change is that rebinding a canvas now ends a live transform session
instead of leaving it running with no controller reading it. What changes is
that the type system now rules out the states the rules have been defending
against.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-session`: one ADDED requirement, "The live edit session is
  one owned value". The existing requirement "At most one live edit session"
  states the behaviour and does not change; the new one states how the app
  holds the session so the behaviour is structural rather than conventional. It
  sits beside the sentence in that requirement that already keeps the Type
  tool's session C++-owned and outside this slot.

## Impact

- `crates/pictura-app/src/cxxqt_object.rs`: the bridge declares the new
  invokable.
- `crates/pictura-app/src/cxxqt_object/`: `state.rs` loses the live-gesture
  fields; `document_session/` gains `ActiveSession` in `session.rs` and the new
  `stroke/`, `selection.rs`, `previews.rs`, and `transform.rs` modules;
  `impl_paint`, `impl_paint/mask`, `impl_paint/mask_gpu`, `impl_filters`,
  `filter_tools`, `filter_tools/jobs`, `filter_tools/canvas_proxy`,
  `impl_selection`, `selection_move`, `magnetic`, `healing`, `paths`,
  `image_adjust/image_mode`, `impl_transform/bridge`, `impl_transform/dispatch`,
  `impl_transform/session`, `impl_transform/warp`, `impl_history`, `impl_core`,
  `paint_tools`, `paint_tools/fills`, `shapes`, and the canvas/profile tests
  route through the slot.
- `crates/pictura-app/cpp/`: `tools.h`, `tools.cpp`, and `tool_transform.cpp`
  lose the two mirror fields.
- Bridge: one new invokable, `transform_drag_handle`, declared in
  `crates/pictura-app/src/cxxqt_object.rs` and implemented in the transform
  session module. No new signal.
- Tests: new Rust tests on the slot's kind and exclusivity in
  `crates/pictura-app/src/cxxqt_object/document_session/tests.rs`, a new Qt
  case in `crates/pictura-app/cpp/tests/tst_document_session.cpp`, and the
  existing Rust and Qt suites as the regression oracle.
- `docs/dev/STATE.md` records the slot as an owned value instead of a rule
  over the gesture fields.
- No new dependencies. No change to the PSD format, the history format, the
  rendered pixels, the signal counts, or the self-test exit codes.
