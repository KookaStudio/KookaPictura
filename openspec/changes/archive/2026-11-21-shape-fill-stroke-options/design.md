# Design

## Appearance model

`pictura_render::ShapeFill` is `Solid([u8; 3]) | Gradient(GradientFillParams) |
Pattern(PatternFillParams)`. `ShapeStroke` carries `ShapeStrokeFill` (the same
three sources), the width, the Align position, and a `StrokeLine`
(`Solid | Dashed | Dotted`). The shape layer's fill stays the existing
`FillContent` (`SoCo` / `GdFl` / `PtFl`) and the stroke stays the authored
`lfx2` `FrFX` effect. This keeps compositing on the existing paths and makes
Copy/Paste Shape Attributes a typed value copy.

Gradient and pattern fills are written with the existing
`pictura_codec::encode_gradient_fill_params` / `encode_pattern_fill` encoders
and read back with the existing `FillContent` decoder. Gradient and pattern
strokes merge the same descriptor items into `FrFX` and select `PntT`
`GrFl` / `Ptrn`; the layer-effect decoder already reads both.

## Dashed and dotted strokes

The stroke effect's band is a dilate/erode ring around the content matte, not a
path stroke. A dash is therefore a modulation of that band. `FrFX` gains an
authored `DshP` text (`on,off` px) and an `Lntp` text (`solid`, `dashed`,
`dotted`). The renderer flattens the layer's vector-mask subpaths (already
polyline vertices in 1/256 px), walks the dash pattern along their arc length,
and rasterizes each "on" interval as a capsule into a mask over the effect's
padded build region. The band is multiplied by that mask.

Ceiling (`ponytail:`): a layer without a vector mask keeps a solid stroke; the
dash phase restarts at each subpath; caps are square-ish capsules rather than
CS6's cap menu; the pattern is derived from the line type and width (no
separate dash editor).

## Path operations

The engine records a subpath's boolean operation in `vmsk`, but the renderer
samples union only. The shell already has robust path booleans in
`QPainterPath::united` / `subtracted` / `intersected`, so the shape tool
performs the operation in C++: it encodes the target outline and the new shape
into the same flat anchor/handle form the canvas preview uses, combines them,
and sends the result back through new bridge calls that replace the active
shape layer's outline or the Work Path.

Values: 0 New Shape (existing behaviour), 1 Combine, 2 Subtract, 3 Intersect,
4 Exclude. A non-zero operation with an existing target replaces that target in
one "Path Operation" state; Combine with no target creates a new shape layer;
the other operations with no target are no-ops. Ceiling (`ponytail:`): the
result is a flattened polygon (line segments), so a boolean result loses the
original curve handles; New Shape is not CS6's default Combine, which keeps the
existing new-layer behaviour and tests intact.

## Fill Pixels options

`ShapeSpec` gains `fill_mode`, `fill_opacity`, and `fill_antialias`. The bridge
maps the mode to the Paint Bucket's supported `PaintMode` set (Normal,
Dissolve, Behind, Clear, Multiply), passes opacity through, and thresholds the
anti-aliased coverage to 0/255 when Anti-alias is off. Ceiling (`ponytail:`):
the mode list is the paint engine's five modes, not CS6's 27.

## Geometry

`ShapeOptions` gains `proportion: Option<f64>` (width / height). `outline`
uses it when Shift is off; Shift still squares or uses the custom shape's
designed aspect. Snap To Pixels rounds the Rectangle and Rounded Rectangle drag
box to whole pixels before generating the outline; other tools ignore it.

## UI wiring

`ShapeOptions` in `tool_options.h` gains the appearance kinds and payloads,
the path operation, the Fill Pixels fields, Snap To Pixels, and the
proportional ratio. `options_bar_shape.cpp` reuses `GradientFillDialog` and
`PatternFillDialog` for the pickers, enables the line-type and path-operation
menus, and shows the Fill Pixels fields in Pixels mode. `tool_shape.cpp` builds
the `ShapeSpec` and performs the path-operation combine before landing.
