# Design

## Context

See `proposal.md` — Why, and `specs/document/document-session/spec.md` for the
requirements. This section records only the constraints that shape the approach.

**The preview fields are shared, not transform-private.** The twenty fields
(`move_base`, `move_base_origin`, `move_view`, `move_view_pixels`, `move_layer`,
`move_x`, `move_y`, `move_layer_offset`, `move_opacity`, `move_above`,
`move_above_origin`, `move_above_scale`, `move_above_presented`,
`move_prepared_revision`, `move_prepared_layer`, `move_prepared_union`,
`move_preview_cache_hit`, `move_drag_active`, `mask_move_baseline`,
`transform_preview_level`) are written by five consumers: the Free Transform
session, the Move drag (`compute_move_preview`), the mask move
(`render_mask_move_preview`), the Alt-duplicate selection move
(`begin_selection_duplicate_preview`), and the warp refresh.
`impl_core/deferred_composite/imp.rs` also reads `move_drag_active`.
"One teardown" therefore has to serve all five, not just the transform.

**`document_session/session.rs` names no `cxx_qt` type.** No script enforces it,
but the archived design of `2026-10-22-document-session-slot` records the
decision that the `QImage` move-preview fields stay in the adapter. `state.rs`
and `document_session/adapter.rs` are the Qt half; `move_base` and `move_layer`
are `Option<QImage>`, so the fields stay on `PictureViewRust`.

**`impl_transform/session.rs` is at the hard cap.** 1200 lines exactly, not in
`scripts/file-size-allowlist.txt`. Lines 479–971 are the Move-tool preview code,
and lines 973–1024 are `compute_view_move_preview`, which belongs with it; they
import nothing the transform needs. `clear_transform_preview` sits inside that
range (`:886-908`) and stays behind.

**The C++ flag is nearly derivable.** `transformControls_` is true when the idle
box is up (`controlsIdle_`), and true during a gesture while the Rust session
flag `transform_controls` is true. It is false once a changed release promotes
the session to pending. It is true again after a cancel, because the idle box is
rebuilt.

**Two production literal sites** construct `TransformSession` besides seven test
sites: `prepare_transform` and `begin_selection_transform`.

## Goals / Non-Goals

**Goals:**

- One preview field teardown with one field set, called by every ending path.
- One constructor for `TransformSession`.
- One owner for the controls-session predicate.
- `impl_transform/session.rs` back under the 1200-line cap with no behaviour
  change.

**Non-Goals:**

- Moving the preview fields into `TransformSession`, or converting `QImage` to
  `PixelBuffer`. Both are rejected below.
- Introducing a `PreviewOverlay` struct that renames the fields. It buys nothing
  the single teardown does not already give.
- Changing any begin, gesture, commit, cancel, or history rule.
- Touching the Move tool's own `end_move_preview`, `commit_move`, or cache
  lifetime rules.

## Decisions

### One field teardown on `PictureViewRust`, not a struct

Add `PictureViewRust::clear_preview_overlay(&mut self)` holding the twenty
assignments, including `mask_move_baseline`. `clear_transform_preview` becomes a
thin wrapper with no extra mask-move step. `cancel_transform_session` and
`clear_document_scoped_state` call the shared method instead of their own lists.
When a replacement begin fails and leaves no live Rust session, the C++
controller resets its overlay and emits `transformChanged`. A refusal that keeps
the live session leaves the C++ preview alone.

This gives two methods, and the spec names both: `clear_preview_overlay` is the
field teardown every ending funnels through (`clear_transform_preview` for the
cancel, commit, deferred-land, and smart-object commit paths), and the transform
drop is the extra step the cancel paths take on top of it. There is no script
that enforces "one clear path", so the spec scenarios `ds_preview_one_teardown`,
`ds_preview_cancel_clears`, and `ds_preview_replacement` are the guard.

The alternative — a `PreviewOverlay` struct with a `Default` — was rejected: it
renames 251 references (108 in `impl_transform/session.rs` alone) for the same
structural guarantee, which is exactly the kind of rename this codebase's
no-unrequested-abstractions rule forbids. A single `Default`-based clear would
also need `move_view_pixels: Default::default()` semantics kept, which the
field-by-field method already expresses.

### The teardown stays in the adapter

The preview fields keep their `QImage`s and stay on `PictureViewRust`.

The alternative — moving them into `TransformSession` — was rejected on three
counts. `document_session/session.rs` must name no `cxx_qt` type, so the fields
would have to become `PixelBuffer`, converting to `QImage` on every
`setMovePreviewLayer` of a drag. The fields are written by four non-transform
consumers, so "the transform session owns them" would either force those
consumers through the transform slot or reintroduce a second owner. And the
`PreviewOverlay` rename has no behavioural benefit over one clear method.

