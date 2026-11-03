# Design

## Where the rectangle lives

The processing rectangle belongs to the mesh, not to the dialog session. Three
consumers must agree on it: the dialog preview, the destructive resample
(`liquify()`), and a kept mesh reapplied later (Load Last Mesh, Last Filter).
The commit only receives `Filter::Liquify { mesh }`, so anything the session
holds separately would have to be threaded through the filter record. A
`LiquifyMesh::clip: Option<PsdRect>` is the smallest seam that reaches all
three.

The clip is layer-local and half-open, like `LiquifyMesh`'s own `width` and
`height` (node `(0, 0)` sits at `(0.5, 0.5)`).

## Confinement, not masking

`LiquifyMesh` enforces the clip in three places:

- `dab` skips nodes outside it, so a stroke outside the rectangle leaves the
  mesh identity and returns no dirty rectangle;
- `displacement` returns zero outside it, so the minified preview, `grid_lines`,
  and `scaled_to` see an undistorted field there;
- `set_clip` zeroes any offsets outside it, so a mesh loaded with a different
  history cannot leak a warp past its rectangle.

`liquify()` additionally limits its resample to the clip's rows and columns, so
pixels outside are bit-identical rather than merely re-sampled with a zero
offset. `LiquifySource::render` clears output pixels whose layer point is
outside the clip, which crops the preview.

## The commit must not also apply the coverage mask

`apply_op_active_region` and the commit worker pass the selection as a
`LayerMask` coverage plane into `apply_filter`, which blends per pixel. For a
lasso or ellipse that restricts the result to the selection's shape. The
documented rule is the bounding rectangle: the shape only matters through the
Mask Options freeze source. A clipped mesh already carries the confinement, so
`apply_filter_region` drops the coverage mask for `Filter::Liquify` when its
mesh has a clip. Without a clip (no selection) the old path is unchanged.

## Preview frame

The canvas currently treats the layer as its frame. With a clip, the frame is
the clip: `liquify_geometry` returns the clip's size and `liquify_origin`
returns its top-left in layer coordinates. `LiquifyCanvas` keeps its existing
`offset_` (widget position of the frame's top-left) and gains `origin_` (frame
top-left in layer coordinates), so `toLayer`/`toWidget` stay simple:

```
toLayer(widget) = (widget - offset_) / zoom_ + origin_
toWidget(layer) = offset_ + (layer - origin_) * zoom_
```

`liquify_backdrop` crops the other layers to the same frame. `Show Mesh`
already receives a layer rectangle from the canvas and `grid_lines` intersects
it with the clip, so the mesh overlay stops at the rectangle.

## Alternatives considered

- **Crop the session's source and mesh to the rectangle.** Would need an origin
  on the mesh anyway (to place the commit), plus a second scaling rule to
  distinguish "sub-rect of a layer" from "stretched to fit". The clip keeps one
  mesh geometry and one scaling rule.
- **Keep the whole-layer preview and only make warps inert outside.** Rejected
  against the docs: "for a rectangular marquee the preview and selected area
  are identical" describes a cropped preview.
- **Replace the coverage mask with a bounding-box mask in the app commit path.**
  Duplicates the rectangle in a second place and misses the resample inside
  `pictura-filters`. The mesh is the one source of truth.

## Edge cases

- **Selection misses the layer, or is empty** — `liquify_begin` returns false;
  no dialog, no history.
- **Layer offset from the document** — the clip is converted to layer
  coordinates by subtracting the layer rect's origin.
- **Selection covering the whole layer** — the clip equals the layer rect and
  the session behaves as before, except the preview frame is the layer.
- **Load Last Mesh across sizes** — `scaled_to` scales the clip; the loading
  session then replaces it with its own clip, so a mesh committed under one
  selection does not confine a session opened under another.
- **Feathered selection** — the rectangle is the coverage `> 0` bounds, as
  `Selection::bounds` already defines; processing inside is hard-edged.
- **Last Filter** — the stored mesh keeps the rectangle it was committed with.
