# Perspective transform gesture

## Why

`Edit > Transform > Perspective` does not behave like Photoshop (#750). A corner
drag sets the corner to the pointer and moves the diagonally opposite corner the
other way, twisting the box instead of forming a trapezoid. Neither photorust
nor photocraft has a Perspective gesture to port, so it is rebuilt from scratch
against the owner's CS6 screen recording (`test.mp4`, attached to the issue
discussion). The recording shows:

- a corner drag slides that corner along one of its two edges and the edge's
  other corner the same distance the opposite way, about the edge's midpoint;
- the edge is picked from the drag's dominant direction and re-picked on every
  move, so one drag can switch between the top/bottom and the side edges;
- the two corners may meet (a triangle) and pass through each other (a bowtie);
- an edge handle slides only that edge along its own axis;
- the reference point keeps its place in the box (the corner average for the
  default centre);
- a readout next to the pointer shows the side edges' angle while dragging.

Free Transform also reaches Perspective with Ctrl+Alt+Shift on a corner, and
the owner asked for that shortcut with Photoshop's behavior: Free Transform keeps
the perspective box, and later plain drags still scale and rotate it.
Photoshop greys Perspective out for editable type, and the owner asked for the
same.

## What Changes

- Replace the Perspective corner gesture with the symmetric edge slide, and use
  the Skew edge slide for Perspective's edge handles.
- Allow a Perspective box to pass through a triangle into a bowtie; only a
  degenerate quad (non-finite, under 1 px², or three collinear corners) is
  refused, keeping the last quad.
- Fill a crossed (bowtie) box as Photoshop does, its own outline (two
  straight-edged triangles meeting where its edges cross), in both the engine
  and the live preview; a homography onto a crossed quad would fill the outside
  instead.
- While any transform is pending, disable every `Layer` menu command and grey
  the Layers panel (owner request on #750).
- Ask "Apply the transformation?" (Apply / Cancel / Don't Apply) before a tool
  switch, by click or shortcut, leaves a pending transform (owner request on
  #750); today a switch silently cancels.
- Carry the reference point by its bilinear position within the box.
- Add a Free Transform "perspective base" quad: Ctrl+Alt+Shift on a corner edits
  it with the Perspective gesture, while the existing scale, rotate, move, and
  numeric setters keep composing after it. A session with a base commits and
  previews through the projective op.
- Refuse Perspective on editable type, disable `Edit > Transform > Perspective`
  for it, and ignore the Ctrl+Alt+Shift shortcut on it.
- Show the Perspective drag angle in the canvas drag readout (Perspective drags
  only; other modes are a follow-up).

## Capabilities

- `document/free-transform` (MODIFIED quad op and CPU-warp preview; ADDED
  requirements for the gestures, the shortcut, and the readout)
- `document/transform-reference-point` (MODIFIED)

## Impact

- Code:
  - `crates/pictura-app/src/cxxqt_object/impl_transform/{perspective,geometry,dispatch,session,bridge}.rs`
  - `crates/pictura-app/src/cxxqt_object/state.rs`
  - `crates/pictura-app/cpp/{tools.cpp,tool_transform.cpp,frame_menus.cpp}`
  - `crates/pictura-app/cpp/{frame_transform_lock.cpp,commands.cpp,dialogs.cpp}`
    (the layer lock and the tool-switch prompt)
- Tests: Rust gesture units in `impl_transform/perspective.rs`, a new
  `tst_transform_perspective` Qt Test suite.
- Engine: `pictura-render`'s homography resampler (backward mapping with the
  chosen interpolation) is already the algorithm the issue describes; the only
  engine change is the two-triangle map for a crossed quad
  (`layer_ops/crossed_quad.rs`), with the matching path in
  `cpp/image_view_transform_warp.cpp`.
