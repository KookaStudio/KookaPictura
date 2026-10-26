# Spec Delta

## MODIFIED Requirements

### Requirement: Zoom-cached canvas present

The canvas SHALL present the document from the view-pyramid level chosen for the
zoom instead of scaling the full-resolution document on every paint. For a
stored level below 0 it SHALL draw the level's pixels from the app-side borrowed
level view without copying the visible rectangle. For level 0, which is planar
and straight alpha, it SHALL draw a cached premultiplied crop and update that
crop from the changed document rectangle rather than re-cropping the viewport.
The canvas SHALL choose the coarsest level whose scale is still at least as fine
as the screen:
level 0 for zoom > 0.5; level 1 for 0.25 < zoom <= 0.5; level 2 for
0.125 < zoom <= 0.25; and so on. A pan or hover repaint MUST NOT resample the
full-resolution document. There SHALL be no pixel-count bound that disables the
level present path: a document of any size SHALL present through a level.

#### Scenario: A pan repaint does not resample the document

- **WHEN** the view is panned without changing the zoom
- **THEN** the paint presents the level chosen for the zoom and does not rescale
  the full-resolution document

#### Scenario: A stored level is presented without a viewport copy [cvda_zero_copy_level]

- **WHEN** the zoom selects a stored level below 0 and the canvas paints
- **THEN** the paint draws the level's own pixels through a source rectangle and
  the viewport rectangle is not copied into a new image

#### Scenario: A level-0 present reuses the cached crop [cvda_level0_crop_reused]

- **WHEN** the zoom selects level 0 and the canvas paints twice at the same
  document rectangle
- **THEN** the second paint reuses the cached premultiplied crop instead of
  re-cropping the document

#### Scenario: A zoom change invalidates the cache

- **WHEN** the zoom changes across a level boundary
- **THEN** the next paint chooses the corresponding level and presents it

#### Scenario: Cached present matches the transform draw

- **WHEN** the canvas paints a document at zoom 100 % or above (level 0) through
  the present path and through a direct full-resolution transform draw, with the
  same sampling filter
- **THEN** the two rendered images are pixel-identical

#### Scenario: A coarse-level present is a single reduction

- **WHEN** the canvas paints at a zoom that selects a level below 0
- **THEN** the source is that level's pixels, a single reduction of the composite,
  and the paint does not rescale the full-resolution document

#### Scenario: A document past the former cache bound still presents through a crop

- **WHEN** a document whose scaled present would exceed 64 megapixels is painted
  at a high zoom
- **THEN** the paint presents the level selected for the zoom rather than
  drawing the full-resolution source, and does not fall back to the transform
  draw

## REMOVED Requirements

### Requirement: C++ region blit and present-cache coherence

**Reason**: A document region refresh now notifies the canvas of the changed
document rectangle instead of building a region image the canvas never reads; the
pixel blit survives only for a canvas that holds its own image. The replacement
requirement, `C++ damage notification and present-cache coherence`, carries the
still-valid clauses of this one.

**Migration**: Document canvases consume `regionDamaged(x, y, w, h)`;
`regionBlitted(QImage, x, y)` and `ImageView::blitRegion` are used only by the
non-document `image_` fallback. Shell and test code that read the blit image for
a document switches to the damage rectangle.

### Requirement: Canvas crops level 0 while a GPU stroke presents

**Reason**: The pyramid update already repairs every stored level after each GPU
dab, so keeping the present path on level 0 only forces the whole visible
document through a straight-alpha to premultiplied conversion every frame. The
canvas instead presents the zoom-selected stored level, which is available
zero-copy.

**Migration**: `preview_present_level()` no longer reports `-1` for a live GPU
stroke; it reports the preview level while a reduced preview runs and `0`
otherwise. The canvas keeps the zoom-selected level for a GPU stroke, exactly as
it does for an exact stroke, and the existing region refresh keeps that level
current.

## ADDED Requirements

### Requirement: C++ damage notification and present-cache coherence

