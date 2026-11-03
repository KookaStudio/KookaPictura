# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lbt_outline_size` | Hover the Brush at 100 % zoom; set the diameter to 10 then 30 and render the canvas each time | The outer dark-edge distance on the centre row grows by 20 screen pixels, so the ring follows the brush size | `tst_brush_outline::ringTracksBrushSize` |
| `lbt_outline_zoom` | Hover the Brush with `D=8` at zooms 0.5, 0.75, 1, 1.25, 1.5, 2, 3, and 4; render and measure `hi - lo` through the ring's centre. Then set `D=10` at zoom 1.25 (`12.5 -> 13`) | With `D=8` the extent `hi - lo` equals `round(D × zoom)` at every zoom (4, 6, 8, 10, 12, 16, 24, 32); with `D=10` at 1.25× it is 13 within ±1 pixel | `tst_brush_outline::ringDiameterMatchesZoom` |
| `lbt_outline_snap` | With the view unrotated at DPR 1, at the same zooms with `D=8`, scan the centre row and the centre column and compare the stroke extents with the snapped centre | The centre is on a whole screen pixel (a device pixel at DPR 1), the left/right and top/bottom extents are symmetric (exact because every `D=8` product is even), each cardinal stroke is one device pixel thick, and the extent is `round(D × zoom)` screen pixels, within one pixel for an odd rounded diameter | `tst_brush_outline::ringEdgeSnapsToScaledPixelBoundary` |
| `lbt_outline_halo` | On a mid-gray document at DPR 1, hover the Brush at zooms 0.5, 1, 2, and 4 | The radial offset between the black and white rings is one device pixel at every zoom | `tst_brush_outline::ringHaloIsOneDevicePixel` |
| `lbt_outline_subpixel` | Set `D=1` at zoom 0.25 and hover the Brush | The ring is drawn at one screen pixel rather than disappearing | `tst_brush_outline::subPixelBrushKeepsAOnePixelRing` |
| `lbt_outline_outside` | Place a brush ring so half of it crosses the document's right edge and render the canvas | Dark ring pixels exist outside the document rectangle, bounded only by the widget | `tst_brush_outline::ringDrawsPastTheDocumentEdge` |
| `lbt_outline_hidden` | Switch from Brush to Move over the canvas, then send a leave event while Brush is active | No ring is reported or drawn in either case | `tst_brush_outline::noRingForOtherTools` |

The ring sampling tests that already exist are updated to follow the snapped
centre: `tst_canvas_pan::panHidesTheBrushRing` and
`tst_transform_overlay::freeTransformClearsTheBrushRing`. `tst_eraser_tools`
keeps its existing tolerance: the Eraser Block's size requirement is unchanged
and only its centre snaps.
