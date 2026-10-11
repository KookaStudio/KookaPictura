# document-canvas Specification

## Purpose
Document-level canvas resize: nine-anchor translation of layer and mask bounds, channel re-extension, and composite refresh.

## Requirements

### Requirement: Document canvas resize entry point and error contract

The system SHALL provide `pictura_render::resize_canvas_document(doc: &mut Document, width: u32, height: u32, anchor: Anchor) -> Result<(), OpsError>`. On success it SHALL set `doc.width` to `width` and `doc.height` to `height` and return `Ok(())`, performing all validation before mutating `doc`. A `width` or `height` below 1 SHALL be rejected with `OpsError::InvalidParams`, and a malformed pixel-layer channel or decoded mask SHALL likewise be rejected before mutation. On any error `doc` MUST be left bit-identical to its state before the call. The function MUST NOT panic for any input, including 1×1 documents and empty layer stacks.

#### Scenario: A successful canvas resize sets the document dimensions

- **WHEN** `resize_canvas_document` is called with `width` and `height` of 1 or greater
- **THEN** it returns `Ok(())` and `doc.width` and `doc.height` equal the requested values

#### Scenario: Invalid dimensions are rejected and the document is untouched

- **WHEN** `resize_canvas_document` is called with `width` 0 or `height` 0
- **THEN** it returns `OpsError::InvalidParams` and every field of `doc` equals its pre-call value

#### Scenario: A malformed layer channel errors instead of panicking

- **WHEN** a pixel layer's channel or mask `data.len()` does not match the pixel count implied by its `rect`
- **THEN** `resize_canvas_document` returns `OpsError::InvalidParams`, leaves `doc` unchanged, and does not panic

### Requirement: Nine-anchor translation of layer and mask bounds

`resize_canvas_document` SHALL compute the horizontal and vertical offsets `dx`
and `dy` from the differences `width - old_width` and `height - old_height` and
`anchor` using the same nine-anchor math as `pictura_ops::resize_canvas` (the
left/middle/right family yields `0`, half the delta, or the whole delta, and
likewise for the top/middle/bottom family). It SHALL translate every layer
`rect` and every decoded `LayerMask` `rect` by `(dx, dy)` and SHALL recurse
through group `children`. Layer and mask channel `data` MUST NOT be resampled or
reordered; only bounds move, and the compositor clips content to the new canvas.
Every layer's `name`, `blend`, `opacity`, `clipping`, `visible`, and
`adjustment` payload MUST be preserved unchanged.

#### Scenario: Center anchor translates a full-canvas layer symmetrically

- **WHEN** a 1000×1000 document with a full-canvas layer is grown to 1200×1200 with `Anchor::Center`
- **THEN** the layer `rect` becomes `left`/`top` 100 and `right`/`bottom` 1100

#### Scenario: Top-left anchor adds space on the right and bottom

- **WHEN** a document is grown with `Anchor::TopLeft`
- **THEN** every layer `rect` keeps its `left` and `top` and only `right` and `bottom` increase by the deltas

#### Scenario: Bottom-right anchor adds space on the left and top

- **WHEN** a document is grown with `Anchor::BottomRight`
- **THEN** every layer `rect` keeps its `right` and `bottom` and `left` and `top` decrease by the deltas

#### Scenario: Nested layers and masks are translated

- **WHEN** a group contains a nested masked pixel layer and the canvas is resized
- **THEN** the nested layer `rect` and its mask `rect` are both translated by the same `(dx, dy)`

### Requirement: Document channel re-extension and transparent added canvas

`resize_canvas_document` SHALL re-extend or crop every entry of `doc.channels` to
the new document size with an all-zero fill, preserving each channel's `id`.
Because layer content only translates and no layer data covers the grown region,
the added canvas area MUST be transparent (alpha 0) in the recomputed composite:
the added document-channel samples are extended with 0 and no added pixel carries
any fill color.

#### Scenario: Document channels are re-extended and cropped

- **WHEN** a document carrying a document-level channel is grown and then shrunk
- **THEN** the channel `data.len()` tracks the new document size, its `id` is unchanged, and the added samples are 0

#### Scenario: The added canvas is transparent

