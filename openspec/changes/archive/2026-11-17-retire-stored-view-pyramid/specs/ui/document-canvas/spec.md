## MODIFIED Requirements

### Requirement: Mid-stroke canvas present from the view pyramid

While a paint stroke is in progress the canvas SHALL present the same
zoom-selected tile-pyramid level crop it presents when idle, rather than
rescaling the full-resolution document. The presented tile crops SHALL reflect the
stroke's in-progress pixels at each frame-bounded present. The presented canvas after any dab
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

### Requirement: Canvas presents the reduced level during a preview stroke

While a preview stroke is in progress the canvas SHALL crop the tile-pyramid
level the preview wrote rather than the level the zoom would select, so the
whole in-progress image — not only the painted pixels — is shown at the preview
resolution. The preview SHALL patch the cached tiles at that level directly and MUST NOT
replace them with exact tiles from level 0 while the preview is live, because level 0 is deliberately left untouched until
the stroke ends. Releasing or cancelling the stroke SHALL restore the
zoom-selected level: the commit repairs or invalidates affected cached tiles from the exact document,
which replaces the preview.

#### Scenario: The canvas crops the preview level while previewing [cv_preview_present]

- **WHEN** a preview stroke presents a region
- **THEN** the canvas crops the tile-pyramid level the preview wrote, that
  level is repaired in place rather than rebuilt from level 0, and level 0 is
  unchanged

#### Scenario: The commit restores the zoom-selected level [cv_preview_commit_restores_level]

- **WHEN** a previewed stroke is released
- **THEN** the exact region repairs or invalidates the affected tiles at every cached level, and the canvas returns to the level the zoom selects

#### Scenario: A cancelled preview restores the zoom-selected level [cv_preview_cancel_restores_level]

- **WHEN** a previewed stroke is cancelled
- **THEN** the pre-stroke document is presented and the canvas returns to the
  level the zoom selects

## REMOVED Requirements

### Requirement: Zoom-cached canvas present

**Reason**: Borrowing whole stored reduced levels conflicts with tile-cache presentation.

**Migration**: Use Tile-cached canvas present; existing scenario tags remain traceable in the replacement.

### Requirement: Canvas presents the zoom-selected level during a GPU stroke

**Reason**: The old scenario requires repairing every whole stored level after a GPU dab.

**Migration**: Use GPU strokes present through the tile pyramid; retain zoom behavior and compare exact cached tiles with the independent reference.

## ADDED Requirements

### Requirement: Tile-cached canvas present

The canvas SHALL present the document from the logical tile-pyramid level chosen
for the zoom instead of scaling the full-resolution document on every paint. At
every level it SHALL draw a cached premultiplied crop assembled from the
requested tiles. An unchanged crop SHALL be reused, and damage SHALL update or
invalidate affected crop pixels without requiring a whole stored level.
When the visible document rectangle changes at an unchanged level — a pan or a
zoom step that does not cross a level boundary — the canvas SHALL reuse the
overlap of the cached crop and crop only the newly exposed band(s) from the
level provider, instead of re-cropping the whole viewport; the reused crop MUST
be byte-identical to a fresh crop of that level over the same rectangle. The
canvas SHALL choose the coarsest level whose scale is still at least as fine as
the screen: level 0 for zoom > 0.5; level 1 for 0.25 < zoom <= 0.5; level 2 for
0.125 < zoom <= 0.25; and so on. A pan or hover repaint MUST NOT resample the
full-resolution document. There SHALL be no pixel-count bound that disables the
level present path: a document of any size SHALL present through a level.

#### Scenario: A pan repaint does not resample the document

- **WHEN** the view is panned without changing the zoom
- **THEN** the paint presents the level chosen for the zoom and does not rescale
  the full-resolution document

#### Scenario: A reduced level reuses its assembled tile crop [cvda_zero_copy_level]

- **WHEN** the zoom selects a reduced level and the canvas paints the same current rectangle twice
- **THEN** the second paint reuses its assembled tile crop without reconstructing a whole level or re-copying the unchanged crop

#### Scenario: A level-0 present reuses the cached crop [cvda_level0_crop_reused]

- **WHEN** the zoom selects level 0 and the canvas paints twice at the same
  document rectangle
- **THEN** the second paint reuses the cached premultiplied crop instead of
  re-cropping the document

#### Scenario: A pan at level 0 reuses the cached crop [cvda_pan_reuses_crop]

- **WHEN** the zoom selects level 0, the canvas paints, and then the view pans
  without changing the zoom
- **THEN** the next paint reuses the cached crop's overlap and crops only the
  newly exposed band, and the presented pixels are byte-identical to a fresh
  level-0 crop over the new rectangle

#### Scenario: A zoom step at an unchanged level reuses the cached crop [cvda_zoom_reuses_crop]

- **WHEN** the canvas paints at a zoom that selects level 0 and then zooms in
  without crossing the level-0 boundary
- **THEN** the next paint reuses the cached crop (the new rectangle is inside
  the cached one) and the presented pixels are byte-identical to a fresh
  level-0 crop over the new rectangle

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
- **THEN** the source is that level's exact pixels, reduced according to compositing/region-composite,
  and the paint does not rescale the full-resolution document

#### Scenario: A document past the former cache bound still presents through a crop

- **WHEN** a document whose scaled present would exceed 64 megapixels is painted
  at a high zoom
- **THEN** the paint presents the level selected for the zoom rather than
  drawing the full-resolution source, and does not fall back to the transform
  draw

### Requirement: GPU strokes present through the tile pyramid

While a GPU stroke is live the canvas SHALL present the logical tile-pyramid
level selected by zoom, using current tiles from the stroke's working document.
The present-level signal SHALL report the reduced preview level while a preview
is running and `0` when no preview is active, without a distinct GPU state.
Every dab's read-back rectangle SHALL reach the same damage-driven tile repair
or invalidation path as CPU damage. Exact cached tiles affected by the dab SHALL
match the independent whole-rebuild reference before being presented as current.
Releasing or cancelling the stroke SHALL leave the canvas on the zoom-selected
level. A whole stored level SHALL NOT be required.

#### Scenario: A live GPU stroke presents the zoom-selected level [cvda_gpu_zoom_level]

- **WHEN** the canvas paints while a GPU stroke is live at a zoom that selects a reduced level
- **THEN** it presents that tile-pyramid level rather than forcing level 0

#### Scenario: A GPU dab repairs affected cached tiles [cvda_gpu_dab_in_pyramid]

- **WHEN** a GPU dab changes a rectangle of the working document
- **THEN** affected cached tiles are repaired or made stale, and exact tiles presented as current match the independent reference at every requested level

#### Scenario: An idle refresh is unaffected [cvda_gpu_idle]

- **WHEN** no stroke is live
- **THEN** the present level is the zoom-selected level, exactly as before