For a non-document canvas that holds its own image, the view SHALL provide a C++
region blit (`ImageView::blitRegion(const QImage& region, int x, int y)`) that
draws the region at `(x, y)` using `QPainter` with `CompositionMode_Source`. The
blit SHALL keep the transparency checkerboard, the document clip, and the
pan/zoom behaviour unchanged, and SHALL schedule a repaint. A region refresh for
a document canvas SHALL instead notify the canvas of the changed document
rectangle (`regionDamaged(x, y, w, h)`) and MUST NOT build, convert, or pass a
region `QImage` for that canvas. A filter commit SHALL route its clipped
rectangle through the same damage notification and MUST NOT build or send a
marker pixel image. The view pyramid SHALL be updated from a region
refresh by the app's Rust view through the damage account, so the next paint
presents the updated levels; the C++ side MUST NOT be responsible for updating
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
- **THEN** a document canvas receives the changed document rectangle through the
  damage notification, no region `QImage` is created for that canvas, the app's
  Rust view updates the affected pyramid levels, and the next paint presents
  those updated levels

#### Scenario: A filter commit presents through damage

- **WHEN** a filter commit lands a clipped region on a document canvas
- **THEN** the canvas receives `regionDamaged` at that rectangle, no marker
  `regionBlitted` is emitted, and the next paint presents the filtered region

#### Scenario: A non-document canvas still blits pixels [cvda_pixel_blit_fallback]

- **WHEN** a region blit is emitted for a canvas that holds its own image
- **THEN** `ImageView::blitRegion` draws the region over that image at `(x, y)`
  and the canvas repaints

#### Scenario: The on-screen canvas after a region refresh matches a full recomposite

- **WHEN** a region refresh is followed by a forced full recomposite of the same
  document
- **THEN** the canvas image before the recomposite is pixel-identical to the image
  after it

#### Scenario: image() rebuilds from the composite only when dirty

- **WHEN** `image()` is called after a region refresh and again after a full
  recomposite
- **THEN** both calls return an image built from the cached level-0 frame and
  byte-identical to a full recomposite, and no persistent full-resolution `QImage`
  is held between calls

#### Scenario: sample_argb reads the current composite

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

#### Scenario: The repaint path does not depend on image()

- **WHEN** the canvas repaints after a region refresh at any zoom, before any
  caller has consumed `image()`
- **THEN** it presents view-pyramid level pixels and does not build or read a
  full-resolution image

### Requirement: Canvas presents the zoom-selected level during a GPU stroke

While a GPU stroke is live the canvas SHALL present the level the zoom selects,
using the pixels the stroke's region refresh folded into the pyramid. The
present-level signal SHALL report the preview level while a reduced preview is
running and `0` when no preview is active, and MUST NOT report a distinct GPU
state. Every dab's read-back rectangle SHALL reach the pyramid through the same
damage-driven update the CPU path uses. Releasing or cancelling the stroke SHALL
leave the canvas on the zoom-selected level.

#### Scenario: A live GPU stroke presents the zoom-selected level [cvda_gpu_zoom_level]

- **WHEN** the canvas paints while a GPU stroke is live at a zoom that selects a
  stored level
- **THEN** it presents that stored level, not level 0

#### Scenario: A GPU dab reaches every stored level [cvda_gpu_dab_in_pyramid]

- **WHEN** a GPU dab changes a rectangle of the working document
- **THEN** the pyramid update repairs every stored level over that rectangle
  before the canvas presents it

#### Scenario: An idle refresh is unaffected [cvda_gpu_idle]

- **WHEN** no stroke is live
- **THEN** the present level is the zoom-selected level, exactly as before

### Requirement: Damage-patched level-0 crop

While the cached crop is for level 0, a document damage notification SHALL patch
only the affected part of the cached crop and MUST NOT re-crop the viewport. The
patch SHALL map the damage rectangle to level coordinates, read that rectangle of
level 0, and write it into the cached crop. The patched crop MUST be byte-identical
to a fresh crop of the same level over the same document rectangle. A damage
rectangle outside the cached document rectangle MUST NOT change the crop, and a
damage arriving for a different level than the cached crop SHALL discard the
cache so the next paint presents the correct level.

#### Scenario: A patch equals a fresh crop [cvda_patch_matches_crop]

- **WHEN** a damage rectangle inside the cached document rectangle is patched
  into a level-0 crop
- **THEN** the patched crop bytes equal a fresh crop of level 0 over the same
  document rectangle

#### Scenario: Damage outside the cached rectangle changes nothing [cvda_patch_outside]

- **WHEN** a damage rectangle does not intersect the cached document rectangle
- **THEN** the cached crop is unchanged

#### Scenario: A level change discards the crop [cvda_patch_level_change]

- **WHEN** a damage arrives while the zoom or a preview selects a level other
  than the cached crop's level
- **THEN** the cached crop is discarded and the next paint presents the selected
  level
