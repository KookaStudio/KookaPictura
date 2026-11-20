# Design

## Context

The Move tool's `Show Transform Controls` box is an idle overlay drawn from the
active layer's content rect (`ToolController::syncTransformControls`). A press
on one of its handles starts a transform session tagged as a controls session
(`transformControls_` true, Rust `transform_controls` true). Today
`handleReleased` calls `commitTransformControls`, which runs the commit and
restores the idle box, so the options bar stays on the Move page
(`options_bar.cpp` deliberately excludes a controls session from the transform
page).

A modal Free Transform is a pending session (`transformControls_` false): the
options bar shows the transform page, `transformPending()` drives the Layer /
Image / Filter lock and the Apply / Cancel / Don't Apply prompt, and the canvas
draws the session overlay with its reference point.

## Goals / Non-Goals

**Goals:**

- A changed Move controls gesture becomes pending on release and shows the
  transform page with Confirm/Reject.
- The pending state behaves like a modal transform: lock, prompt, refine.
- An identity gesture and the idle box behave as before.

**Non-Goals:**

- Changing how the idle box is built, how inside/outside presses move the
  layer while idle, or how `Ctrl+T` takes the box over.
- Changing the transform page's controls or the commit engine.
- Making a Move controls transform survive a document close or a tool switch
  without the standard prompt.

## Decisions

### Promote on release, not on press

The in-progress gesture keeps `transformControls_` true, so the options bar
stays on the Move page, the Rust `transform_controls` flag keeps Fill/Stroke
available, and the reference point stays hidden. On release:

- `transform_is_identity()` (new readout) checks the session's quad and
  similarity parameters against the source rect.
- Identity: `cancelFreeTransform()` drops the session and restores the idle
  box. No history state.
- Changed: `promoteControlsTransform()` clears `transformControls_` (C++ and
  Rust), shows the reference point, focuses the canvas, and emits
  `transformChanged`. The session stays live; the bar switches to the transform
  page. No history state yet.

This keeps a click on a handle from flashing the transform page and keeps the
existing "the controls box does not lock while dragging" behavior.

### Reuse the modal session

Once pending, the session is indistinguishable from a modal Free Transform:
`transformPending()` is true, `commitFreeTransform()` records the one
`"Free Transform"` state, `cancelFreeTransform()` restores the document, and
the existing `transform_release` / `transform_press` paths refine the same
session. An inside press is handled by the normal transform press (move within
the box), not by the Move tool's ordinary drag.

### Identity readout

`identity_transform` already lives in `impl_transform/geometry.rs` and is used
by `commit_transform`. The new `transform_is_identity` qinvokable wraps it over
the live session, so the C++ release path reuses the exact commit-time identity
rule instead of comparing readout fields.

## Risks

- The options bar's transform page can now be entered from the Move tool; the
  page already reads the live session, so no new field wiring is needed.
- The controls gesture's Rust `transform_controls` flag must be cleared on
  promotion or the direct document-edit refusal would stay off.
- Tests that relied on commit-on-release (`tst_transform_overlay`,
  `tst_linked_layers`) must confirm the pending transform before checking the
  layer.
