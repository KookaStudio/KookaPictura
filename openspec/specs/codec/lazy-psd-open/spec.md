# codec/lazy-psd-open Specification

## Purpose
Opening a large PSD or PSB indexes its 8-bit raw and RLE layer channels instead of decoding them, decodes a tile's band on first read, and offers the file's merged image as the first picture, so the open costs neither the time nor the memory of every layer's pixels.

## Requirements

### Requirement: An opened PSD indexes its 8-bit raw and RLE layer channels

When a PSD or PSB is opened with a pinned source, the reader SHALL index every
8-bit layer and mask channel stored raw or RLE (its payload offset, its
compression, and each RLE row's offset) and SHALL NOT decode its pixels. Each
such channel SHALL be a tiled plane whose tiles are backed by the source. ZIP
channels, 16- and 32-bit channels, depth-1 channels, and every format without
random access SHALL decode at open as before. A document opened this way SHALL
equal the same file opened eagerly, sample for sample, and SHALL save to the
same bytes.

#### Scenario: A lazy open matches an eager one [lp_identical]

- **WHEN** each PSD of the codec round-trip corpus and a generated layered RGB
  file with raw and RLE channels are opened lazily and eagerly
- **THEN** the two documents are equal and their re-saved bytes are identical

#### Scenario: A lazy open decodes no layer pixels [lp_no_decode]

- **WHEN** a layered 8-bit RGB PSD with RLE channels is opened lazily
- **THEN** every dense tile of its layer and mask channels is away and backed,
  and no band was decoded

#### Scenario: Other channels decode at open [lp_eager_kinds]

- **WHEN** a ZIP-compressed 8-bit PSD and a 16-bit PSD are opened lazily
- **THEN** none of their layer tiles is backed and the documents equal the
  eager opens

### Requirement: A backed tile decodes only the rows it covers

Decoding a backed tile SHALL read only the bytes of the rows of its 256-row
band. A decoded band SHALL be kept in a process-wide cache of at most 32 MiB, so
reading every tile of a band decodes it once.

#### Scenario: A tile reads its band only [lp_rows]

- **WHEN** one tile of a lazily opened 1024-row RLE channel is read
- **THEN** the bytes read are those of its band's rows, and reading the other
  tiles of that band reads nothing more

### Requirement: Offsets and lengths are checked at index and at decode

Indexing SHALL refuse a channel whose RLE count table or rows do not fit in its
declared length, or whose payload ends past the file. Every decode SHALL check
its byte range against the file's size again, and SHALL reject a row whose
PackBits data overruns. A failed decode SHALL leave the tile unreadable and
SHALL NOT panic.

#### Scenario: A truncated count table is refused at open [lp_hostile_index]

- **WHEN** a PSD whose RLE channel declares fewer bytes than its row counts
  claim, or whose channel ends past the file, is opened lazily
- **THEN** the open fails with the same error variant as the eager open

#### Scenario: A source cut short after the open reads as unreadable [lp_hostile_decode]

- **WHEN** a lazily opened file is truncated in place and a backed tile past
  the new end is read
- **THEN** the read returns default samples, the tile is counted unreadable,
  and nothing panics

### Requirement: The file's merged image is offered as the first picture

A lazy open of a layered 8-bit RGB file whose merged image is raw or RLE SHALL
offer that image as a preview that decodes a rectangle on demand, as straight
planar RGBA with opaque alpha. The preview SHALL be offered only when the
bottom layer is a visible Background, so the picture is opaque, and only when
the read converted no colour profile.

#### Scenario: The preview reads the merged image [lp_preview]

- **WHEN** a layered RGB PSD with a Background layer is opened lazily and its
  preview is read at a rectangle crossing two bands
- **THEN** the colour planes equal that crop of the file's merged image decoded
  eagerly, and alpha is 255

#### Scenario: No preview without a visible Background [lp_preview_gate]

- **WHEN** a layered RGB PSD whose bottom layer is not a Background, and one
  whose Background is hidden, are opened lazily
- **THEN** no preview is offered
