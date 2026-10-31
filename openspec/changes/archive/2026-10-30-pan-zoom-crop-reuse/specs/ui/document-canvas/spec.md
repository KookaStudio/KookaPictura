## MODIFIED Requirements

### Requirement: Zoom-cached canvas present

The canvas SHALL present the document from the view-pyramid level chosen for the
zoom instead of scaling the full-resolution document on every paint. For a
stored level below 0 it SHALL draw the level's pixels from the app-side borrowed
level view without copying the visible rectangle. For level 0, which is planar
and straight alpha, it SHALL draw a cached premultiplied crop and update that
crop from the changed document rectangle rather than re-cropping the viewport.
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

#### Scenario: A stored level is presented without a viewport copy [cvda_zero_copy_level]

- **WHEN** the zoom selects a stored level below 0 and the canvas paints
- **THEN** the paint draws the level's own pixels through a source rectangle and
  the viewport rectangle is not copied into a new image

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
- **THEN** the source is that level's pixels, a single reduction of the composite,
  and the paint does not rescale the full-resolution document

#### Scenario: A document past the former cache bound still presents through a crop

- **WHEN** a document whose scaled present would exceed 64 megapixels is painted
  at a high zoom
- **THEN** the paint presents the level selected for the zoom rather than
  drawing the full-resolution source, and does not fall back to the transform
  draw