### Clear the controls flag at transform lifecycle boundaries

The transform owns `transform_controls`. A begin clears it when `prepare_transform`
takes an outgoing session and when a new session is installed. A refused begin
before the outgoing take leaves the live session and flag alone. `commit_transform`
clears it at entry, so direct bridge commits cannot leave it set. Both cancel
paths also clear it: `drop_transform_session` for the controller's cancel and
`cancel_transform_session` for history restore, Undo, tool switch, and document
replacement. `clear_preview_overlay` does not clear it.

The C++ controller still calls `transform_set_controls(*v, false)` before a
controller-driven commit. That setter is no longer the only commit path that
clears the flag.

### `TransformSession::new` as the only constructor

`prepare_transform` already computes the corner set, centre, and type-layer flag
inline. Move the corners, centre, `quad`, `start_quad`, and the three pivot
fields into an associated function taking the target, path, members, rect, mode,
`type_layer`, and history label. `begin_selection_transform` and the seven test
fixtures call it and then override `quad`, `pivot`, `dragging`, or `show_pivot`
where their case differs.

`type_layer` and `history_label` stay caller-supplied: `prepare_transform`
derives `type_layer` by resolving each member against the document
(`impl_transform/session.rs:1126-1130`) and `history_label` from
`pending_vector_place` (`:1170-1174`). The constructor lives in
`document_session/transform.rs`, which has no document, so it cannot compute
either.

The alternative — deriving `Default` and letting each site override — was
rejected: `Default` would have to invent a `path`, an `orig_rect`, and a
`history_label`, so the "default" would be a fake session and every site would
still set the same five fields.

### The C++ predicate is derived

`transformControlsSession()` becomes a declaration in `tools.h` and a definition
in `tool_transform.cpp`:

```cpp
bool ToolController::transformControlsSession() const
{
    PictureView* v = view();
    return controlsIdle_ || (v && transform_controls_session(*v));
}
```

It cannot be an inline `tools.h` expression: `tools.h` includes no `api/` header,
and `view()` is null whenever the controller holds no live view
(`tools.cpp:749`). `transformControls_` is deleted, along with its thirteen
assignments.

The alternative — keeping the C++ copy — is the duplication the ticket removes.
The risk is the fifteen non-selftest C++ sites that call `v->cancel_transform()`
without also calling `transform_set_controls(*v, false)`; today the local
assignment covers them. Once the field is gone, the Rust teardown must clear the
flag on both cancel routes, which is why the teardown decision above is ordered
first.

The bridge needs one new accessor, `transform_controls_session(view)`, returning
the Rust flag. `transform_set_controls` already exists.

## Risks / Trade-offs

- **A missed reader of `transformControls_`** → the compiler names every use: the
  member has ten read sites (`tools.h:496` and `tools.h:509`, the two header
  reads that become the derived predicate; `tools.cpp:1060`;
  `tool_transform.cpp:56`, `:130`, `:285`, `:449`, `:453`, `:474`, `:481`), all
  routed through the derived predicate, plus five non-test call sites of
  `transformControlsSession()` (`frame_keys.cpp:58`, `menu_transform.cpp:65`,
  `:113`, `:153`, `options_bar.cpp:149`). All are checked against the idle /
  gesture / pending / cancelled four-state table in the spec. `transformPending()`
  (`tools.h:509`) keeps its meaning: the session is active and the derived
  predicate is false.
- **Ordering: deleting the C++ field before the Rust teardown clears the flag**
  → do the teardown and the flag clear in one change, before the C++ deletion.
  The task list orders it that way.
- **The Move-preview module move loses a `super` path** → the moved code uses
  `super::super::…` today; in `session/move_preview.rs` one level changes. The
  compiler names every one; it is a pure move otherwise.
- **`clear_transform_preview` callers that relied on `end_mask_move`** → the
  shared teardown clears `mask_move_baseline`, so the wrapper needs no extra call.
- **A test incorrectly expects the old narrower cancel** → the seven fields the
  cancel used to miss are the point of the change; any test asserting they
  survive is asserting the bug and is corrected, with the reason in the task
  result.
- **File size** → the move takes `session.rs` from 1200 to roughly 680; no
  allowlist entry is needed or added.

## Migration Plan

1. Clear the overlay on the failed begin; add the teardown and route both cancels
   and the document-scoped clear through it; clear the controls flag at begin,
   commit, and cancel boundaries. Green and revertible on its own.
2. Move the Move-preview code to `session/move_preview.rs`.
3. Add `TransformSession::new` and convert the nine sites.
4. Add the bridge accessor, derive the C++ predicate, delete the field.

No persisted state, format, or compatibility shim is involved.

## Open Questions

None.
