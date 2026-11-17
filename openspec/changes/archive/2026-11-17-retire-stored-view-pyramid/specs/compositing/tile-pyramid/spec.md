## MODIFIED Requirements

### Requirement: The viewport presents tiles from a tile pyramid cache

The canvas SHALL present a view by assembling the visible 256² tiles of the level
it is displaying from a cache keyed by source, level and tile position. Level-0
tiles SHALL hold straight-alpha sRGB pixels and tiles above level 0 SHALL hold
premultiplied pixels. A crop assembled from cached tiles SHALL equal the same crop
of an independent pyramid rebuilt from the whole display composite using
compositing/region-composite pixel semantics, byte for byte. That whole rebuild
SHALL be a verification reference, not retained production storage. The
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

## ADDED Requirements

### Requirement: Logical pyramid levels do not require stored images

The tile pyramid SHALL report level 0 at the document dimensions. Each next
level SHALL ceil-halve both dimensions and SHALL be available only while its
long side is at least 256 pixels and both dimensions are greater than 1.
Level count and dimensions SHALL be determined without constructing whole
level images. The availability rule SHALL NOT require the oracle in an
ordinary application build.

#### Scenario: A typical image has the expected logical levels [tp_level_sizes]

- **WHEN** levels are reported for a 1500x700 image
- **THEN** their sizes are 1500x700, 750x350 and 375x175, with no further level

#### Scenario: Ceil-halving includes a 256-long-side level [tp_level_511]

- **WHEN** levels are reported for a 511x300 image
- **THEN** their sizes are 511x300 and 256x150, with no further level

#### Scenario: A small image has only level zero [tp_level_small]

- **WHEN** levels are reported for a 300x200 image
- **THEN** only level 0 is available

#### Scenario: A thin image stops before a one-pixel dimension [tp_level_thin]

- **WHEN** levels are reported for a 4096x2 image or its transpose
- **THEN** only level 0 is available even though another ceil-halving would leave the long side above 256
