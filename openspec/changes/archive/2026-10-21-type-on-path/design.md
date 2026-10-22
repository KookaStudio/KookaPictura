# Design: type on a path

## Model

`pictura_core::TextPath` holds one `pictura_core::path::Subpath`, a kind
(`OnPath` / `Area`), and the start offset in document pixels. `TypeSpec` and
`TypeTool` gain `path: Option<TextPath>`; a path type layer is still a type
layer, so the existing TySh pixels, locks, reopen, and Free Transform paths are
unchanged.

The path is stored in document coordinates. `TypeSpec::origin` remains the
TySh transform origin (the point where text starts for on-path type, the box
top-left for area type). `TypeSpec::box_bounds` is set for area type so the
existing paragraph wrap, caret, and overflow code applies.

## Arc-length layout

`pictura_core::path::Subpath::flatten` already gives the polyline; the new
`FlattenedSubpath` adds cumulative arc lengths and answers:

- `point_at(arc)` — position and unit tangent, clamping an open path and
  wrapping a closed one;
- `nearest(x, y)` — nearest arc length and distance, used for the click start
  and for path hit-testing.

`pictura_render::type_path` shapes the text once, adds tracking, then walks the
glyphs: glyph *n* starts at `start + Σ advances`, its baseline is the path
point there, and its outline is rotated by the tangent. Horizontal type puts
the glyph upright axis across the tangent; vertical type rotates it a further
90° so the upright axis follows the tangent. A closed path wraps once; an open
path stops at its end. Glyphs past the limit are not painted and the layer
reports overflow at the limit point.

Area type reuses `type_paragraph::visual_lines` with a box equal to the path's
bounding box, then multiplies the rendered alpha by the path interior coverage
(4 samples per pixel, even-odd). This is the recorded approximation: Photoshop
reflows each line to the shape instead of clipping a rectangle.

## Persistence

Photoshop keeps on-path/area geometry in the document-level `Txt2` block's
`ResourceDict.TextFrameSet`, one entry per text layer, matched by text-layer
order. The structure was established from public `ag-psd` samples:

```
/8 << /0 [ << /0 << /1 << /0 [8 numbers per cubic segment] >>
            /2 << /0 2 /6 [start_fraction length_fraction]
                  /11 << /3 0 /5 0 /7 false >> >> >> >> ] >>
```

`Type` 1 is area type (`TextRange [-2, -2]`, `PathData 18 -2 / spacing -2`);
`Type` 2 is on-path type. Pictura writes one entry per text layer (an empty
entry for non-path layers keeps the order), preserves every other key in an
existing `Txt2`, and reads the entries back onto the text layers in the same
order. `TextRange[0]` is the start fraction of the path length.

The `Txt2` bytes are only an approximation of Photoshop's full global engine
data when Pictura authors a new document; the on-path geometry is what
Photoshop and `ag-psd` consume. The project's own save→open round-trip is the
automated oracle; Photoshop rendering compatibility is not claimed without a
CS6 fixture.

## UI

`TypeSetting` carries `path_kind`, `path_closed`, `path_start`, and
`path_points` (8 numbers per cubic segment, `;` separated). `type_api::pathAt`
returns the hit path from the Work Path or saved paths, preferring the edge
(within four screen pixels) over the interior. `tool_type.cpp` keeps the hit in
`TypeFrame`, disables the paragraph box gestures, shows the path as the
baseline guide, and draws the overflow `+` at the path end.

## Ceilings

- One subpath per path-type session; a multi-subpath path uses the clicked
  subpath.
- Area type clips a bounding-box paragraph layout; it does not reflow each line
  to the closed path.
- A hard line break in on-path text is laid out as a space; Photoshop's
  multi-line-on-path offsets are not modelled.
- Dragging the start marker, flipping sides, and baseline-shift-on-path are not
  implemented.
