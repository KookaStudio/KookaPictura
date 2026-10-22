# Design

## Context

See `proposal.md` — Why. The rule over the live gesture fields shipped with
#740 and works: every begin consults `slot_allows`, the per-kind rules are
honoured, and `cancel_active_sessions` cancels whatever is set. What it cannot
do is make two live sessions impossible, and the session's own state is spread
over 21 fields on a 66-field struct that the session module does not own.

The capability is `document/document-session`; the change adds one requirement
to it, "The live edit session is one owned value", beside the existing "At
most one live edit session" that states the behaviour.

Three constraints shape the approach.

The session module is Qt-free. `document_session/session.rs` must name no
`cxx_qt` type, so every payload the slot owns has to be a Qt-free type living
in a Qt-free module. `state.rs` is not one: it imports `cxx_qt_lib::QImage`
for the move-preview images (`move_base`, `move_layer`), which stay in the
adapter.

The paint hot path is real. The dab loop reads the live stroke and the
document at the same time, so the slot's accessors must not widen the borrow
past the slot.

The change is a representation change. The begin, commit, and cancel rules,
the history, the pixels, the signals, and the self-test tokens do not move.

## Goals / Non-Goals

**Goals:**

- One owned value for the live session, with a variant per session family, so
  the kind is read and not inferred.
- One owner per session's state, including the paint session's present, GPU,
  and preview state.
- One cancel path that cancels the one live session rather than every field
  that might be set.
- The C++ controller reads the transform drag state over the bridge and keeps
  no copy of it.

**Non-Goals:**

- Changing any begin, commit, or cancel rule. The rules move, they do not
  change.
- Moving the Type tool's session into Rust, or the whole-layer Move and
  Alt-clone gesture. Both stay C++-owned and are reset by the in-place restore
  hook.
- Moving the adapter's state — `InteropState`, the filter jobs, the
  move-preview `QImage`s, `canvas_proxy`, the tool preferences — into the
  session.
- Reorganising `document_session` beyond the files this change touches.

## Decisions

### The slot owns the payload, not a rule over the fields

`ActiveSession` is an enum whose variants own their data:

```rust
enum ActiveSession {
    Idle,
    Stroke(Box<StrokeSession>),
    Heal(Box<HealStroke>),
    Selection(Box<SelectionGesture>),
    SelectionMove(Box<SelectionMoveState>),
    Transform(Box<TransformSession>),
    Path(Box<PathRestore>),
    FilterPreview(Box<FilterPreviewSession>),
    ModePreview(Box<Document>),
}
```

The alternative is the shape #740 shipped: no slot at all, just a rule over the
independent gesture fields, with `active_session_kind()` inferring the kind
from them. It is a smaller diff, but it leaves two payloads settable at once
and the chain's order deciding the winner. That is the gap this change closes,
so the rule-only shape is rejected.

`SessionKind` stays as a `Copy` discriminant and `ActiveSession::kind()` maps
to it. The payload enum cannot be `Copy`, and the rules need a kind-only
argument before the payload exists, so the kind vocabulary is kept separate.
The mapping is one exhaustive match over the variants, returning
`Option<SessionKind>`: `Idle` is `None`, and every payload variant reports the
kind it holds — except the stroke variant, which reports one of two.

| `SessionKind` | `ActiveSession` variant | How the kind is decided |
|---|---|---|
| `Paint` | `Stroke` | the stroke payload holds the layer's paint stroke |
| `MaskPaint` | `Stroke` | the stroke payload holds the mask's coverage stroke |
| `Healing` | `Heal` | the heal payload is present |
| `SelectionMove` | `SelectionMove` | the move payload is present |
| `Transform` | `Transform` | the transform payload is present |
| `Path` | `Path` | the path restore is present |
| `Lasso` | `Selection` | the selection gesture holds the lasso points, the lasso mode, or the edge map |
| `FilterPreview` | `FilterPreview` | the preview payload holds the layer preview or the mask preview |
| `ModePreview` | `ModePreview` | the mode preview's base document is present |

Nine kinds, eight payload variants. The stroke variant is the only place where
the kind is not simply "the variant is present": it holds the layer's paint
stroke and the mask's coverage stroke, and the kind is decided by which of the
two it carries. The two are mutually exclusive by construction — the mask
target is resolved before the stroke begins — so the payload never holds both.
The filter-preview variant likewise carries either the layer preview or the
mask preview, but both are the same kind, so the variant's presence is the
kind.

### The payload moves with the type it belongs to

