# Design

## Context

See `proposal.md` — Why. The current app bridge is one `qobject::PictureView`
whose methods both mutate `PictureViewRust` and emit `changed`/`region_blitted`.
The backing state has dozens of fields; live gestures occupy independent slots
with no shared lifecycle. `reset_edit_state` clears 17 direct fields plus
`reset_pyramid`, but leaves the document-scoped and pending-preview state that
the proposal lists. `restore` (`impl_history.rs:40`) cancels a paint or mask
stroke through `cancel_stroke`, but no other live session.
`ToolController::setActiveTool` (`cpp/tools.cpp:273`) asks handlers to
deactivate first, then runs the paint, selection-move, and transform cleanup
blocks by hand. That order is deliberate: Brush, Pen, Path Selection, and Type
commit on deactivate (`tool_brush.cpp:107-114`, `tool_pen.cpp:104-121`,
`tool_path_selection.cpp:50-62`, `tool_type.cpp:221-226`), while healing,
lasso, magnetic, selection move, and transform do not. Move's whole-layer
drag/Alt-clone and Path Selection are C++-owned gesture state that Rust cannot
reach.

`DocumentSession` must stay Qt-free: `QImage` overlay caches, GPU interop, async
jobs, tool preferences, and the mask display options stay in the adapter. The
change builds on #739's declared-outcome transaction. #739 currently defines
`Edit::{None, Meta, Record, Regions { rects, notify }, Full, Structural}` and
`Effect::{Nothing, Notify, Blit { notify }, Rebuild}`; this change must not
invent a second transaction or silently change those variant names.

## Goals / Non-Goals

**Goals:**

- One live-session slot and one cancellation path for document replacement,
  history navigation, and tool switching.
- Document state, history, pending previews, and live sessions driven directly
  by `cargo test`.
- A headless display frame whose region updates equal a full rebuild.
- Preserve pixels, history states, signal counts, self-test tokens, and exit
  codes.

**Non-Goals:**

- Redesigning the paint raster, history storage, compositing, or the PSD format.
- Moving `QImage` overlay caches, `InteropState`, async jobs, or tool
  preferences into the session.
- Moving Type tool editing out of C++. It stays a C++-owned session; only its
  stale handler state is reset on in-place restore.
- Moving the whole-layer Move/Alt-clone gesture into Rust. It stays C++-owned and
  is reset through the handler hook.
- Changing documented commit-on-deactivate tool switching into cancellation.
- Redesigning C++ tool handlers beyond the common reset hook.

## Decisions

### Staged ownership: the session, then the frame

The session came first: `DocumentSession` owns the document, the selection, the
history, and the PDF options, and the history and edit transactions live on it.
The display caches move into a `ViewFrame` afterwards, in #783; that work is
out of this change's scope, so its scenarios are not in the spec delta.

The staging kept every step green: the slot and the cancel path landed before
the fields moved, so no half-moved `DocumentSession` could not implement
`replace_document` or `undo`.

### One enum, with explicit update and composed begins

`ActiveSession` is an enum whose variants own their data:
`Stroke(StrokeSession)`, `Heal(HealStroke)`, `SelectionGesture`,
`SelectionMove`, `Transform`, `PathGesture`, `FilterPreview`, `ModePreview`.
`StrokeSession` absorbs the pixel and mask stroke fields and the present state.
`PendingPreviews` is separate: it owns `opacity_preview_changed`,
`fill_preview_changed`, and `mask_refine_preview`; these are cleared on
replacement and history restore even though they are not cancellable gestures.

The begin rules are per kind, not one global matrix:

| Entry point | Rule |
|---|---|
| `begin_paint` | Resolve the mask/layer target first; acquire one `Stroke` slot. A second stroke begin refuses. It never nests `begin_mask_paint` as a second begin. |
| `begin_mask_paint` | Internal target used after `begin_paint` resolves a mask; not a standalone second slot. |
| `healing_begin` | Refuse if any session is live. A repeated heal begin while heal is live refuses. |
| `magnetic_begin` | Create the `SelectionGesture` and its edge map. |
| `begin_lasso` | If a `SelectionGesture` is live, attach/clear its points and mode; otherwise create one. It is an update, not a second slot. |
| `begin_selection_move` | Refuse if any session is live; a repeated begin refuses (does not silently replace the origin). |
| `begin_transform_*` | Same path + same mode: no-op. Same path + different mode or different path: cancel the old session and start the new one. |
| `path_gesture_begin` | Refuse if a path gesture is live; snapshot the Work/Saved path or, for a shape-layer target, the pre-gesture document. |
| `filter_preview*` | Update the live `FilterPreview`; the first call captures the baseline. Repeated calls do not create a second session. |
| `preview_indexed` | Update the live `ModePreview`; the first call captures the base document. Repeated calls do not create a second session. |

