# document/source-backed-tiles Specification

## Purpose
A tile still matching the file a document was opened from is backed by it: it is dropped without a write and decoded again, the file stays pinned while the document lives, and the merged image stands for the layers on the canvas until the first edit.

## Requirements

### Requirement: An unedited backed tile drops without a write and decodes again

A tile backed by its source file SHALL start away and SHALL decode from the
source on its first read. Evicting it while it still matches the source SHALL
free its samples without packing or writing scratch, and a later read SHALL
decode the same samples again. The first write SHALL detach the tile from its
source, after which it packs and spills as any dense tile. A transient read
SHALL decode an away backed tile without keeping its samples resident.

#### Scenario: A dropped tile decodes to the same samples [sb_redecode]

- **WHEN** a backed tile is read, evicted, and read again
- **THEN** both reads return the same samples and the eviction wrote nothing

#### Scenario: A write detaches the tile [sb_detach]

- **WHEN** a backed tile is written and then evicted
- **THEN** it is packed or spilled, not dropped, and reads back the written
  samples

#### Scenario: A transient read keeps nothing [sb_transient]

- **WHEN** an away backed tile is read under a transient scope
- **THEN** the read returns the decoded samples and the tile is still away

### Requirement: The evictor frees backed tiles first

An eviction pass SHALL free cold backed tiles, held by history or not,
before any other dense tile, after the display cache.

#### Scenario: Backed tiles go before live ones [sb_priority]

- **WHEN** a pass needs one tile's bytes with a resident backed tile and a
  resident live tile, both cold
- **THEN** the backed tile is freed and the live tile stays resident

### Requirement: The source stays pinned while the document lives

A document opened lazily SHALL keep its source file's descriptor open and
SHALL decode through it. A source replaced by a rename or deleted SHALL leave
every backed tile reading the data it was opened with. A source rewritten in
place, detected by its descriptor's size or modification time, SHALL make
every later decode fail, so its unread tiles read as unreadable and the edit
or save that reads them is refused with the typed error.

#### Scenario: A replaced source still reads [sb_replaced]

- **WHEN** another file is renamed over a lazily opened source and the backed
  tiles are read
- **THEN** they hold the samples of the file that was opened

#### Scenario: A deleted source still reads [sb_deleted]

- **WHEN** a lazily opened source is deleted and the backed tiles are read
- **THEN** they hold the samples of the file that was opened

#### Scenario: A source rewritten in place fails its unread tiles [sb_modified]

- **WHEN** a lazily opened source is rewritten in place with different
  contents and an unread backed tile is read
- **THEN** the tile reads as unreadable

#### Scenario: Save over the source with backed tiles [sb_save_over]

- **WHEN** a lazily opened PSD is saved over its own path, one layer is then
  edited, and it is saved again
- **THEN** the second file equals the same edit made on an eager open, saved

### Requirement: The first picture is the file's merged image until the first edit

A lazily opened document with a merged preview SHALL build its display tiles
from the preview, compositing no layer, until the first change to the
document. That change SHALL drop the preview and rebuild the display from the
layers. While the preview is shown, every reader of the picture other than
the display (saves, exports, Copy Merged, sampling) SHALL composite the
layers.

#### Scenario: The first paint reads the preview [sb_first_paint]

- **WHEN** a lazily opened document with a preview builds its fit-to-screen
  tiles
- **THEN** the tiles equal the preview's pyramid and no layer tile was decoded

#### Scenario: An edit replaces the preview [sb_first_edit]

- **WHEN** a layer of that document is edited
- **THEN** the preview is dropped and the display tiles equal the layer
  composite

#### Scenario: The picture composites the layers [sb_picture]

- **WHEN** the whole picture is read while the preview is shown
- **THEN** it equals the layer composite, not the preview

#### Scenario: M12 opens lazily within its bounds [sb_m12]

- **WHEN** `scripts/profile-tools.py` runs M12 at 17000² with an eager and a
  lazy open, and M24 (open, pan across, fill on one layer) under
  `MemoryMax=2G` and without a limit
- **THEN** the lazy open's time to its first fit-to-screen picture and its
  peak resident size are recorded against the eager open's, and M24's open,
  pan and fill complete under the limit with the same layer pixels as without
  it
