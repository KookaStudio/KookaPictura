# compositing/view-pyramid Specification

## Purpose
A progressively halved view of the document composite that the canvas and panels
crop from, so drawing never rescales the full-resolution document.

## Requirements

### Requirement: Halved-level view structure

The view SHALL store a sequence of progressively halved levels, each level's
long side being half the previous one, stopping before the next level's long
side would fall below 256 pixels. Level 0 (the document's own composite buffer)
SHALL be supplied to the view by the caller on each call and MUST NOT be stored
or copied. Every level below 0
MUST be premultiplied alpha, so averaging a level cannot darken soft edges; a
crop of any level SHALL be returned premultiplied for display. The structure
SHALL report its level count, each level's size, and whether it was built for a
given document size.

#### Scenario: Levels halve down to the smallest side

- **WHEN** a 1500×700 composite is built into the view structure
- **THEN** the levels are 1500×700, 750×350, and 375×175, and no further level is
  added

#### Scenario: A 511-long-side composite still gets a 256 level

- **WHEN** a composite whose long side is 511 pixels is built into the view
  structure
- **THEN** it gains a level whose long side is 256 pixels and no further level,
  because the loop stops only when the next long side would fall below 256

#### Scenario: A small composite is its own only level

- **WHEN** a composite whose long side is 300 pixels is built into the view
  structure
- **THEN** it has exactly one level

#### Scenario: Level 0 is supplied, not stored

- **WHEN** the view is built or updated for a document
- **THEN** it reads level 0 from the supplied buffer and allocates no
  full-resolution copy

#### Scenario: Levels below 0 are premultiplied

- **WHEN** a level below 0 is read after a build
- **THEN** each pixel's color components are the straight-alpha color multiplied
  by its alpha, so averaging cannot darken soft edges

### Requirement: Damage-driven level update

Given the current straight-alpha level-0 pixels and a rectangle in document
pixels, the view SHALL recompute only the region of each level below 0 that the
change can affect, using the changed level-0 pixels. The result MUST be byte-identical to rebuilding every level from
the changed composite. The rectangle SHALL be clipped to level 0's bounds, and
an empty region MUST be a no-op.

#### Scenario: An update equals a rebuild at every level

- **WHEN** two regions of a composite are updated through the damage path
- **THEN** every level is byte-identical to the same composite built from
  scratch after the same two regions are replaced in the source pixels

#### Scenario: An empty region changes nothing

- **WHEN** the update is called with an empty rectangle
- **THEN** every level is unchanged

#### Scenario: Damage is clipped to level 0

- **WHEN** the update rectangle extends past the document's bounds
- **THEN** the recomputed region is clipped and no out-of-bounds write occurs

### Requirement: Viewport crop

For a level index and a rectangle in that level's pixel coordinates, the view
SHALL return a rectangle-sized premultiplied copy of those pixels without
materializing any other level. Any part of the rectangle outside the level MUST
come back transparent.

#### Scenario: A crop returns that level's pixels

- **WHEN** a rectangle fully inside a level is cropped
- **THEN** the copy equals that rectangle of the level, premultiplied

#### Scenario: A level-0 crop is premultiplied

- **WHEN** level 0, which is straight alpha, is cropped for display
- **THEN** each returned pixel's color is its straight color multiplied by its
  alpha, so the crop matches the display format

#### Scenario: A crop past the level is transparent

- **WHEN** a rectangle extends past the level's bounds
- **THEN** the overlapping pixels are the level's and the rest are transparent

### Requirement: Tile-aligned damage update

The view structure SHALL round a damage rectangle out to the 64×64 tile grid
before recomputing the levels below 0, then expand it outward per level by the
2× filter footprint a level-`N` texel reads from `N-1`. The aligned update MUST
remain byte-identical to a full rebuild, and level 0 MUST stay the caller's
single supplied buffer — the alignment MUST NOT copy or detach a full-frame
buffer.

#### Scenario: A non-aligned damage rect still equals a rebuild [vp_tile_update]

- **WHEN** an update is called with a rectangle whose edges are not tile-aligned
- **THEN** every level below 0 equals a rebuild of the same level-0 pixels

#### Scenario: The damage rect is rounded outward [vp_tile_align]

- **WHEN** a damage rectangle of 10×20 at (130, 70) is aligned on a 64-pixel grid
- **THEN** it becomes the rectangle (128, 64)–(192, 128), covering the original
  rectangle

#### Scenario: Level 0 is not detached [vp_tile_no_detach]

- **WHEN** a tile-aligned update runs
- **THEN** level 0 is read from the supplied buffer and no full-frame copy is
  made

### Requirement: Borrowed zero-copy level view

For a stored level below 0, the view SHALL expose that level's premultiplied
interleaved pixels as a std-only borrowed level handle that shares the level's
storage instead of copying the level. The handle keeps the storage alive; a
consumer MUST drop the handle when it is done. The Qt display image is built
app-side from this handle, so `pictura-render` gains no Qt dependency. When a
level is updated while a handle is alive, the handle MUST keep showing the bytes
it was created with and the update SHALL write to fresh storage (copy on write);
a handle created after the update SHALL show the new pixels. When no handle is
alive, an update MUST write in place and MUST NOT copy the level. Level 0 has no
stored interleaved buffer and MUST NOT be exposed as a borrowed handle.

#### Scenario: A borrowed view shares the level bytes [vp_view_shares_bytes]

- **WHEN** a borrowed level handle is requested for a stored level
- **THEN** its bytes equal that level's pixels and it reads the level's own
  storage rather than a new copy

#### Scenario: A view survives an update unchanged [vp_view_copies_on_write]

- **WHEN** a stored level is updated while a borrowed level handle of it is alive
- **THEN** the handle still shows the pixels it was created with, and a newly
  created handle shows the updated pixels

#### Scenario: An update without a view copies nothing [vp_view_update_in_place]

- **WHEN** a stored level is updated and no borrowed level handle of it is alive
- **THEN** the update writes the level's own storage in place and allocates no
  second level buffer

#### Scenario: Level 0 is not borrowed [vp_view_no_level0]

- **WHEN** a borrowed level handle is requested for level 0
- **THEN** the view reports that level 0 has no stored interleaved buffer rather
  than exposing the planar source