- **WHEN** the canvas is grown
- **THEN** the recomputed composite has alpha 0 in every added pixel

#### Scenario: Grown and cropped content matches the anchor offset

- **WHEN** the canvas is grown and then a document-level channel is inspected
- **THEN** the original samples appear at the anchor offset and the newly exposed samples are 0

### Requirement: Composite recomputation after document canvas resize

After a successful `resize_canvas_document`, `doc.composite` SHALL be replaced by
`pictura_render::composite_rgba(doc)` at the new document size. The composite
MUST be recomputed from the translated layer tree so that `doc.composite` always
equals `composite_rgba(doc)` immediately after the call.

#### Scenario: The cached composite equals a fresh composite after canvas resize

- **WHEN** `resize_canvas_document` succeeds
- **THEN** `doc.composite` is bit-identical to `composite_rgba(&doc)` and has the new dimensions

### Requirement: Document canvas resize oracle

The system SHALL ship an oracle for `resize_canvas_document` in
`crates/pictura-render/tests/document_oracle.rs` that (a) writes the resized
document with `pictura_codec::write_psd`, re-reads it with
`pictura_codec::read_psd`, and confirms the structural round-trip, (b) opens the
written file with the independent `psd-tools` library and confirms the reported
document dimensions and layer count match the
anchor-translated document, and (c) asserts `doc.composite ==
composite_rgba(&doc)`. The oracle SHALL skip with a message when `psd-tools` is
not importable and MUST NOT be marked `#[ignore]`.

#### Scenario: The canvas-resized document round-trips structurally

- **WHEN** a layered document is canvas-resized and written, then re-read by `pictura_codec::read_psd`
- **THEN** the re-read document has the new dimensions and every layer's `rect` matches the translated rect

#### Scenario: psd-tools sees the canvas-resized document

- **WHEN** the written PSD is opened with `psd-tools`
- **THEN** psd-tools reports the new document dimensions and layer count

#### Scenario: Missing psd-tools skips cleanly

- **WHEN** `psd-tools` is not importable
- **THEN** the oracle prints a skip message and the suite still passes

### Requirement: Layer translation uses the active backend

Translating a layer SHALL recompute the document composite through the active
backend — the GPU compositor when GPU compute is enabled and a usable adapter
exists, otherwise the CPU compositor — and SHALL remain within ±1 LSB of the CPU
oracle. The CPU `translate_layer` and `recompute` remain the oracle and MUST NOT
change.

#### Scenario: A Move commit on a large document composites on the GPU

- **WHEN** a layer move is committed on a 4000×4000 document with GPU compute
  enabled and a usable adapter present
- **THEN** the document composite is recomputed on the GPU and every channel
  differs from the CPU oracle by at most 1 LSB

### Requirement: Dirty-region canvas refresh

The canvas SHALL treat the document's planar `composite` buffer as the
authoritative full-document canvas and SHALL NOT maintain a full-resolution
`QImage` as a per-pixel region-patch target. When a mutation reports a dirty
rectangle, the view SHALL composite only that rectangle through the active
backend, write the result into the document's `composite` at the rectangle's
origin with a plane `copy_from_slice` (no per-pixel FFI), convert only the
rectangle to a `QImage`, mark the display dirty, and emit a
`regionBlitted(region, x, y)` signal. It MUST NOT emit `changed`, and it MUST NOT
require rebuilding the full-resolution image. A Move commit SHALL invalidate
`old_layer_rect ∪ new_layer_rect`; a paint dab SHALL invalidate the dab's bounding
box. A canvas updated only through such region refreshes SHALL be byte-identical
to a full recomposite after any sequence of those mutations. There SHALL be no
area budget: a dirty rectangle of any size SHALL take the region-blit path and
MUST NOT fall back to a full-document recomposite on account of its size. A
mutation that does not report a dirty rectangle SHALL keep the full recomposite
and SHALL emit `changed`.

#### Scenario: After several Move commits the region-refreshed canvas equals a full recomposite

- **WHEN** several Move commits are applied to a document, with GPU compute
  enabled or falling back to the CPU oracle