A refused begin must leave the document bit-identical. In particular, a
transform begin that trims the target and then fails to build its preview or
read its rectangle must restore the pre-trim (and pre-lift) document.

### Modules live under `cxxqt_object`

```
crates/pictura-app/src/cxxqt_object/
  document_session/
    mod.rs          DocumentSession, SessionChange/RegionBlit, replace/edit/undo
    state.rs        fields, Default, accessors, document metadata, pending previews
    session.rs      ActiveSession, begin/cancel/commit dispatch
    stroke/
      mod.rs        StrokeSession, target, commit/cancel
      paint.rs      layer stroke and present state
      mask.rs       mask stroke
      preview.rs    reduced-level preview
      gpu.rs        GPU stroke dab glue
    selection.rs    selection gesture and move
    previews.rs     filter and mode preview
    tests.rs
  view_frame/
    mod.rs          ViewFrame, Blit, damage, revisions, RebuildOutcome
    mask.rs         solo-mask plane and pyramid
    tests.rs
```

Keeping them under `cxxqt_object` preserves the existing `pub(super)` visibility
for `TransformSession`, `MaskStroke`, `PreviewStroke`, `FilterPreview`,
`ActivePath`, and the composite helpers. A sibling module at `src/` would need
those types and fields promoted to `pub(crate)`.

### No `Deref` shortcut for the field move

The document/selection/history accesses and the `PictureViewRust { … }`
construction literals are renamed mechanically. A
`Deref<Target = DocumentSession>` would keep the call sites unchanged but hides
the seam and makes IDE/compiler errors resolve through two types. Rejected.

### The live session is a rule, not a nested type

Phase 1 keeps the live gesture fields (`stroke`, `mask_stroke`, `heal_stroke`,
`transform_session`, `path_gesture`, the lasso points, the previews) where they
are and adds the slot as a *rule* over them:

```rust
fn active_session_kind(&self) -> Option<SessionKind>   // inferred from the live state
fn slot_allows(&self, kind: SessionKind) -> bool         // the per-kind begin rule
```

A begin of another kind is refused; a repeat begin follows its kind's rule (a
transform replaces its own session, a filter or mode preview updates it, a
lasso attaches to the magnetic gesture it started, and a healing,
selection-move, path, or paint begin is refused). Every begin path consults
`slot_allows` before it touches the document, and one `cancel_active_sessions`
cancels all of them.

A nested `ActiveSession` enum would own the payloads, but that move is the same
churn as the document-state move and buys no behavior the rule does not already
give. The fields move in phase 2, together with the document state they belong
to. `PendingPreviews` is already one struct: the three panel-preview flags
travel together and are cleared together.

### The session module is split in two

`cxxqt_object/document_session/session.rs` is the plain-Rust core: the slot
rules, the cancel path, the replacement, and the change report. It imports no
Qt type. `cxxqt_object/document_session/adapter.rs` is the Qt half: it turns the
session's report into `region_blitted`, `changed`, and `recomposite`. The
phase-2 check that the session names no `cxx_qt` therefore applies to
`session.rs`; the adapter is Qt by definition.

### Cancel before replace or restore

`replace_document` and every history navigation call `cancel_active()` first.
Cancelling a mask stroke, a lifted transform, a shape-layer path gesture, or a
filter preview mutates the outgoing document; installing the new document or
restoring a snapshot first would let that mutation touch the wrong state. If
cancellation reports a display change during history navigation, the restore
requests a full rebuild rather than a union of regions. `cancel_active` also
clears `PendingPreviews`; history restore must not leave a pending opacity/fill/
mask-refine flag to be recorded later.

Replacement distinguishes **outgoing** from **incoming** state: it clears the
outgoing selection, prior-selection memory, history, pointers, pending previews,
ruler, PDF options, and canvas-proxy validity and bumps `document_generation`;
the calling entry point then installs the incoming document and a history it
built (Open/Revert state, one New state, one Duplicate state).
`replace_document` is install-only so `new_document` and `duplicate_into` cannot
double-record. The incoming PDF password is installed for a protected PDF
Open/Revert and cleared for New, Duplicate, or a non-PDF incoming document.

### Cancel and Path Selection restore

`PathGesture` stores a `PathRestore`:

- `WorkOrSaved { active, path }` for a Pen/Work/Saved path edit.
- `Layer(Box<Document>)` for a shape-layer path edit or a Path Selection drag.

`tool_path_selection.cpp` must begin the gesture at press (not only on release)
and abort it on Escape/cancel; otherwise Rust has nothing to restore. The
`Path Selection` command remains commit-on-deactivate, so `onDeactivate` commits
and ends the gesture, while Revert/Undo cancels it through `cancel_active`.

