# Spec Delta

## Purpose

Writes a document's composite as a Zoomify tile pyramid with a self-contained
HTML viewer page, so a high-resolution image can be published for web
pan-and-zoom viewing.

## ADDED Requirements

### Requirement: Zoomify command and dialog

`File ▸ Export ▸ Zoomify…` SHALL be enabled when a document is open and SHALL
open one modal dialog with an **Output Location** folder, **Browser Options**
base width and height, an **Image Tile Options** JPEG quality, a **Template**
menu offering the built-in `Default` viewer, `Cancel`, and `Export`. The base
width and height SHALL default to the document size and SHALL be clamped to the
document size; a size of zero or less SHALL be refused. The JPEG quality SHALL
range from 0 to 100 and default to 75. Opening the dialog and cancelling it
SHALL leave the document's pixels, path, modified state, and history unchanged.

#### Scenario: The dialog presents the export controls [zf_dialog]

- **WHEN** the user opens `File ▸ Export ▸ Zoomify…` on an open document
- **THEN** the dialog shows Output Location, Browser Options base width and height, Image Tile Options quality, a Template menu, and Cancel and Export

#### Scenario: The dialog carries the documented defaults [zf_dialog_defaults]

- **WHEN** the Zoomify dialog opens on a 1200 × 800 document
- **THEN** the base width and height are 1200 and 800, the quality is 75, and the allowed quality range is 0 to 100

#### Scenario: The base size is bounded [zf_zero_base]

- **WHEN** the user sets a base dimension to 0 or to a value above the document size
- **THEN** zero is refused and a larger value is clamped to the document dimension

#### Scenario: Opening Zoomify changes nothing [zf_dialog_unchanged]

- **WHEN** the user opens the Zoomify dialog and cancels
- **THEN** the document's pixels, path, modified state, and history are unchanged

#### Scenario: The command is disabled without a document [zf_no_document]

- **WHEN** no document is open
- **THEN** `File ▸ Export ▸ Zoomify…` is disabled

### Requirement: The tile pyramid follows the Zoomify layout

The export SHALL treat the Browser Options size as the base image `W × H`. It
SHALL build `levels = ceil(log2(max(W, H) / 256)) + 1` tiers, numbered from 0
(smallest) to `levels - 1` (the base image), where tier `z` is
`ceil(W / 2^(levels - 1 - z)) × ceil(H / 2^(levels - 1 - z))`. Each tier SHALL
be cut into 256 × 256 tiles in row-major order, tiles SHALL be numbered from
tier 0 upward, and tile `n` SHALL be written to
`TileGroup{floor(n / 256)}/{z}-{x}-{y}.jpg` under the output folder. Any edge
tile SHALL keep its full cell index and contain only the covered pixels. The
export SHALL write `ImageProperties.xml` with the attributes `WIDTH`, `HEIGHT`,
`NUMTILES` (the total tile count), `NUMIMAGES="1"`, `VERSION="1.8"`, and
`TILESIZE="256"`. Tiles SHALL be JPEG encoded at the chosen quality with the
composite flattened onto white.

#### Scenario: A single-tier image writes one tile [zf_one_tier]

- **WHEN** the base size is 200 × 100
- **THEN** `TileGroup0/0-0-0.jpg` is written and `ImageProperties.xml` reports `NUMTILES="1"`

#### Scenario: A multi-tier image writes every tile [zf_multi_tier]

- **WHEN** the base size is 1000 × 700
- **THEN** three tiers hold 1, 4, and 12 tiles, `NUMTILES="17"`, and the last-tier tile at the bottom-right edge is `TileGroup0/2-3-2.jpg`

#### Scenario: Tile groups roll over at 256 tiles [zf_tile_group_rollover]

- **WHEN** the layout produces 257 or more tiles
- **THEN** tile 256 is written under `TileGroup1/` and the preceding tiles under `TileGroup0/`

#### Scenario: Edge tiles keep the full 256-pixel cell [zf_edge_tiles]

- **WHEN** a tier is 300 × 200
- **THEN** its bottom-right tile covers 44 × 200 pixels and is written as a 44 × 200 JPEG at its full cell index

#### Scenario: Transparency is flattened onto white [zf_white_background]

- **WHEN** the composite has a transparent pixel
- **THEN** the corresponding tile pixel is opaque white

### Requirement: The viewer page is self-contained

The export SHALL write an HTML page next to the tiles that displays the base
image and supports panning and zooming through the tile pyramid. The page SHALL
reference only files inside the output folder, SHALL NOT load network resources,
and SHALL work when opened from the filesystem without a network request. It
SHALL use the base width and height and the tile size the export wrote.

#### Scenario: The page references the exported tiles [zf_viewer_tiles]

- **WHEN** Zoomify export completes
- **THEN** the output folder contains an HTML page that references `ImageProperties.xml` or the tile paths and the base dimensions

#### Scenario: The page has no external resources [zf_viewer_offline]

- **WHEN** the exported HTML is inspected
- **THEN** it contains no absolute URL, remote script, remote stylesheet, `fetch`, `XMLHttpRequest`, or module import

### Requirement: The export is bounded and non-mutating

The export SHALL hold at most the base-size image, one tier, and one encoded
tile at a time. It SHALL run as a cancellable job that reports progress and
stops between tiles when cancelled. A cancelled or failed export SHALL leave
the document unchanged; tiles already written MAY remain. The export SHALL NOT
change the document's path, modified state, or history, and SHALL NOT add a
history state. Files with names the export writes SHALL be replaced; other
files in the output folder SHALL be left untouched.

#### Scenario: A cancelled Zoomify export changes nothing [zf_cancel]

- **WHEN** the user cancels a Zoomify export while it is writing tiles
- **THEN** the document is unchanged and no history state is added

#### Scenario: A successful export leaves the document alone [zf_unchanged]

- **WHEN** Zoomify export completes
- **THEN** the document's path, modified state, history, and pixels are unchanged

#### Scenario: An unwritable output folder is refused [zf_bad_dest]

- **WHEN** the output folder does not exist or is not writable
- **THEN** the export reports failure and writes no tile or HTML file

#### Scenario: Unrelated files survive [zf_other_files]

- **WHEN** the output folder already contains a file the export does not write
- **THEN** that file is unchanged after the export