- **THEN** the document composite and the rebuilt full image are byte-identical to
  a full recomposite of the final document

#### Scenario: A region update does not disturb pixels outside the region

- **WHEN** a dirty rectangle is refreshed through the active backend
- **THEN** every pixel outside the rectangle keeps its previous value and every
  pixel inside the rectangle equals the corresponding pixel of a full recomposite

#### Scenario: A large dirty rectangle does not fall back to a full composite

- **WHEN** a 1024×1024 paint dab is refreshed on a 4000×4000 document
- **THEN** only the rectangle is composited and blitted, the view does not run a
  full-document composite because of the rectangle's size, and the result equals a
  full recomposite

#### Scenario: A region refresh does not emit `changed`

- **WHEN** a region refresh runs on a document
- **THEN** it emits `regionBlitted(region, x, y)` and the panel refresh is not
  driven more frequently than by a full recomposite

### Requirement: Region-composited move-preview base

Starting a Move-tool preview (`PictureView::begin_move_preview`) MUST NOT
composite the whole document. It SHALL build the preview base by region-
compositing only the moved layer's clamped document rectangle with that layer
hidden (`visible = false`) against the current cached canvas, and MUST NOT modify
the document's stored `composite` or the persistent cached canvas. The resulting
base image MUST be byte-identical to a full recomposite of the document with that
layer hidden. The base SHALL be derived from the authoritative document composite,
never from a cached full image that a region refresh has left stale. The moved
layer's image, its document-space origin, and its opacity that the preview exposes
SHALL be unchanged.

#### Scenario: The move-preview base equals a full composite with the layer hidden

- **WHEN** `begin_move_preview` runs on a document whose cached canvas is current
- **THEN** the returned base image is byte-identical to a full composite of the
  document with the topmost pixel layer hidden

#### Scenario: The preview start does not composite the whole document

- **WHEN** a Move-tool drag starts on a 4000×4000 document
- **THEN** only the moved layer's rectangle is composited, and the document's
  stored composite and the persistent cached canvas are unchanged

#### Scenario: A stale cached image is not used for the preview base

- **WHEN** a region refresh has marked the display dirty and a Move-tool drag then
  starts
- **THEN** the preview base is built from the document composite, not from the
  stale cached image, and equals a full composite with the layer hidden

### Requirement: Region-composited layer visibility toggle

Toggling a layer's visibility (`PictureView::set_layer_visible`) SHALL refresh
only the toggled layer's influence rectangle when that is provably equivalent to
a full recomposite — a raster layer's clamped `rect`, or an adjustment layer's
mask rectangle only when the mask is enabled, carries pixel data, and has a zero
`default_color` — rather than recompositing the whole document. When the toggle
is not provably confined to such a rectangle — an adjustment layer with an
unmasked mask or a non-zero `default_color`, a group, or any layer whose effect
cannot be bounded — it MUST fall back to a full recomposite. The refreshed canvas
MUST be byte-identical to a full recomposite of the document after the toggle.

#### Scenario: Toggling a raster layer updates only its rectangle

- **WHEN** a raster layer that covers a sub-rectangle of a 4000×4000 document is
  hidden or shown
- **THEN** only that layer's clamped rectangle is composited, the rest of the
  canvas keeps its previous value, and the result equals a full recomposite

#### Scenario: A bounded adjustment toggle is byte-identical

- **WHEN** an adjustment layer whose enabled mask carries pixel data with a zero
  `default_color` is hidden or shown
- **THEN** only its mask rectangle is composited and the canvas equals a full
  recomposite of the document

#### Scenario: An unboundable toggle falls back to a full recomposite

- **WHEN** a group layer, or an adjustment layer with an unmasked mask or a
  non-zero `default_color`, is hidden or shown
- **THEN** the canvas is recomposited in full and equals a full recomposite of the
  document

### Requirement: Zoom-cached canvas present

The canvas SHALL present the document by cropping a view-pyramid level instead
of scaling the full-resolution document on every paint. The canvas SHALL choose
the coarsest level whose scale is still at least as fine as the screen: level 0
for zoom > 0.5; level 1 for 0.25 < zoom <= 0.5; level 2 for 0.125 < zoom <= 0.25;
and so on. A pan or hover repaint MUST NOT resample the full-resolution document.
There SHALL be no pixel-count bound that disables the crop path: a document of
any size SHALL present through a level crop.

