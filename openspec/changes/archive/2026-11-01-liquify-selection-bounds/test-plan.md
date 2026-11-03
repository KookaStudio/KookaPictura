# Test plan — liquify-selection-bounds

Every scenario from `specs/imaging/liquify-filter/spec.md`, plus the engine
tests that back the mesh clip. The Coverage cell names the automated test; the
apply phase must create each one.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lq_selection_rect` | With a rectangular selection over the left half of a 160×120 document, open Liquify, then (a) read `liquify_geometry`/`liquify_origin` and the preview image, (b) stroke entirely outside the rectangle, (c) press OK. | Geometry is the rectangle's size at its origin; preview pixels outside the rectangle are clear; the stroke leaves the preview bit-identical and `liquify_distorted` false; OK records no history state and leaves the document image unchanged. | `tst_liquify_dialog::aSelectionConfinesThePreviewAndCommitToItsBounds`, `pictura-filters::a_clip_leaves_the_picture_outside_it_untouched` |
| `lq_selection_rect_inside` | Reopen Liquify under the same selection, stroke inside the rectangle, capture the preview, then press OK. | The preview changes inside the rectangle; the committed document changes under the stroke and every pixel outside the rectangle is unchanged. | `tst_liquify_dialog::aSelectionConfinesThePreviewAndCommitToItsBounds` |
| `lq_selection_bbox` | With an elliptical selection at (20,20) 60×60 (bounding box to (80,80)), open Liquify, stroke outside the box, then reopen and stroke at (25,25) — inside the box, outside the ellipse — and press OK. | The outside stroke changes nothing; the inside stroke changes the pixel at (25,25) and every pixel outside the box is unchanged. | `tst_liquify_dialog::aNonRectangularSelectionProcessesItsBoundingBox` |
| `lq_selection_miss` | Build a selection that lies wholly off an offset layer rectangle, and an empty selection, and compute the session clip for each. | Both are refused (`Err`), while no selection yields the whole layer and a covering selection yields the layer rect in layer coordinates. | `pictura-app::session_clip_refuses_a_selection_off_the_layer` |

## Engine tests beyond the scenarios

These back the mesh clip itself rather than a dialog scenario; they run with the
scenario tests.

- **A dab outside the clip leaves the mesh identity** — clip the left half of a
  mesh and drag in the right half: `is_identity` stays true and no dirty
  rectangle is returned. `pictura-filters::a_dab_outside_the_clip_moves_nothing`
- **A dab straddling the clip edge moves only inside** — clip a middle
  rectangle and drag across its right edge: nodes inside move, the resampled
  picture outside the rectangle is bit-identical, and the dirty rectangle stays
  inside. `pictura-filters::a_clip_leaves_the_picture_outside_it_untouched`
- **The preview crops to the clip** — render a view whose frame starts outside
  the clip: pixels whose layer point is outside the clip are clear, and inside
  matches the undistorted source.
  `pictura-filters::a_clipped_view_is_clear_outside_its_bounds`
- **A scaled mesh scales its clip** — scale a clipped mesh to a larger layer:
  the clip scales with the mesh and the scaled mesh's displacement is zero
  outside it. `pictura-filters::a_mesh_scaled_to_a_larger_layer_scales_its_clip`

## Manual checks

None.
