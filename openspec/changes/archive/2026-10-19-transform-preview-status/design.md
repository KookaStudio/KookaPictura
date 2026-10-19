# Design — Transform preview appearance and action status

## Preview appearance

`build_transform_preview` already caches two images: the document with the
target hidden (the base) and the target's own pixels (the moving layer). The
moving image came from `layer_image`, which reads the layer's planar channels.
That is exact for a plain raster layer, but not for a layer whose visible
appearance is produced elsewhere: a layer style (Color Overlay), a layer mask, a
vector mask, a fill adjustment, or editable type.

Add `needs_rendered_appearance(layer)`:

- layer mask, vector mask, fill adjustment, type, or `has_layer_style`.

When true, `rendered_layer_image` composites the target alone in a scratch
document, neutralizes its opacity (`255`) and blend (`Normal`), converts the
result to sRGB, and crops it to the layer's rect. The preview then applies the
layer opacity once, as before. Plain raster layers keep the cheaper raw-channel
path, so the per-move cost is unchanged.

The scratch composite runs once when the session begins (and when a warp
preview refreshes), not on every mouse move.

## Action status

The status bar gains `statusActionLabel` (with its own separator), hidden by
default. `updateStatus` derives the text from
`transform_session_mode(*view)` while `ToolController::transformSessionActive()`
is true:

| mode | text |
|---|---|
| `free` | `Free Transform in progress` |
| `skew` | `Skew in progress` |
| `distort` | `Distort in progress` |
| `perspective` | `Perspective in progress` |

`ToolController::transformChanged` already fires on begin, mode switch, commit,
and cancel; it is connected to `updateStatus`, so the readout follows the
session. With no document or no session the label and separator are hidden,
matching the rest of the footer.

## Verification

`tst_transform_skew::styleKeepsTheLayerColor` creates a white raster layer with
a black Color Overlay, begins Skew, and samples the canvas: the preview must be
black. `tst_transform_skew::previewKeepsTheLayerColor` covers the plain raster
path at 83% zoom. `tst_transform_skew::statusShowsTheSkewAction` checks the
label through Free Transform → Skew → cancel.