#### Scenario: A pan repaint does not resample the document

- **WHEN** the view is panned without changing the zoom
- **THEN** the paint crops the level chosen for the zoom and does not rescale the
  full-resolution document

#### Scenario: A zoom change invalidates the cache

- **WHEN** the zoom changes across a level boundary
- **THEN** the next paint chooses the corresponding level and presents it

#### Scenario: Cached present matches the transform draw

- **WHEN** the canvas paints a document at zoom 100 % or above (level 0) through
  the level crop and through a direct full-resolution transform draw, with the
  same sampling filter
- **THEN** the two rendered images are pixel-identical

#### Scenario: A coarse-level present is a single reduction

- **WHEN** the canvas paints at a zoom that selects a level below 0
- **THEN** the source is that level's crop, a single reduction of the composite,
  and the paint does not rescale the full-resolution document

#### Scenario: A document past the former cache bound still presents through a crop

- **WHEN** a document whose scaled present would exceed 64 megapixels is painted
  at a high zoom
- **THEN** the paint still crops a level rather than drawing the full-resolution
  source, and does not fall back to the transform draw

### Requirement: C++ region blit and present-cache coherence

The view SHALL provide a C++ region blit
(`ImageView::blitRegion(const QImage& region, int x, int y)`) that draws the
region at `(x, y)` using `QPainter` with `CompositionMode_Source`. The blit SHALL
keep the transparency checkerboard, the document clip, and the pan/zoom behaviour
unchanged, and SHALL schedule a repaint. The view pyramid SHALL be updated from a
region refresh by the app's Rust view through the damage account, so the next
paint crops the updated levels; the C++ blit MUST NOT be responsible for updating
the pyramid. There SHALL be no per-pixel FFI blit of a region into the canvas.
The displayed canvas SHALL be pixel-identical to a full recomposite of the
document. `PictureView::image()` SHALL build a full-resolution image on demand
from the cached level-0 frame (or the document's planar composite when there is
no level-0 frame) and MUST NOT hold a persistent full-resolution `QImage`; the
repaint path MUST NOT depend on it. `sample_argb` SHALL read the document's
planar composite directly without building a full image; and `move_preview_base`
SHALL be derived from the current composite, never from a stale cached image.

#### Scenario: The C++ blit path is exercised

- **WHEN** a region refresh is emitted while its document is active
- **THEN** `ImageView::blitRegion` draws it, the app's Rust view updates the
  affected pyramid levels, and the next paint crops those updated levels

#### Scenario: The on-screen canvas after a region refresh matches a full recomposite

- **WHEN** a region refresh is followed by a forced full recomposite of the same
  document
- **THEN** the canvas image before the recomposite is pixel-identical to the image
  after it

#### Scenario: `image()` rebuilds from the composite only when dirty

- **WHEN** `image()` is called after a region refresh and again after a full
  recomposite
- **THEN** both calls return an image built from the cached level-0 frame and
  byte-identical to a full recomposite, and no persistent full-resolution `QImage`
  is held between calls

#### Scenario: `sample_argb` reads the current composite

- **WHEN** `sample_argb(x, y)` is called after a region refresh
- **THEN** it returns the document composite's pixel at `(x, y)`, not a stale
  displayed value, and does not build a full image

#### Scenario: The existing M31/M32 canvas-equality checks still hold

- **WHEN** the M31 region-move and M32 move-preview/visibility self-tests run
  after a region refresh
- **THEN** the canvas they compare is byte-identical to a full recomposite, as
  before

#### Scenario: image() builds the full image on demand

- **WHEN** `image()` is called for an open document
- **THEN** it returns an image built from the current level-0 frame and
  byte-identical to a full recomposite

#### Scenario: No persistent full-resolution QImage is held

- **WHEN** a document is open and no caller has consumed `image()`
- **THEN** the view holds no persistent full-resolution `QImage`

#### Scenario: The repaint path does not depend on `image()`

