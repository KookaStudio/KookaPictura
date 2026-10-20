# Distort transform parity

## Why

`Edit > Transform > Distort` does not behave like Photoshop (#748). The grabbed
corner jumps to the pointer instead of following it, the box can fold over
itself, the reference point stays behind while the box moves, editable type is
accepted even though Photoshop greys Distort out, and corners do not snap to
guides. The owner's reference implementation in photocraft has the intended
Distort behavior.

## What Changes

- Port photocraft's Distort corner gesture:
  - move the grabbed corner by the press-to-pointer delta, keeping the grab
    offset;
  - stop where the quad would become concave or fold over itself;
  - carry the reference point with the quad, preserving its place in the box.
- Snap a Distort corner to visible document guides within 8 screen pixels.
  Because `View > Snap` / `Snap To > Guides` are still stubs, this applies
  whenever guides are visible.
- Refuse `Distort` on an editable type layer (and disable the menu command),
  except when the session targets pixels lifted by a selection.
- Port photocraft's projective front-side rule: a destination pixel whose
  inverse-mapped source point lies beyond the horizon is left transparent, and
  the live projective preview clips to the same front side.

## Impact

- Capabilities: `document/free-transform` (MODIFIED),
  `document/transform-reference-point` (MODIFIED).
- Code:
  - `crates/pictura-app/src/cxxqt_object/impl_transform/{distort,convex,dispatch,session,geometry,state}.rs`
  - `crates/pictura-app/src/cxxqt_object/impl_transform/bridge.rs`
  - `crates/pictura-app/cpp/{tools,tools.h,frame_menus,image_view,image_view.h}.cpp`
  - `crates/pictura-render/src/document_ops/layer_ops/transform.rs`
- Tests: Rust gesture/engine units, a new `tst_transform_distort` Qt Test suite,
  and the existing Skew/Perspective coverage kept green.
