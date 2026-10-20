# Proposal: large-tool-performance

## Why

Issue #719: a brush over ~900 px is choppy on a large image, and adding a
shape on the 16507×16196 world map takes 5–10 s. Measured end to end in the
app (`scripts/profile-tools.py`, below), the two have different causes:

- **Brush.** The Brush, Pencil and Eraser start through `paint_tools::begin`,
  which never takes the reduced-resolution preview or the GPU stroke (those
  live only on `begin_paint`, which the shell does not call). Every dab is the
  exact raster on one thread: a ⌀1940 dab costs ~200 ms, so a mouse move costs
  210–310 ms. The commit then re-composites the stroke's whole extent only to
  patch the stored composite the presents had skipped: 0.3–0.9 s on the map.
- **Shapes.** Adding a shape layer re-composites the whole document. On top of
  that, a vector mask's row table spans only the path's own rows, and a sample
  in any other row fell back to walking every edge of the flattened path, per
  pixel. An ellipse has hundreds of edges, so every pixel above and below it
  paid for them: 10.2 s on the map against 2.7 s for a rectangle.

## What Changes

- **Tool profiler.** `scripts/profile-tools.py` drives the real app over the
  control server (new `tool_options`; `pointer` `down` / `up`) and times each
  press, move, release and undo for brushes, Pencil, Eraser, shapes in Shape and
  Pixels mode, Marquee, Gradient and Paint Bucket, with `--save` / `--compare`
  for before/after tables. The brush family's stroke start, shape layers and
  Shape Pixels open paint-timing sessions so `--timing-log` breaks them down.
- **Banded dab raster.** A Paint-kind dab of 128² px or more rasters its rows
  in parallel bands, byte-identical to one thread (Dissolve stays serial).
- **Pending-only stroke commit.** In-stroke presents patch the stored
  composite; the commit presents only what is still pending; cancel
  re-composites the stroke's extent.
- **Vector-mask rows beyond the path** answer outside without the edge scan.
- **Bounded shape and fill commits.** A new shape layer composites its outline
  and stroke bounds; the fill family composites the rectangle its edit
  reports. Both fall back to a full recomposite when a layer style elsewhere
  could spread the change, or (shapes) a clipped layer sits just above.
- **Parallel fills.** `fill_layer` (Gradient, Paint Bucket, Fill, Stroke,
  Fill/Stroke Path, Shape Pixels) runs rows in parallel, and shape coverage
  sweeps only the path's columns.

## Capabilities

### New Capabilities

- `tools/banded-dab-raster`: parallel dab bands, parallel fills, column-bounded
  shape coverage.
- `tools/bounded-tool-commit`: pending-only stroke commit, cancel repair,
  bounded shape-layer and fill commits.
- `verification/tool-profiler`: the end-to-end tool profiler.

### Modified Capabilities

- `compositing/vector-mask-sampling`: ADDED a requirement for rows beyond the
  path.
- `interop/agentic-control`: ADDED `tool_options` and held-button pointer
  steps.

## Impact

- `pictura-paint` (`stroke.rs`, `fill.rs`), `pictura-core` (`vector.rs`,
  `shape.rs`), `pictura-app` (`impl_paint.rs`, `paint_tools.rs`,
  `paint_tools/fills.rs`, `shapes.rs`), the control server
  (`control_server*.{h,cpp}`), a new Qt suite `tst_tool_commit`, and
  `scripts/profile-tools.py`.
- `rayon` is added to `pictura-paint`; it is already in the lockfile
  (`pictura-render`, `pictura-filters`, `pictura-app`), so no new crate enters.
- Painted pixels are unchanged: the banded raster and the parallel fill are
  byte-identical to the serial ones, and every bounded commit is held to a full
  recomposite by `tst_tool_commit`.
