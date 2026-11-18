# codec/streamed-raster-save Specification

## Purpose
Write the row-based Save As formats a band of rows at a time from a tiled
composite into the save's temp file, with the bytes of the whole-image writers,
and declare the formats that still need the whole image (#1320).

## Requirements

### Requirement: Row-based raster formats stream from a tiled composite

The PNG, BMP (16, 24 and 32 bits), TGA, Photoshop Raw, Cineon and TIFF writers
SHALL read the composite one band of rows at a time from a tiled composite,
and write their output into the save's temp file through a seekable sink. A
format's lengths, offsets and checksums that precede the data they describe
SHALL be patched after it is written. The TIFF writer SHALL stream its strips
(uncompressed, PackBits, LZW and ZIP; chunky and planar), its pyramid level,
its layer section and its metadata. Each writer's bytes SHALL equal the
previous whole-image writer's bytes for the same document and options.

#### Scenario: Streamed raster files keep their bytes [srs_golden]

- **WHEN** a fixed matrix of documents (RGB with and without transparency, grayscale, Indexed) is saved with each streaming format and its options (PNG compression and interlace; BMP depths, 16-bit layouts and Flip; TGA depths and RLE; Raw header and interleave; Cineon; TIFF compressions, pixel order, byte order, pyramid, transparency, layers and metadata)
- **THEN** the SHA-256 digest of each file equals the digest the previous writer produced

#### Scenario: A large streamed raster save stays bounded [srs_bounded]

- **WHEN** a 4096 × 4096 RGBA tiled composite is saved as PNG, as LZW TIFF with layers, and as 24-bit BMP
- **THEN** each writer's heap peak is below 64 MiB, excluding the tiled composite it reads

### Requirement: Whole-image raster formats are declared

GIF, JPEG, WebP, EPS, DCS, BMP at 8 bits or fewer, JPEG-compressed TIFF, and
CMYK or Lab raster output MAY hold the whole image while they encode, and the
code SHALL mark each of them as a deliberate shortcut. These writers SHALL
produce the same bytes as before.

#### Scenario: Whole-image formats keep their bytes [srs_whole_golden]

- **WHEN** the golden matrix is saved as GIF, EPS, DCS, 8-bit BMP, and CMYK and Lab TIFF and Raw
- **THEN** the SHA-256 digest of each file equals the digest the previous writer produced