- **WHEN** the canvas repaints after a region refresh at any zoom, before any
  caller has consumed `image()`
- **THEN** it presents a view-pyramid level crop and does not build or read a
  full-resolution image

### Requirement: Initial new-document canvas render

A newly created document with a white background SHALL render pure white on its
first present, before any layer move, visibility change, or other recomposite.
The rendered canvas SHALL NOT show an uninitialized, stale, or otherwise
garbage frame on first use, and a subsequent recomposite SHALL produce the same
uniform result. The initial render SHALL use the same composite/readback seam as
every other composite, so the fix applies to the path all presenters route
through rather than to a single caller. The implementation SHALL first
reproduce the failure empirically, record the named root cause, and fix the
seam; the requirement SHALL be pinned by at least one runnable regression check
that composites a fresh white document twice and asserts both frames are uniform
white and equal.

#### Scenario: A new white document is pure white immediately

- **WHEN** a white-background document is created and its canvas is presented
  for the first time
- **THEN** every pixel of the presented canvas is fully opaque white, with no
  garbage frame

#### Scenario: A recomposite does not change the initial result

- **WHEN** the initial composite of a fresh white document is compared with a
  second composite of the same document
- **THEN** both frames are uniform white and equal

#### Scenario: The regression check fails on a garbage first frame

- **WHEN** the initial composite seam returns a non-uniform or non-white frame
  for a fresh white document
- **THEN** the regression check fails rather than passing vacuously

### Requirement: Damage account for partial repaint

The app's Rust view SHALL track outstanding display damage as an account that
distinguishes described regions from undescribed edits. A mutation that does not
describe its changed region SHALL mark the whole canvas as damaged. A described
mark SHALL only add to the damage and MUST NOT clear or mask an earlier
undescribed edit. Taking the damage SHALL clip it to the canvas and clear the
account. Any sequence of described and undescribed mutations SHALL leave the
presented composite byte-identical to a full recomposite.

#### Scenario: Nothing changed means nothing to redraw

- **WHEN** no mutation has occurred since the account was taken
- **THEN** taking the damage yields an empty region

#### Scenario: Described changes add up

- **WHEN** two described regions are marked without an intervening take
- **THEN** taking the damage yields their union, clipped to the canvas

#### Scenario: An undescribed change is the whole canvas

- **WHEN** a described region is marked and then an undescribed edit occurs
- **THEN** taking the damage yields the whole canvas

#### Scenario: A mark cannot hide an earlier undescribed change

- **WHEN** an undescribed edit occurs and then a small described region is marked
  before the canvas catches up
- **THEN** taking the damage yields the whole canvas, not the small region

#### Scenario: Taking starts afresh

- **WHEN** damage is taken and then taken again without any new mutation
- **THEN** the second take yields an empty region

### Requirement: Premultiplied display pixels

The Rust-to-Qt display conversion that hands canvas pixels to Qt SHALL produce
premultiplied RGBA, so Qt paints them without a per-frame conversion and averages
transparent edges correctly. The shared straight-alpha conversions used for
export encoding and for layer, mask, and histogram thumbnails MUST remain straight
alpha and MUST produce byte-identical output to before this change.

#### Scenario: A displayed pixel is premultiplied

- **WHEN** a straight-alpha composite pixel with alpha below 255 is converted for
  display
- **THEN** its color components equal `round(color * alpha / 255)` and its alpha
  is unchanged

#### Scenario: A transparent display pixel has zero color

- **WHEN** a composite pixel with alpha 0 is converted for display
- **THEN** its color components are 0

#### Scenario: Export and thumbnail bytes are unchanged

- **WHEN** the same composite is written through the export encoder or reduced for
  a layer, mask, or histogram thumbnail
- **THEN** the resulting bytes are identical to those produced before this change,
  not premultiplied

### Requirement: Zoom-dependent canvas sampling filter

When presenting a level crop below 200 % zoom the canvas SHALL sample it with
smooth (interpolated) filtering; at and above 200 % zoom it SHALL sample with
nearest-neighbour filtering so individual pixels stay crisp. The pixel-identity
guarantee compares two draws made with the same sampling filter; changing the
filter is an intended change of presented pixels, not a violation of it.

