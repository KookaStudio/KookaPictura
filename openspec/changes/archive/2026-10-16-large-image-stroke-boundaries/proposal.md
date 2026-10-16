# Proposal: large-image-stroke-boundaries

## Why

Issue #185: painting on a large image (the 16507×16196 world map, 267 MP) is
choppy, strokes come out straight, and the CPU spikes. Profiled on that image,
each mouse move of a stroke costs under a millisecond; the time is all at the
stroke's boundaries, and every cost is a whole-plane copy or scan:

| step | before |
| --- | ---: |
| first dab of a stroke (forks the target layer's planes) | 0.5–1.1 s |
| commit refresh (forks the 1 GB document composite) | 0.64 s |
| history capture (full-plane compare + single-thread tile scan) | 0.4–0.66 s |
| undo (history forks its planes, then the canvas is rebuilt) | 1.1 s + 1.7 s |
| any full canvas refresh (a full-resolution `QImage`) | 1.4 s |

The root is shared ownership: the live document's copy-on-write planes are
shared with history, so the first write anywhere copies the whole plane, and
history then has to compare whole planes to find what changed. A stall at a
stroke's start or end is what straightens strokes: Qt merges the pointer moves
it could not deliver into one segment.

photorust solved the same image this way (`docs/ARCHITECTURE.md`, "Large
images"): history keeps its own copy and finds changes tile by tile in
parallel, undo redraws only the tiles it restored, the stroke starts without a
snapshot, and the canvas holds only what is on screen. This change ports that
approach. Tiled layer storage (photocraft's model) is the follow-up if the
boundaries are still too slow afterwards.

## What Changes

- `History` keeps a private copy of the state at the cursor, never shared with
  the live document. Capture diffs the live document against it on every core
  and copies only the changed tiles in; undo, redo and jump apply tiles to the
  private copy and to the live document in place and report the document
  rectangle they changed. The anchor of the cursor's segment holds metadata
  only, so the document is not held twice over by history.
- `Stroke` paints the live document in place. Before a dab writes a 64×64 tile
  for the first time it saves that tile's original pixels; cancel restores
  them. A stroke's start no longer depends on the document size.
- The bridge records history from the live document by reference, and undo /
  redo / jump refresh only the restored region.
- A document canvas holds no full-resolution image: it knows the document by
  its size and presents from pyramid crops; the canvas keeps its crops when no
  frame was rebuilt (a `frame_revision`
  that only a whole-frame rebuild bumps), builds it once on open and on every
  core, and the Move tool no longer warms a preview for a Background it may not
  move. Dropping the canvas's full-resolution image altogether is the
  follow-up (see `design.md`).
- Layer lock and colour label no longer composite; blend, opacity and fill
  repaint only the changed layers' bounds (a type layer reaches no further
  than its rect).
- The CPU compositor composites per-pixel stacks in parallel bands instead of
  one document-sized `f32` canvas, converts its canvas to planes in parallel,
  and the view pyramid's shrink kernels run row-parallel.
- GPU composites write layer sources and masks straight into staging memory in
  parallel, read back into one zeroed allocation, and reuse the readback
  buffer; unchanged layer uploads stay resident, keyed by plane stamps;
  whole-region patches of the composite and level 0 run row-parallel.
- The brush family's stroke start opens a paint-timing session, and an ignored
  profile (`large_document_stroke_profile`) measures every boundary at the
  world map's size.

## Capabilities

### New Capabilities

- `document/in-place-history`: history with a private current state and
  in-place, damage-reporting undo.
- `tools/in-place-stroke`: strokes that write the live document and save only
  the tiles they touch.
- `ui/canvas-frame-reuse`: a canvas that rebuilds its image only when the
  frame was rebuilt, a Move tool that warms only a movable layer, and layer
  properties that repaint only their layer.
- `compositing/banded-compositing`: per-pixel stacks composited in bands.
- `compositing/gpu-transfer`: GPU composites without host-side pixel copies.

## Impact

- `pictura-app` (`history.rs`, `cxxqt_object/impl_core.rs`, `impl_history*`,
  `impl_paint.rs`, `paint_tools.rs`, `impl_transform/session.rs`),
  `pictura-paint` (`stroke.rs` and the tools built on it), `pictura-core`
  (`Plane` stamps), the shell (`frame.cpp`, `frame_menus.cpp`).
- `rayon` is added to `pictura-app`; it is already a workspace dependency
  (`pictura-render`, `pictura-filters`), so no new crate enters the lockfile.
- Undo/redo/jump remain byte-exact (`document/history-retention`).

## Provenance

The approach follows photorust's large-image work (`core/src/history.rs`,
`core/src/damage.rs`, `core/src/view.rs`, same author); no code is copied.
