# Test Plan

One row per spec scenario. Coverage names the automated test, or a manual check
with the reason it cannot be automated.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Gradient and pattern fill on a new shape | Set the bar's Fill to a gradient, draw a rectangle in Shape mode; repeat with Pattern. | The new shape layer's fill content is a gradient/pattern inside its outline. | `tst_shape_tools::fillStrokeOptions`; `pictura-render::gradient_fill_and_pattern_fill_round_trip` |
| Gradient and pattern stroke on a new shape | Set the bar's Stroke to a pattern, draw a rectangle. | The outline is stroked with the pattern at the bar's width and Align. | `tst_shape_tools::fillStrokeOptions` |
| Restyling the active shape mirrors the bar | Select a solid-filled shape, change the bar to a gradient fill and dashed stroke, reselect it. | The layer shows both, the bar mirrors both, and each change is one state. | `tst_shape_tools::fillStrokeOptions` |
| Dashed stroke has gaps | Stroke a rectangle 4 px dashed and sample along its top edge. | Some samples carry the stroke colour, some show the background. | `tst_shape_tools::fillStrokeOptions`; `pictura-render::dashed_and_dotted_strokes_render_gaps` |
| Dotted stroke has smaller marks | Stroke a rectangle with Dotted. | The stroke is short marks, not a continuous band. | `pictura-render::dashed_and_dotted_strokes_render_gaps` |
| Copy and paste a gradient stroke | Give A a gradient fill and a dashed pattern stroke; copy and paste onto B. | B's fill, stroke, width, Align, and line type match A's. | `pictura-render::copy_and_paste_attributes_transfer_gradient_and_pattern` |
| The menu lists the five operations | Activate a shape tool. | The Path Operations menu is enabled and lists the five operations. | `tst_shape_tools::optionsBar` |
| Subtract removes the overlap | Active rectangle layer, Subtract, draw an overlapping rectangle. | The active outline is the difference; the overlap shows the layer below. | `tst_shape_tools::pathOperations` |
| Intersect keeps the overlap | Active rectangle layer, Intersect, draw an overlapping rectangle. | The active outline is the intersection. | `tst_shape_tools::pathOperations` |
| Exclude removes the shared area | Active rectangle layer, Exclude, draw an overlapping rectangle. | The active outline covers both rectangles except the overlap. | `tst_shape_tools::pathOperations` |
| Combine with no target creates a layer | No shape layer active, Combine, draw a rectangle. | A new shape layer with the rectangle outline is created. | `tst_shape_tools::pathOperations` |
| Subtract on the Work Path | Work Path holds a rectangle; Subtract; draw an overlapping rectangle in Path mode. | The Work Path becomes the difference. | `tst_shape_tools::pathOperations` |
| Opacity and anti-alias affect the painted outline | Pixels mode, 50 % Opacity, Anti-aliased off, fill a rectangle on white. | Covered pixels are a hard half-strength blend and the edge has no fractional coverage. | `tst_shape_tools::fillPixelsOptions` |
| Mode affects the painted outline | Pixels mode, Multiply, fill a rectangle over a coloured layer. | Covered pixels are multiplied rather than replaced. | `tst_shape_tools::fillPixelsOptions` |
| A fractional rectangle snaps to whole pixels | Snap To Pixels on, drag (10.4, 10.4) to (40.4, 30.6). | The outline anchors lie on whole pixels. | `tst_shape_tools::geometryOptions` |
| A proportional drag keeps the ratio | Proportional 2:1, drag wider than 2:1. | The box is 2:1. | `tst_shape_tools::geometryOptions`; `pictura-core::proportional_drag_uses_the_ratio` |
| Shift still squares | Proportional 2:1, hold Shift. | The box is square. | `tst_shape_tools::geometryOptions` |
| Draw, mirror, restyle, resize, constrain | The existing `optionsBar` flow. | The shape, bar mirror, states, fixed size, and square drag behave as before. | `tst_shape_tools::optionsBar` |
| The stroke and no-fill read elsewhere | Save an unfilled shape with a 3 px inside stroke and read with psd-tools. | psd-tools reports the enabled inside Stroke effect and fill opacity 0. | `pictura-codec::psd_tools_reads_a_shape_stroke_and_no_fill` |
