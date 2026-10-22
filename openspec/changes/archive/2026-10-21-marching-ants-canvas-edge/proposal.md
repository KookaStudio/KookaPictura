# Proposal

## Why

The live selection outline and the committed selection outline are computed from
two different geometries, and only the committed one is clipped to the canvas.
The live rubber band is the analytic shape the tool is dragging; the canvas then
crops it with a hard rectangle clip, so a marquee or lasso dragged past the
canvas edge stops mid-air at the border instead of turning the corner along it.
The release commits the canvas-clipped mask, and its traced outline does close
along the border, so for the whole drag the outline and the selection the user
is about to get disagree (#796).

## What Changes

- Clip the live selection outline to the canvas rectangle: a closed shape is
  closed along the border.
- Add one shared clip helper to the app's pure selection-geometry module and let
  the canvas apply it to the previews that stand for a selection shape.
- Apply it to the rectangular and elliptical marquee, the freehand lasso, the
  magnetic lasso, and the content-aware-move and patch region traces.
- Leave the polygonal lasso's band, the zoom marquee, the red-eye ring, the
  region drag's moved outline, and the move-selection drag's offset outline
  alone. The polygonal band is an open rubber band with no closing edge to
  add, and the paint already stops it at the border. The last two draw an
  existing outline rather than a shape being defined, and the offset outline's
  behaviour is already specified: the capability requires the committed outline
  translated by the drag offset.
- Re-word the `ponytail:` marker in the committed-ants block, which describes
  the move-selection drag's offset outline, so it no longer reads as a note
  about the marquee.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/selection-outline-performance`: the live selection outline is clipped
  to the canvas, so the rubber band and the committed outline describe the same
  region at the border.

## Impact

- `crates/pictura-app/cpp/selection_geometry.{h,cpp}`: the clip helper.
- `crates/pictura-app/cpp/image_view.{h,cpp}`: `setSelectionShapePreview` for
  the previews that stand for a selection shape, and an accessor for the
  preview loops the tests read.
- `crates/pictura-app/cpp/tool_selection.cpp`, `tool_magneticlasso.cpp`,
  `tool_region_drag.cpp`: the selection-shape previews ask for the clip.
- `crates/pictura-app/cpp/tests/tst_selection_preview.cpp` (new) and its entry
  in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- The app self-test cases that assert on preview geometry (`tsc_ellipse_preview`
  and the magnetic-lasso case) are re-verified against the change.
- No new dependencies, no `docs/` change (the corpus already describes the
  outline as the mask-derived, border-closed edge), no control-server or bridge
  change, and no new history state.