A Work or Saved path is restored from the gesture's backup, which changes no
pixel: the abort reports a notification. A shape layer's outline lives in the
layer, so the pre-gesture document backup is the only way to restore it. Until
that backup lands in phase 2, an abort on a shape-layer path asks for a rebuild
instead of a notification, because the layer's pixels changed.

### Tool switching ends the session according to its contract

`ToolController::setActiveTool` keeps calling `old->onDeactivate` first. Brush,
Pen, Path Selection, and Type commit there, preserving documented behavior. The
existing C++ cleanup blocks stay and run in their current order, so
`transformDragging_`, `transformHandle_`, the canvas preview,
`transformChanged`, and the selection-move flags are still reset. After those
blocks, `cancel_active_session()` clears any remaining Rust session (healing,
lasso, magnetic, previews) as the single safety net.

### In-place restore resets all handler state

`ToolController` cannot reach `pressView_`, `gesture_`, polygon, typing, or
Move's `pressed_`/`pendingDuplicate_`/`duplicated_` fields. Add a virtual
`onSessionCancelled()` to `ToolHandler` (default no-op), implement it in Pen,
Path Selection, Type, Move, Lasso, and Healing, and have `ToolController` call
it for every registered handler on a `session_cancelled` notification.
`PictureView` emits `session_cancelled` unconditionally from every replacement
and history restore, not only when a Rust session was live, so the Type and Move
C++ sessions are reached too. `PicturaMainWindow::wireCanvas` connects it per
view. This signal is also the only safe way to stop a Move Alt-clone or a Path
Selection drag before the next pointer release.

### Reuse #739's transaction and effect report

The session's `edit(label, mutation)` takes #739's `Edit`. The session adds a
`SessionChange`:

```rust
struct RegionBlit {
    rect: PsdRect,
    srgb: Option<PixelBuffer>, // None: adapter composites; Some: use these pixels
}

struct SessionChange {
    effect: Effect,           // #739: Nothing | Notify | Blit { notify } | Rebuild
    regions: Vec<RegionBlit>, // payload for Blit; empty means no blit
}
```

#739 still owns the transaction, history capture, and the signal policy,
including the per-region `notify` bit. The session reuses `Effect::Notify` for
notification-only cancellations and `Effect::Blit { notify: true }` for region +
`changed` commits such as healing, patch, red-eye, and batch property. There is
no separate session-level notify parameter and no `edit_notify`; a mutation
declares `Edit::Regions { rects, notify: true }` when it needs both.

Adapter dispatch order:

1. `Nothing`: no display and no notification.
2. `Notify`: emit exactly one `changed`.
3. `Blit { notify }`: emit the non-empty region blits, then one `changed` when
   `notify` is true.
4. `Rebuild`: rebuild the frame, then one `changed`.

In phase 1 `RegionBlit.srgb` is `None` and the adapter composites. In phase 2
the session fills it and the adapter only premultiplies/patches. #739's
`Edit::Regions` remains rectangles only; the buffer lives in the session wrapper.

### Pending previews, document generation, and async installs

`PendingPreviews` clears on replacement and history restore. `document_generation`
is bumped by every install: `replace_document`, `new_document`, `Duplicate`,
`Trim`, `apply_opened`, and `finish`. `canvas_proxy` keys on
`document_generation` as well as `content_revision`. `filter_jobs` and
`open_jobs` carry a request/generation id; an install whose id no longer matches
is dropped. The stale-async test exercises the pure guard
(`apply_if_current`/`commit_finish` with a mismatched id) rather than trying to
run a real job headlessly. A Qt test with the existing open-delay hook covers
the view-level path.

### `gpu_compute` moves with the document

The backend choice affects `doc.composite`, which the session owns. It moves as
a render option in phase 2. Passing it to every call would be noisy; leaving it
in the adapter would let session tests depend on adapter state.

### The frame is #783's design

The next two sections describe the display frame, which #783 carries. They are
kept here because the session hands the frame its pixels; the frame itself is
not part of this change.

### Preview state stays in the stroke session; the frame paints it

`PreviewStroke` holds samples that the commit must replay, so it belongs to the
stroke session. It needs `level0`/`pyramid` only when it is built; begin
receives a read-only `PreviewSource`. Dabs return `PreviewPresent { level,
rect, rgba, scale }`; `ViewFrame` patches the level and returns the blit.

`ViewFrame::rebuild` needs to know the live source, and it must return enough
for the adapter to clear session state:

```rust
enum LiveStrokeSource<'a> { Layer(&'a Document), Mask(&'a Document) }

struct RebuildOutcome {
    preview_invalidated: bool,
    presents_invalidated: bool,
    mask_solo_missing: bool,
}

fn rebuild(&mut self, doc: Option<&Document>, live: Option<LiveStrokeSource<'_>>,
           gpu: bool, mask_solo: Option<&str>) -> RebuildOutcome;
```