`ActiveSession` and `SessionKind` live in `document_session/active.rs`.
`TransformSession` and its mode types (`TransformMode`, `RepeatTransform`,
`WarpDrag`) move to `document_session/transform.rs`; `MaskStroke` to
`document_session/stroke/mask.rs` and `PreviewStroke` to
`document_session/stroke/preview.rs`; the new `SelectionGesture` (the
lasso points, the lasso mode, and the edge map) to
`document_session/selection.rs`; `FilterPreview`, `FilterPreviewSession`, and
`PreviewBaseline` to `document_session/previews.rs`. `state.rs` keeps the
frame, the move-preview images, the layer target, and the adapter's state.

The alternative is leaving the payload types in `state.rs` and importing them.
It would not break the Qt-free check — a Qt-free type stays Qt-free wherever it
lives — but it splits the ownership this change is about: `state.rs` would
keep holding the state the session owns, and a reader of the session module
would have to follow a hop into the adapter to find the session's own types.
The moved types' fields are widened either way, so the move costs nothing
extra.

Moving a type changes what `pub(super)` means: in `state.rs` it reaches
`crate::cxxqt_object`, but in `document_session/` it reaches
`crate::cxxqt_object::document_session`. The moved types' fields are read and
written from the whole `cxxqt_object` tree, so they are widened to
`pub(in crate::cxxqt_object)` as they move.

### The kind stays where the callers read it

`active_session_kind()` keeps its name and its `Option<SessionKind>` signature
and stays reachable from `PictureViewRust`; it becomes a read of the slot
(`self.session.live.kind()`). `has_active_session()` keeps its shape too. The
23 existing session tests assert on both (`Some(SessionKind::Healing)`,
`Some(SessionKind::FilterPreview)`, …) and keep their assertions unchanged.

### The accessors borrow the slot, not the session

This is the constraint the paint loop runs into. `impl_paint.rs` reads the
live stroke and the document in one expression:

```rust
match (rust.session.live.stroke_mut(), rust.session.doc.as_mut()) { … }
```

The accessors must therefore live on `ActiveSession` and be reached through the
field path, so each borrow is of `rust.session.live` and stays disjoint from
`rust.session.doc`:

```rust
impl ActiveSession {
    fn stroke_mut(&mut self) -> Option<&mut Stroke> { … }
}

let (Some(s), Some(doc)) = (rust.session.live.stroke_mut(), rust.session.doc.as_mut());
```

An accessor written as `&mut self` on `DocumentSession` borrows the whole
session and breaks every site that needs the document at the same time. This
is the most likely way the refactor stalls; it is called out in `tasks.md` and
reviewed for in phase A.

The slot exposes its payloads through accessors on `ActiveSession`, so a test
can ask whether the state of another kind is present: `stroke_session_mut`,
`stroke_mut`/`mask_stroke_mut`, `heal_stroke_mut`, `selection_gesture_mut`,
`selection_move_origin`/`selection_move_delta`, `transform_mut`, `take_path`,
`filter_preview`/`mask_filter_preview`, and `mode_preview`. Each returns `None`
unless the matching variant is the live one.
The stroke variant covers both stroke kinds, so `stroke_session_mut` hands back
the whole stroke session and the caller takes the layer stroke or the mask
stroke out of it — `s.stroke.as_mut()` in the paint loop — which keeps the hot
path's borrow of the slot and of the document disjoint.

### Only a begin takes the slot

`impl_filters.rs` gates the preview but not the commit: a filter can be
committed while a transform is live, and the transform stays live. The enum
must not change that. A commit either ends the session it owns — the filter
commit takes the live filter preview out of the slot and puts the slot back to
idle — or it does not touch the slot at all. Routing a commit through the slot
for symmetry would silently start refusing commits CS6 allows.

### The cancel path becomes a match

`cancel_active_sessions` today cancels the mode preview first, because its
restore replaces the whole document and would otherwise discard an in-place
stroke or transform cancel. With one live variant that ordering hazard is gone:
the cancel matches the one live session and cancels it. The report it returns
is the same one that kind's cancel returns today, so the adapter's effect and
regions do not move.

### The paint session absorbs its present state

The stroke family is one gesture with a target and a backend choice: the CPU
stroke or the mask stroke, the GPU stroke and its placer or the reduced-level
preview, plus the dirty tiles, the pending-present frame, the present-due
flag, the pre-stroke document, and the history label. `StrokeSession` owns all
of it.

`reset_pyramid` currently tears six of those fields down by hand on a full
rebuild. Under the session it becomes one call on the live session. Leaving
them on `PictureViewRust` would keep the split this change removes.

### The big variants are boxed

