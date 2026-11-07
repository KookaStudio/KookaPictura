# Proposal

## Why

Adding Reveal All to a small raster layer creates a layer-bounded mask with a
white exterior, and the Layers thumbnail represents its margins as transparency.
Issue [#1199](https://github.com/KookaStudio/KookaPictura/issues/1199), extended
by the user's confirmed requirements, needs masks editable throughout the canvas,
black backgrounds after moves, and lossless recovery of coverage moved off-canvas.

## What Changes

- Create masks with an editable domain covering the canvas, independent of the
  owning layer's bounds. Reveal All on a nonempty pixel layer is white inside
  its content rectangle and black outside; From Transparency places alpha at
  its document coordinates on black. Hide All is black throughout.
- Support the same canvas-wide edits on existing/imported masks, preserving
  their current in-canvas coverage and retained off-canvas samples when their
  domain expands. Viewing or selecting a mask does not rewrite the document.
  Data-less imports keep their existing visible coverage of 255 on first local
  edit, independently of a nonwhite stored default.
- Treat the canvas as a display boundary only. Keep the stored domain large
  enough for the canvas and retained coverage; do not clip a mask when moving it
  beyond an edge. Integer moves out and back restore all coverage values exactly,
  including partially covered pixels.
- Make uncovered areas after a whole-mask move and vacated areas after a
  selected-coverage move black, independently of the color swatches. Preserve
  off-canvas data through subsequent edits, undo/redo, and PSD save/reopen.
- Render raster mask thumbnails as opaque grayscale over the document aspect,
  with black margins where coverage is zero, without scanning a full-size plane.
- Apply Properties-panel Invert to the whole canvas, preserving its existing
  selection-independent policy, off-canvas samples, and a black-backed exterior
  in one undo state.
- Reserve mask allocation/growth through the existing memory governor and make
  refusals, cancellations, and no-op gestures atomic.

## Capabilities

### New Capabilities

- `compositing/canvas-mask-domain`: Canvas-wide mask editing, imported-mask
  expansion, black movement backgrounds, off-canvas retention, thumbnail
  sampling, persistence, and allocation-failure behavior.

### Modified Capabilities

- `compositing/layer-masks`: Replace layer-bounded creation and mask-adapter
  bounds with a canvas-covering domain while retaining existing operation and
  edit-target semantics.
- `compositing/mask-refinement`: Make Properties Invert canvas-wide rather than
  inverting retained off-canvas samples and the exterior default; preserve the
  existing density, feather, and selection-independent Invert behavior.
- `tools/selection-content-move`: Move selected mask coverage without clipping
  destinations to the old mask bounds; fill vacated samples toward black rather
  than the background swatch, and allow the stored rectangle to grow.
- `ui/layers-panel`: Replace transparent raster-mask margins and mask-bounds
  raster images with opaque document-space coverage in both thumbnail contents
  modes, preserving unrelated thumbnail and card-layout behavior.

## Impact

- Engine: `pictura-render` mask creation/edit adapters, selected-coverage moves,
  and mask translation in the layer-move path; existing `LayerMask.rect`,
  `default_color`, and `Plane<u8>` represent the domain without a new format.
- Application: mask brush/eraser, shared fills, destructive filters, Properties
  Invert, Move
  gestures, history/damage reporting, Layers thumbnails, and solo/Channels
  display; imported high-depth mask caches must not override edited coverage.
- Verification: Rust engine and codec round trips plus Qt Test integration.
  Update tests that deliberately assert white exterior coverage, transparent
  thumbnail margins, or clipping at the old mask bounds.
- No new dependencies. PSD link-state interpretation and flag-bit corrections
  tracked by #1200 remain a separate change; existing link routing must be
  respected when integrating movement.
- This is a user-requested behavior contract, not a claim of independently
  verified Photoshop parity. No Adobe-authored fixture is required.