The adapter applies the outcome to the session: drop the preview, clear
`gpu_stroke`/`gpu_placer`/pending present/tiles, and clear a missing `mask_solo`
target. `ViewFrame` never mutates the document and never converts to `QImage`.

### ViewFrame does not composite or mutate the document

`refresh_region_buffer` composites and patches `doc.composite`; that stays in
the session. `ViewFrame` receives the resulting `RegionBlit` and folds the
straight-sRGB pixels into level 0, the pyramid, damage, and mask-solo planes. It
owns `level0`, both pyramids, `damage`, `canvas_revision`, and `frame_revision`
only. `QImage` conversion (`display_crop`, `canvas_present_image`, `image`) stays
in `impl_core/display.rs`.

### History navigation reports through the same adapter

`restore` already returns `Restored::{Region, Everywhere}`. The session maps
that to `SessionChange` internally; the public QObject `undo`/`redo`/`jump`
booleans stay unchanged. When a cancel reported a display change, the restore
uses `Rebuild` so a stale region cannot survive.

### App-level state stays in the adapter

`InteropState`, `filter_jobs`, `recovery_write`, open-job state,
`move_base`/`move_layer` and the transform overlay images, `canvas_proxy`,
`color_policy`, `add_copy`, `use_default_masks`, `last_filter`, `filter_error`,
`shape_attributes`, `mask_display`, `mask_overlay_revision`, and
`document_generation` are not document state. Move/Path Selection/Type gesture
fields stay in their C++ handlers and are reset by `onSessionCancelled`.

### One tool-switch invokable and one cancellation signal

`PictureView::cancel_active_session()` wraps `DocumentSession::cancel_active`
and adapts the change. `PictureView` also exposes `session_cancelled` for the
C++ handler reset. This is the one bridge surface addition; no second bridge.

### File size

Files stay under the 800-LOC preference. The stroke code is split into
`stroke/{mod,paint,mask,preview,gpu}.rs`; the current `impl_paint.rs` (1077),
`mask.rs` (531), and `mask_gpu.rs` (117) cannot fit one file under the cap.

## Risks / Trade-offs

- **Signal-count drift** → `notify` bit plus selftest 363, the new
  region+notify tests, and `tst_layers_preview_coalescing`.
- **#739 contract mismatch** → use its current `Record`/`Regions` names and
  `Effect`; do not rename or extend the enum. The session wrapper is additive.
- **Mechanical rename size** → one compiler-guided commit, no `Deref`; keep the
  rename separate from behavior changes.
- **Paint hot-path regression** → phase 3 lands last; `scripts/profile-tools.py`
  before/after and byte comparisons against the pre-change build.
- **Preview/present invalidation on a frame rebuild** → `RebuildOutcome` and the
  `reset_pyramid_drops_a_live_preview` /
  `reset_pyramid_rebuilds_and_clears_damage` tests.
- **C++ handler state on Revert/Undo** → `ToolHandler::onSessionCancelled` and
  Qt tests for Pen, Path Selection, content move, Move/Alt-clone, and Type.
- **Path Selection live edits** → begin the gesture at press and snapshot the
  shape document; test a cancelled drag.
- **Async races** → request/generation ids on open and filter installs; a pure
  guard test plus the Qt open-delay test.
- **Cancel-before-swap ordering** → regression tests replace during a stroke,
  mask stroke, lifted transform, filter preview, mode preview, and shape-path
  gesture.
- **Review size** → three commits/phase in one change. If phase 3 is split, it
  must be a separate OpenSpec change with its own capability and test plan; this
  change cannot be left open under the zero-open rule.
- **#739 not merged** → do not start implementation before the transaction
  lands; otherwise two commit pipelines coexist.

## Migration Plan

1. **Phase 0** — land #739 with its current `Edit`/`Effect` contract.
2. **Phase 1** — the live-session slot, `cancel_active`, `replace_document`, and
   history cancel-first; pending-preview/cache/generation clearing; the C++
   `onSessionCancelled` hook; begin routing through the slot; stale-state and
   Reselect regressions.
3. **Phase 2** — move the document state, the pending previews, and the
   transaction into `DocumentSession`; rename the access sites; add the open
   request id; add the headless session tests.
4. **Phase 3 (#783)** — extract `ViewFrame`; move the display caches and the
   PixelBuffer presentation; keep the `QImage` conversion and the signals in
   the adapter; move the canvas tests.

Each phase is independently green and revertible. No persisted state, format, or
compatibility shim is involved. Rollback is a commit revert.