`Document` carries a 768-byte inline palette, and `TransformSession` holds one
for its lift backup. Boxing every payload variant keeps `DocumentSession` small
enough to live inline in `PictureViewRust`. The dab loop gains one pointer hop;
`scripts/profile-tools.py --compare` is the guard.

### The controller reads the drag handle over the bridge

`TransformSession` already tracks `handle` and `dragging`, and the C++ copy
agrees with it today: `transform_press` sets `handle` to the hit and `dragging`
to true, and `transform_release` sets `handle` back to -1 and `dragging` to
false. One accessor, `transform_drag_handle`, reports the dragged handle, or
-1 when no drag is live, which is what `dragging == false` means today. The
two C++ fields and their eight hand resets go.

The C++ still has to know whether a drag is live: `setTransformCursor` falls
back to a hover hit-test when it is not, and the move handle's cursor is the
closed hand during a drag and the open hand otherwise. Both follow from the
accessor's value — a handle of -1 means no live drag — so the flag the cursor
code needs is derived from the accessor, not stored beside it.

The two copies agree today at all eight hand resets, and they must keep
agreeing once the copy is gone. The tool switch cancels the Rust session
before it clears the C++ fields; the commit and the cancel take the Rust
session with them; the begin clears the C++ fields because the new Rust
session starts with no handle and no drag; the warp enter and the warp leave
clear `dragging` and the handle on the Rust side; the release clears both. The
one site that does not cancel the Rust session is
`ToolController::unbindCanvas`, which clears the C++ fields and drops the
canvas. Today the Rust session survives that with its handle still set, and
the two copies disagree — harmlessly, because the C++ copy is the one the
cursor and move code reads. With the copy gone, the controller would read the
live handle and believe the drag survived the rebind. `unbindCanvas` therefore
cancels the Rust transform session, which is what it already means by clearing
the transform preview.

The alternative — keeping the C++ copy — is the duplication this change
removes. The alternative of a separate "is dragging" boolean alongside the
handle is one more field to keep in step for no gain: the Rust side already
sets the handle to -1 exactly when the drag ends.

## Risks / Trade-offs

- **The borrow width** → the accessors live on `ActiveSession` and are reached
  through the field path; the compiler rejects the alternative, and the rule
  is stated in the design so the reviewer looks for it.
- **The C++ copy and the session disagree** → each of the eight hand resets is
  checked against the Rust side it mirrors, and `unbindCanvas`, the one site
  that cleared the C++ copy without cancelling the Rust session, cancels it
  instead.
- **A clear that clears the wrong kind** → the enum makes each kind's clear
  explicit, and the existing Rust suite plus the session tests are the oracle.
  `clear_document_scoped_state` and `reset_pyramid` are the two hand-clears
  that need the most care.
- **A visibility slip in the move** → a moved type's `pub(super)` reaches
  `document_session`, not `cxxqt_object`; the moved fields are widened to
  `pub(in crate::cxxqt_object)` as they move, and the compiler names every
  one that is missed.
- **The paint hot path regresses** → `scripts/profile-tools.py --save` before
  the change and `--compare` after it; the dab loop gains an enum match and a
  pointer hop.
- **A semantic slip in the `Option` → enum conversion** → the compiler-guided
  part is the type change; the semantics are checked by the existing paint,
  retouch, path, selection, transform, and filter suites plus the session
  tests.
- **File size** → the module split keeps every file under the 1200-LOC cap;
  `scripts/check-file-size.sh` gates it. Ten of the affected files are
  already over the 800-LOC target (`cxxqt_object.rs` 1192, `impl_core.rs` 1123,
  `impl_paint.rs` 1104, `filter_tools.rs` 1098, `impl_selection.rs` 1077,
  `impl_filters.rs` 1068, `paint_tools.rs` 1044, `impl_transform/session.rs`
  1023, `tests_profiles.rs` 950, `canvas_view_test.rs` 830); the first has
  only eight lines of headroom for the one new bridge declaration.
- **The Qt-free check** → every payload type moves into a Qt-free module
  under `document_session/`, and `session.rs` names only those.
- **Review size** → two phases, two commits, one PR. Phase A is the large one;
  phase B is small and lands on a green base.

## Migration Plan

1. **Phase A — the slot owns the payloads.** Add `ActiveSession` and the
   session's accessors; move the 21 fields and the types they own into the
   session module; read the kind from the slot; turn the cancel path into a
   match; keep `slot_allows` and the per-kind rules as they are.
2. **Phase B — the controller keeps no copy.** Add the bridge accessor; read
   the drag handle from the session; delete the C++ fields and their resets.

Each phase is independently green and revertible. No persisted state, format,
or compatibility shim is involved. Rollback is a commit revert.

## Open Questions

None.
