# compositing/tile-pyramid Specification

## Purpose
The canvas display as a cache of 256² tiles per source, pyramid level and
tile: built on demand from the composite or a soloed mask, repaired or made
stale by damage, kept to a recomputable memory budget, and presented without
blocking on a large document, so no full-resolution display frame is kept.

## Requirements

### Requirement: The viewport presents tiles from a tile pyramid cache

The canvas SHALL present a view by assembling the visible 256² tiles of the level
it is displaying from a cache keyed by source, level and tile position. Level-0
tiles SHALL hold straight-alpha sRGB pixels and tiles above level 0 SHALL hold
premultiplied pixels. A crop assembled from cached tiles SHALL equal the same crop
of a pyramid rebuilt from the whole display composite, byte for byte. The
application MUST NOT keep a full-resolution display frame or whole stored pyramid
levels.

#### Scenario: Visible tiles match a whole rebuild [tp_crop]

- **WHEN** the canvas requests a crop at levels 0 through 3 of a document whose tiles are built on demand
- **THEN** each crop equals the same crop of a pyramid rebuilt from the whole display composite

#### Scenario: Only the visible tiles are built [tp_visible_only]

- **WHEN** a large document is presented at 100 % in a small viewport
- **THEN** only the tiles under the view and its margin are composited and resident

#### Scenario: No full-resolution display frame is kept [tp_no_frame]

- **WHEN** a large layered document is opened and presented
- **THEN** no buffer of the document's full display size is resident afterward

### Requirement: Damage invalidates and repairs tiles

An edit's damage rectangle SHALL repair every cached tile it touches at every
level from the composited damage region, or, when that region is too large to
composite at once on a large document, SHALL mark the cached tiles under it
stale, so a later request rebuilds them. A whole-document change SHALL
invalidate every tile. A tile built by a worker for a state that damage has since
superseded MUST NOT be installed. After any sequence of edits, undo, redo and
history restores, every presented tile SHALL equal a rebuild from the current
composite.

#### Scenario: An edit repairs the visible tiles [tp_damage]

- **WHEN** a region is painted while the canvas shows it at level 0 and at a reduced level
- **THEN** the tiles under the damage equal a rebuild and tiles outside it are not rebuilt

#### Scenario: A stale worker result is dropped [tp_stale]

- **WHEN** a tile request is outstanding and an edit damages that tile before the result arrives
- **THEN** the result is discarded and the tile is rebuilt from the edited state

#### Scenario: Undo and restore present current tiles [tp_history]

- **WHEN** undo, redo and a named-snapshot restore run while tiles are cached
- **THEN** every presented tile equals a rebuild from the restored state

### Requirement: Tile cache memory is recomputable and dropped first

The tile cache SHALL be a governor consumer with a budget of the smaller of
192 MiB and a quarter of the memory ceiling, never below the pinned tiles of the
presented view. Above the budget it SHALL evict unpinned level-0 tiles before
unpinned tiles of other levels, least recently used first. Under high memory
pressure it SHALL drop every unpinned tile. It SHALL report resident bytes, hits,
misses and evictions.

#### Scenario: The cache stays within its budget [tp_budget]

- **WHEN** tiles are requested across a document larger than the budget
- **THEN** resident tile bytes never exceed the budget, and evictions take level-0 tiles first

#### Scenario: Pressure drops unpinned tiles [tp_pressure]

- **WHEN** the governor reports high pressure
- **THEN** every unpinned tile is dropped and the presented view's tiles remain

#### Scenario: Hits and misses are counted [tp_counters]

- **WHEN** the same view is presented twice
- **THEN** the second presentation is all hits and the counters report it

### Requirement: Missing tiles are presented without blocking

For a document larger than 16 million pixels, a missing visible tile SHALL be
filled at once from its own stale bytes or the nearest cached coarser tile of the
same source (transparent when there is neither), and its exact tile SHALL be
built on a worker and then presented. A stale tile MUST NOT be served as current. Smaller documents SHALL build missing tiles synchronously. Opening a
document SHALL build the tiles of its first view without a full-resolution
composite.

#### Scenario: A large document's missing tile does not block [tp_async]

- **WHEN** a large document pans to tiles that are not cached
- **THEN** the view is presented at once from coarser tiles and the exact tiles replace them when the worker finishes

#### Scenario: A small document builds tiles synchronously [tp_sync_small]

- **WHEN** a document of at most 16 million pixels is presented
- **THEN** every visible tile is exact on the first presentation

### Requirement: A soloed mask is presented from mask tiles

A soloed raster mask SHALL be presented from tiles of the tile pyramid whose
source is the mask plane. Entering solo, undo, redo and whole-document restore
while soloed MUST NOT build a document-sized display plane or an exact
whole-document pyramid synchronously. The visible mask tiles SHALL equal the
mask's exact whole pyramid, and leaving solo SHALL present the document's
composite tiles.

#### Scenario: A restored solo mask presents visible tiles [tp_solo_restore]

- **WHEN** a whole-document restore runs while a mask is soloed
- **THEN** the visible mask tiles equal the exact whole-mask reference, and switching out of solo presents the restored document view

#### Scenario: Solo restore stays off the GUI thread [tp_solo_budget]

- **WHEN** a 16507×16196 document with a soloed mask is restored in a 1920×1080 viewport
- **THEN** the visible mask is presented within 50 ms of GUI-thread time and the exact tiles complete on workers