#### Scenario: Smooth below 200 %

- **WHEN** the canvas paints a document at a zoom below 200 %
- **THEN** the present draw uses smooth filtering

#### Scenario: Nearest at 200 % and above

- **WHEN** the canvas paints a document at 200 % zoom or greater
- **THEN** the present draw uses nearest-neighbour filtering

### Requirement: View-pyramid crop bridge

The view SHALL expose, across the Rust–Qt bridge, a crop of a chosen level and
queries for the level count and a level's size. Rectangles SHALL use the project's
`"x y w h"` string convention, levels SHALL be integers, and a level's size SHALL
use the project's `"w h"` string; no new marshalled rectangle type is introduced.
A crop MUST be premultiplied and sized to the requested rectangle. An empty
rectangle, an out-of-range level, or a rectangle that does not intersect the level
MUST return an empty image and MUST NOT panic. A rectangle that only partially
overlaps the level SHALL be returned at its requested size, transparent where it
falls outside the level.

#### Scenario: A bridge crop is a premultiplied rectangle

- **WHEN** a level and an in-range rectangle are requested across the bridge
- **THEN** the returned image has the rectangle's size, its pixels are that
  level's, premultiplied, and the reported level count and size match the pyramid

#### Scenario: An out-of-range request is empty, not a panic

- **WHEN** a crop is requested for a level index at or past the level count, with
  an empty rectangle, or with a rectangle that does not intersect the level
- **THEN** the bridge returns an empty image and does not panic

#### Scenario: A partially overlapping crop is transparent outside the level

- **WHEN** a rectangle that only partially overlaps the level is requested
- **THEN** the bridge returns a rectangle-sized image whose overlapping pixels are
  the level's, premultiplied, and whose outside pixels are transparent

### Requirement: Mid-stroke canvas present from the view pyramid

While a paint stroke is in progress the canvas SHALL present the same
zoom-selected view-pyramid level crop it presents when idle, rather than
rescaling the full-resolution document. The pyramid levels SHALL reflect the
stroke's in-progress pixels after every dab. The presented canvas after any dab
MUST be pixel-identical to the canvas a full recomposite of the stroke's working
document would produce at the same zoom and sampling filter. A document of any
size SHALL present through a level crop mid-stroke, and the repaint MUST NOT
resample the full-resolution document.

#### Scenario: A dab repaints through a level crop

- **WHEN** the canvas repaints after a dab while a stroke is in progress
- **THEN** it presents the level crop chosen for the zoom and does not rescale the
  full-resolution document

#### Scenario: The mid-stroke present equals a full recomposite

- **WHEN** a dab has been applied and the canvas is presented mid-stroke
- **THEN** the presented pixels are identical to a full recomposite of the
  stroke's working document at the same zoom and sampling filter

#### Scenario: A large document presents through a crop mid-stroke

- **WHEN** a stroke is painted on a 16000²-class document
- **THEN** each repaint crops a level rather than drawing the full-resolution
  source

### Requirement: Frame-bounded in-stroke present

While a paint stroke is in progress the app SHALL bound the present rate
instead of presenting once per input event. The view SHALL accumulate the dirty
rectangles of the dabs it receives after the most recent present into one
pending region, and SHALL present that pending region only when
`PictureView::flush_present()` runs. While a stroke has a pending region the app
SHALL invoke `flush_present()` at least once per frame interval. Committing a
stroke SHALL flush the pending region before the history state is recorded, and
cancelling a stroke SHALL drop it without presenting. A region refresh outside a
stroke SHALL still present immediately. The coalescing MUST NOT change the
pixels: after the flush the canvas SHALL equal a full recomposite of the stroke's
working document over the presented region.

#### Scenario: Dabs between two presents are presented together
[cv_dabs_between_presents_present_once]

- **WHEN** several dabs arrive after a present and before the frame's
  `flush_present()` during a stroke
- **THEN** none of them presents on arrival, and `flush_present()` presents one
  region covering all of them

#### Scenario: A consumer can force a present [cv_flush_present]

- **WHEN** `flush_present()` is called while a stroke has a pending region
- **THEN** the pending region is presented before the call returns and the
  pending region is cleared

