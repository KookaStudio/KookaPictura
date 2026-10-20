# Proposal: smart-alignment-guides

## Why

Moving a layer gives no alignment feedback: the canvas shows only marching ants
and the ΔX / ΔY readout. Users must eyeball edges against the document or other
layers. Photoshop's Smart Guides draw magenta lines when edges or centres line
up and magnetically snap the moving content to them (issue #737, screenshots in
the request).

## What Changes

- The Move tool's layer drag and the Type tool's text move compute the moving
  content's ink box against the document's edges/centre and every other layer's
  ink box edges/centres.
- Within 8 screen pixels the moving content snaps to the nearest alignment and
  the canvas draws a magenta guide at the aligned position; guides clear when
  the drag ends or no alignment is active.
- While snapped, a gap readout shows the distance between the moving and
  reference content boxes along the guide's axis, in the rulers' unit.
- A pure solver (`smart_guides.{h,cpp}`) holds the geometry; the tools feed it
  and the canvas draws the result.
- A new `tst_smart_guides` Qt Test suite covers the solver and both tools.

## Capabilities

### New Capabilities

- `tools/smart-guides`: magenta alignment guides and snapping while moving
  content.

### Modified Capabilities

<!-- None. -->

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/align.rs`, `lib.rs` — expose
  the existing ink-box computation for a layer path.
- `crates/pictura-app/src/cxxqt_object/impl_transform/bridge.rs` — a bridge
  function returning a layer's ink box.
- `crates/pictura-app/cpp/api/transform_api.{h,cpp}` — the C++ wrapper.
- `crates/pictura-app/cpp/smart_guides.{h,cpp}` — the pure solver (new).
- `crates/pictura-app/cpp/image_view.h`, `image_view_smart_guides.cpp` — drawing
  and test hooks (new file).
- `crates/pictura-app/cpp/tool_move.cpp`, `tool_type.cpp` — gather references,
  snap, set/clear guides.
- `CMakeLists.txt`, `crates/pictura-app/cpp/tests/CMakeLists.txt` — new sources
  and the test suite.
- `crates/pictura-app/cpp/tests/tst_smart_guides.cpp` — new suite.
- No new dependencies.

## Ceilings

`ponytail:` always-on magenta; the `View > Show > Smart Guides` toggle and the
Preferences colour are not wired (the Preferences group stays disabled). Ruler
guides, the grid, and slices are not snap targets (they belong with the
`View > Snap` work, #305). Free Transform moves are out of scope (another
branch). Groups and adjustment layers contribute no reference box, matching
the existing Align commands.
