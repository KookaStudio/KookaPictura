# Design

## Context

`ImageView::paintEvent` draws a cached viewport crop plus overlays with
`QPainter`. `wireCanvasLevelProvider` connects the crop to
`PictureView::canvas_display_image`. The provider can return available pixels
and request missing tiles from a worker. `land_tiles` installs accepted tiles
and schedules a later paint. The current GPU timing slot in
`pictura-render::metrics` holds the process's last completed composite time
for edit reporting. Its lifetime extends across canvas and document work.

## Goals / Non-Goals

- Locate the cost of a canvas paint and measure its cadence with the existing
  opt-in flag and control connection.
- Use the current paint and compositor paths. Physical presentation, a Qt
  scene-graph migration, and changes to frame scheduling are outside scope.

## Decisions

### Time the viewport operations and the paint on the GUI thread

Use one monotonic clock for the Qt spans. Measure the app's outer viewport
input handlers for pan, zoom, and Rotate View through the corresponding view
update. Instrument shared input entry points or guard nested calls so a wheel
event and the zoom setter it invokes count once. Accumulate completed input
operations on their canvas until the next paint consumes them. Only a paint
that draws a document creates a frame record. Suppress frame capture around both
control screenshot grab entry points (frame and canvas): their offscreen paint
callbacks must not consume pending input or GPU samples, advance cadence, or
create records. This scoped guard covers explicit application screenshot renders and uses no
private Qt hooks.

Time the paint through `QPainter` teardown. Collect tile gather spans around
present-cache hit, patch, rebuild, and fallback crop work during the paint.
Include Rust bridge work in those spans. The remainder is Qt canvas draw time.
Use the previous completion on the same canvas for cadence. This interval
includes scheduling or idle gaps between paints. The CPU budget calculation
uses measured active work. A renderer GPU duration can overlap that work.

### Carry GPU timings with their render output

Reuse the existing wgpu timestamp query result at readback. Each interval runs from the beginning of the first blend pass through the end
of the last blend pass. It excludes later planar conversion, readback copy,
and window presentation. `gpu_composite_ms` sums the qualifying intervals.
Carry the optional duration out with the successful composite result and through
`DocumentComposite::Refreshed` where the display path consumes it. Keep the
existing composite caller API and the edit timing report compatible. The
frame path uses the result's own timing. Reading the global edit slot at
paint time would allow a completed export, another document, or a discarded
result to supply the sample.

Track both existing attributable producers: `refresh_display_rect`, which can
request a GPU composite and apply its returned region to the display cache, and
`paint_reduced_stand_in`, which can composite reduced document data for stale
tiles. Register one optional sample for the exact result and track the accepted
tile content versions it supplies, including intentionally stale stand-ins. Retire a
sample after its first use, a capture reset, or the loss of all the pixels it
supplied. Use the source, accepted content version, and actual supplied source spans to
decide whether the paint used the result, including a previously patched Qt
crop.
Intersect those spans with the effective `QPaintEvent::region`, transformed
into source coordinates and intersected with the content clip. A disjoint paint
does not consume the sample. Track multiple repaired spans even within one tile
or crop, and reject overwritten stand-in pixels. A single composite can repair
several tiles, so deduplicate by sample identity across the drawn pieces. The usual lazy tile builds use the
CPU. Their completion creates an ordinary new paint with no inherited device
duration. Other stored composite paths lacking provable pixel provenance do not
contribute samples within this scope. Preserve all existing tile admission and
repaint decisions and use only existing GPU paths.

### Publish the capture through the existing metrics snapshot

Use the app's GUI-thread metrics accumulator for a preallocated, bounded
frame ring when enabled. Export it through the existing control method.
1,024 records accommodate a five-second 60 Hz capture with room for other
paints, and the eviction count detects incomplete captures. Buffering keeps
per-frame formatting and synchronous stderr writes outside the measured loop.
The disabled path skips frame clocks, capture state, and bridge calls. The
paint profiler's existing debug logging remains independent.

Use a metrics capture generation to reset pending input, per-canvas cadence,
and unconsumed GPU samples lazily. Bump it after taking a reset snapshot so
later work can identify samples from the earlier capture. Canvas identity
belongs to the widget lifetime and survives a capture reset.

## Risks / Trade-offs

- A cheap paint can follow a long main-loop delay. Interpret the paint cost,
  CPU slack, and paint cadence together. Treat the values as diagnostics of
  the canvas path. Use `gpu_present_ms: null` to identify the unavailable
  presentation phase.
- A crop can reuse overlap, draw a fallback, or be superseded before drawing.
  Report device duration only where the drawn result can be identified.
  Exercise those lifecycle decisions with deterministic samples in tests, and
  check an eligible GPU composite on a timestamp-capable device.
- Timing and bookkeeping affect the captured run. Collect the snapshot after
  the gesture and keep test assertions independent of host speed. A
  five-second run on the reference machine supplies gesture measurements.