#### Scenario: The commit flushes before recording [cv_commit_flushes_pending]

- **WHEN** a stroke with a pending region is released
- **THEN** the pending region is presented before the history state is recorded

#### Scenario: A cancelled stroke presents nothing pending [cv_cancel_drops_pending]

- **WHEN** a stroke with a pending region is cancelled
- **THEN** the pending region is not presented and the canvas is restored to the
  pre-stroke image

#### Scenario: An idle region refresh is unaffected [cv_idle_region_immediate]

- **WHEN** a region refresh runs outside a stroke
- **THEN** it presents immediately, exactly as it does without coalescing

### Requirement: Canvas presents the reduced level during a preview stroke

While a preview stroke is in progress the canvas SHALL crop the view-pyramid
level the preview wrote rather than the level the zoom would select, so the
whole in-progress image — not only the painted pixels — is shown at the preview
resolution. The preview SHALL repair that stored level directly and MUST NOT
rebuild it from level 0, because level 0 is deliberately left untouched until
the stroke ends. Releasing or cancelling the stroke SHALL restore the
zoom-selected level: the commit patches level 0 and rebuilds the stored levels
from it, which overwrites the preview.

#### Scenario: The canvas crops the preview level while previewing [cv_preview_present]

- **WHEN** a preview stroke presents a region
- **THEN** the canvas crops the view-pyramid level the preview wrote, that
  level is repaired in place rather than rebuilt from level 0, and level 0 is
  unchanged

#### Scenario: The commit restores the zoom-selected level [cv_preview_commit_restores_level]

- **WHEN** a previewed stroke is released
- **THEN** the exact region patches level 0, the stored levels are rebuilt from
  it, and the canvas returns to the level the zoom selects

#### Scenario: A cancelled preview restores the zoom-selected level [cv_preview_cancel_restores_level]

- **WHEN** a previewed stroke is cancelled
- **THEN** the pre-stroke document is presented and the canvas returns to the
  level the zoom selects

### Requirement: Canvas crops level 0 while a GPU stroke presents

While a GPU stroke is live, the canvas SHALL crop level 0 — whose region blits
keep it current — instead of the zoom-selected stored level. The present-level
signal SHALL distinguish this state (`-1`) from the preview level (a positive
stored level) and the idle zoom-selected level (`0`). Releasing or cancelling
the stroke SHALL restore the zoom-selected level once the commit or restore
rebuilds the pyramid.

#### Scenario: A live GPU stroke presents level 0 [cv_deferred_level0]

- **WHEN** the canvas paints while a GPU stroke is live
- **THEN** it crops level 0 rather than the zoom-selected stored level

#### Scenario: The commit restores the zoom-selected level [cv_deferred_commit]

- **WHEN** the GPU stroke is released
- **THEN** the pyramid is rebuilt from the committed level 0 and the canvas
  returns to the zoom-selected level

#### Scenario: An idle refresh is unaffected [cv_deferred_idle]

- **WHEN** no stroke is live
- **THEN** the present level is the zoom-selected level, exactly as before

### Requirement: Canvas pan suppresses tool overlays

While the canvas is being panned with Space or the middle mouse button, tool
overlays — including the brush-size circle — SHALL be hidden, and switching into
or out of pan SHALL repaint the canvas immediately. The pan gesture SHALL
override the active tool's cursor and pointer mode over the canvas and SHALL
apply over tool overlays.

#### Scenario: Brush ring hidden while panning [uidc_pan_hides_ring]

- **WHEN** the user holds Space (or middle-drags) over the brush-size circle
- **THEN** the ring is hidden, the hand cursor is shown, and the drag pans the canvas

### Requirement: Tool switch preserves canvas view

Switching tools SHALL preserve the canvas zoom and scroll position. The canvas
SHALL re-fit only on the first sizing after a document is loaded (or when Fit is
explicitly invoked), not on a resize caused by an options-bar height change.

#### Scenario: Switching tools keeps zoom and position [uidc_tool_switch_zoom]

- **WHEN** the user zooms and pans, then switches to another tool
- **THEN** the zoom level and canvas position are unchanged
