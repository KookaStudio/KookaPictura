# raster-save-encoders Specification

## Purpose
Deterministic, Qt-free Save As encoders for the raster and page formats whose
options or containers Qt's image writers cannot express: BMP, PNG, TIFF, GIF,
TGA, Photoshop Raw, Cineon, EPS and DCS.

## Requirements

### Requirement: Save encoders are deterministic and atomic

Given the same pixels, mode and accepted options, a writer SHALL produce
byte-identical output across runs. A single-file write SHALL go through a
temporary file in the destination directory and rename over the target only
after the whole encode succeeds, so a failure leaves the previous file untouched
and no partial file behind. A multi-file write SHALL write every file to a
temporary name and rename the set only after all encodes succeed; if a later
rename fails, already-renamed members SHALL be removed so the set is all or
nothing.

#### Scenario: Repeated encoding is byte-identical [raster-determinism]

- **WHEN** the same document and options are encoded twice
- **THEN** the two output byte sequences are identical

#### Scenario: A failed write leaves no partial file [raster-atomic-failure]

- **WHEN** an encode or the final rename fails
- **THEN** the target path is unchanged and no temporary file remains

#### Scenario: A multi-file set is all or nothing [raster-atomic-set]

- **WHEN** one file of a multi-file set fails during the encode phase
- **THEN** none of that set's files is left at its final name

#### Scenario: A rename failure also leaves no member [raster-atomic-set-rename]

- **WHEN** one file of a multi-file set fails during the rename phase
- **THEN** already-renamed members are removed and no member is left at its final name

### Requirement: BMP writer

The BMP writer SHALL write a Windows or OS/2 bitmap. The Windows form SHALL
support 1, 4, 8, 16, 24 and 32 bits per pixel and SHALL use a Windows bitmap
header; the OS/2 form SHALL support 1, 4, 8 and 24 bits and SHALL use an OS/2
header. The writer SHALL refuse 16- and 32-bit OS/2 output. 32-bit Windows output
SHALL carry the alpha channel. The writer SHALL encode RLE4 and RLE8 for 4- and
8-bit Windows bitmaps when RLE is chosen and SHALL reject RLE with an
incompatible depth or the OS/2 form. It SHALL write rows bottom-up by default
and top-down when Flip Row Order is chosen; Flip Row Order and the Advanced
Modes 16-bit layouts SHALL be refused with the OS/2 form because the OS/2 core
header has an unsigned height and no bit-fields. The Advanced Modes 16-bit
layout SHALL select X1R5G5B5, A1R5G5B5 or R5G6B5 on the Windows form, using the
bit-field masks the chosen layout requires. Sub-24-bit output SHALL carry a
palette: an already-Indexed document's palette, otherwise a deterministic
reduction to the depth's colour count.

#### Scenario: Each depth writes its header [bmp-depth-header]

- **WHEN** the same image is written at 1, 4, 8, 16, 24 and 32 bits
- **THEN** each file's header reports the chosen depth and the pixel data decodes at the image size

#### Scenario: RLE is written only where the format allows [bmp-rle]

- **WHEN** RLE is chosen for an 8-bit Windows bitmap
- **THEN** the file uses RLE8 and decodes identically to the uncompressed file; choosing RLE for 24-bit or OS/2 is refused

#### Scenario: Flip Row Order changes the row direction [bmp-flip]

- **WHEN** Flip Row Order is on
- **THEN** the header's height is negative (top-down) and the decoded image is not vertically flipped

#### Scenario: OS/2 writes an OS/2 header [bmp-os2]

- **WHEN** the file format is OS/2 and the depth is 1, 4, 8 or 24
- **THEN** the file begins with an OS/2 bitmap header and decodes at the image size

#### Scenario: OS/2 unsupported combinations are refused [bmp-os2-limits]

- **WHEN** OS/2 is chosen with Flip Row Order, a 16-bit Advanced Mode, or 32 bits
- **THEN** the writer refuses the request without creating a file

#### Scenario: A 16-bit layout is written [bmp-advanced-16]

- **WHEN** the Windows form's Advanced Modes layout is X1R5G5B5, A1R5G5B5 or R5G6B5
- **THEN** the 16-bit pixels decode with that channel layout and no other

#### Scenario: 32-bit keeps alpha [bmp-alpha-32]

- **WHEN** a document with a transparent pixel is written at 32 bits
- **THEN** the decoded pixel keeps its alpha

### Requirement: PNG writer

The PNG writer SHALL encode 8-bit grayscale, grayscale-with-alpha, RGB and RGBA
PNG. It SHALL apply a deterministic per-row filter and the chosen compression
level, and SHALL write Adam7 interlacing when Interlace is chosen and no
interlace otherwise. Alpha SHALL be preserved when the document has it.

#### Scenario: Alpha is preserved [png-alpha]

- **WHEN** a document with a transparent pixel is written as PNG
- **THEN** the decoded PNG pixel is transparent

#### Scenario: Interlace is declared and encoded [png-interlace-adam7]

- **WHEN** Interlace is chosen
- **THEN** the IHDR interlace method is 1 and the decoded image equals the non-interlaced output

#### Scenario: Compression changes the bytes but not the pixels [png-compression-level]

- **WHEN** the same image is written at the lowest and highest compression levels
- **THEN** the two files differ in size and decode identically

### Requirement: TIFF writer

The TIFF writer SHALL write classic TIFF with strips, the image's bits per
sample, and the photometric interpretation of the document's mode (grayscale,
RGB, CMYK, Lab or Indexed). Flat output SHALL use the chosen little- or
big-endian byte order. Layered output SHALL be big-endian so tag 37724 and the
PSD layer payload share one byte order; Discard Layers restores the chosen
order for the flat file.
It SHALL support None, PackBits (RLE), LZW, Deflate (ZIP) and JPEG compression,
writing the matching Compression tag. JPEG compression SHALL be gated to opaque
8-bpc RGB or grayscale images at most 30,000 px in either dimension; other modes
SHALL be refused. An Indexed TIFF SHALL write the ColorMap tag (320) and the
retained palette. LZW and Deflate MAY use horizontal differencing as an internal
encoding choice where the TIFF predictor applies, and SHALL declare it when
used. Pixel Order SHALL select chunky or planar strip layout. Alpha SHALL be
written as an extra sample and Save Transparency off SHALL composite the samples
over the mode's opaque white (RGB/gray 255; CMYK 0,0,0,0; Lab L=255, a=b=128).
The writer SHALL embed the document's ICC, EXIF, IPTC, XMP and Photoshop
resource blocks in their TIFF tags.

#### Scenario: Compression tags are written [tiff-compression-tags]

- **WHEN** compression is None, RLE, LZW, ZIP or JPEG
- **THEN** tag 259 is 1, 32773, 5, 8 or 7 respectively and the file decodes to the source pixels within the compression's tolerance

#### Scenario: Unsupported TIFF JPEG inputs are refused [tiff-jpeg-limits]

- **WHEN** TIFF JPEG is requested for a transparent, non-8-bpc, CMYK or oversized image
- **THEN** the writer reports failure, writes no file, and leaves the previous target untouched

#### Scenario: Pixel order is planar [tiff-planar]

- **WHEN** Pixel Order is Per Channel
- **THEN** tag 284 is 2 and each channel has its own strips

#### Scenario: Byte order is big-endian for flat output [tiff-byte-order]

- **WHEN** Byte Order is Macintosh and the TIFF has no layer section
- **THEN** the header begins `MM` and every numeric tag is big-endian; IBM PC writes `II`

#### Scenario: Layered TIFF is big-endian [tiff-layered-byte-order]

- **WHEN** a layered TIFF is written with the IBM PC byte-order option
- **THEN** the header is `MM` and tag 37724 is present, so the container and the PSD layer payload agree

#### Scenario: Predictor is declared [tiff-predictor]

- **WHEN** the writer uses horizontal differencing with LZW or Deflate compression
- **THEN** tag 317 declares predictor 2 and the file decodes correctly

#### Scenario: Alpha and flattening follow Save Transparency [tiff-alpha]

- **WHEN** a document with a transparent pixel is written with Save Transparency on and off
- **THEN** the first file carries an extra sample with the alpha and the second decodes the pixel as opaque, composited over the mode's white (RGB/gray 255, CMYK 0,0,0,0, Lab 255/128/128)

#### Scenario: Metadata blocks are embedded [tiff-metadata]

- **WHEN** the document carries ICC, EXIF, IPTC, XMP or Photoshop resources
- **THEN** the written TIFF carries them in tags 34675, 34665, 33723, 700 and 34377

#### Scenario: Document modes map to photometric interpretations [tiff-modes]

- **WHEN** grayscale, RGB, CMYK, Lab or Indexed documents are written
- **THEN** tag 262 is 0/1, 2, 5, 8 or 3 respectively; an Indexed file also carries ColorMap tag 320 and its palette decodes to the document's image

### Requirement: TIFF pyramid and layered TIFF

When Save Image Pyramid is on, the writer SHALL append reduced-resolution image
directories as SubIFDs while the first image remains the full-size image. When
the document has layers and Layer Compression is RLE or ZIP, the writer SHALL
embed the Photoshop layer-and-mask section in tag 37724 with the layer channels
encoded by the chosen compression; the flattened image SHALL remain readable by
applications that ignore the private tag. Discard Layers And Save A Copy SHALL
write a flat TIFF with no layer section.

#### Scenario: Pyramid levels are appended [tiff-pyramid-subifds]

- **WHEN** Save Image Pyramid is on
- **THEN** tag 330 lists reduced-resolution SubIFDs and the first IFD still decodes the full-size image

#### Scenario: The layer section round-trips [tiff-layered]

- **WHEN** a layered document is written with Layer Compression on
- **THEN** tag 37724 exists and its layer count and channel data match the document's layers

#### Scenario: Layer compression follows the option [tiff-layer-compression-bytes]

- **WHEN** Layer Compression is RLE and then ZIP
- **THEN** the layer channels in tag 37724 use PackBits and Deflate respectively

#### Scenario: Discard Layers writes a flat file [tiff-discard-layers]

- **WHEN** Layer Compression is Discard Layers And Save A Copy
- **THEN** the written TIFF has no layer section and decodes the flattened image

### Requirement: GIF writer

The writer SHALL produce a GIF89a single-frame image with a palette of at most
256 entries and one index per pixel, taken from an Indexed document's retained
palette and index plane. It SHALL set the image descriptor's interlace flag when
Interlaced is chosen and SHALL preserve 1-bit transparency when the Indexed
image has a transparent entry.

#### Scenario: The file is a GIF89a [gif89a-signature]

- **WHEN** an Indexed document is written as GIF
- **THEN** the file begins with `GIF89a`, carries a global colour table, and ends with the trailer byte

#### Scenario: The retained palette and indices are used [gif-indexed-plane]

- **WHEN** an Indexed document is written
- **THEN** decoding the GIF reproduces the document's palette and pixel colours without re-quantizing

#### Scenario: Interlace follows the option [gif-interlace-flag]

- **WHEN** Interlaced is chosen
- **THEN** the image descriptor interlace flag is set and the decoded rows match the non-interlaced output

#### Scenario: One-bit transparency survives [gif-transparency]

- **WHEN** the Indexed palette has a transparent entry
- **THEN** the Graphic Control Extension names that index and decoding preserves the transparent pixels

### Requirement: TGA writer

The TGA writer SHALL write an uncompressed or RLE true-color image at 16, 24 or
32 bits per pixel, with 32-bit output carrying 8 attribute bits, and SHALL decode
to the source image. The header SHALL declare the chosen image type and depth
and a consistent origin.

#### Scenario: Each depth decodes [tga-depth]

- **WHEN** the same image is written at 16, 24 and 32 bits
- **THEN** each file decodes at the image size with the chosen pixel depth and 32-bit output keeps alpha

#### Scenario: RLE changes the image type [tga-rle]

- **WHEN** Compress (RLE) is chosen
- **THEN** the header's image type is 10 and the decoded image equals the uncompressed output

#### Scenario: The header origin is consistent [tga-origin]

- **WHEN** a TGA is written
- **THEN** the header's origin bits match the row order the pixel data uses and the decoded image is not flipped

### Requirement: Photoshop Raw writer

The Photoshop Raw writer SHALL write the flattened document's channel samples
in the document's color mode, prefixed by the chosen number of zero header
bytes. Interleaved SHALL order samples pixel by pixel; Non-interleaved SHALL
write each channel as a complete plane. The writer SHALL write 8-bit samples and
SHALL NOT add a signature or metadata. Multichannel documents SHALL be refused:
the normalized working document retains no Multichannel plate store, so the
writer MUST NOT claim to write that mode's channels.

#### Scenario: The header is zero-filled [raw-header]

- **WHEN** Header is 0 and then 16
- **THEN** the second file begins with 16 zero bytes before the samples and the first begins with a sample

#### Scenario: Interleave orders the samples [raw-interleave]

- **WHEN** Interleave is Interleaved and then Non-interleaved
- **THEN** the first orders R,G,B per pixel and the second writes the three planes one after another

#### Scenario: Channel data follows the document mode [raw-mode-channels]

- **WHEN** Gray, RGB, CMYK and Lab documents are written
- **THEN** the sample count and order match the mode's color channels

#### Scenario: Multichannel is refused without plate data [raw-multichannel-refused]

- **WHEN** a Multichannel document is written as Raw
- **THEN** the writer reports failure, writes no file, and does not fall back to RGB samples

#### Scenario: Raw writes 8-bit samples [raw-eight-bit]

- **WHEN** a document is written as Raw
- **THEN** each sample is one byte and the file size is the header plus width × height × channels

### Requirement: Cineon writer

The Cineon writer SHALL write a Kodak Cineon file with the documented magic and
header, RGB only, each channel quantized to 10 bits and packed three samples per
32-bit word, and SHALL decode as an RGB image at the document size. It SHALL map
the document's 8-bit RGB samples deterministically into the 10-bit range and
SHALL NOT write alpha.

#### Scenario: The header identifies a Cineon file [cineon-header]

- **WHEN** a document is written as Cineon
- **THEN** the file begins with the Cineon magic and its header reports the document width, height and three channels

#### Scenario: Samples are packed 3 x 10 bits [cineon-packing]

- **WHEN** a Cineon file is read back
- **THEN** each 32-bit word carries three 10-bit channel values and the image decodes at the document size

#### Scenario: The 8-bit to 10-bit mapping is deterministic [cineon-conversion]

- **WHEN** the same document is written twice
- **THEN** the code values are byte-identical, and black stays at the low end and white at the high end of the 10-bit range

#### Scenario: Alpha is not represented [cineon-no-alpha]

- **WHEN** a document with a transparent pixel is written as Cineon
- **THEN** the file carries three channels only and the transparent pixel is written opaque

### Requirement: EPS writer

The EPS writer SHALL write an Encapsulated PostScript file with a DSC header, a
correct bounding box, and the document flattened to one of RGB, grayscale or
CMYK; Lab, Multichannel, Indexed, Duotone and Bitmap sources SHALL be converted
to one of those spaces before writing. Encoding SHALL be ASCII, ASCII85, Binary
or JPEG with the chosen quality.
Preview SHALL be None, a 1-bit TIFF preview or an 8-bit TIFF preview. The writer
SHALL honor Include Halftone Screen, Include Transfer Function, Transparent
Whites (bitmap images), PostScript Color Management, Include Vector Data and
Image Interpolation, and SHALL emit a clipping path when the document names one.

#### Scenario: The DSC header and bounding box are valid [eps-dsc-header]

- **WHEN** an EPS is written
- **THEN** it starts with `%!PS-Adobe-3.0 EPSF-3.0`, declares a bounding box matching the document size in points, and PostScript interpreters open it

#### Scenario: Preview choices are embedded [eps-preview]

- **WHEN** Preview is None, TIFF 1-bit or TIFF 8-bit
- **THEN** the file carries no preview, a one-bit preview or an eight-bit preview respectively, and the PostScript section is unchanged

#### Scenario: Each encoding writes the image [eps-encodings]

- **WHEN** Encoding is ASCII, ASCII85, Binary or JPEG
- **THEN** each file decodes to the source image within the encoding's tolerance, and the JPEG form uses the accepted quality

#### Scenario: The clipping path is emitted [eps-clipping-path]

- **WHEN** the document names a clipping path
- **THEN** the EPS sets that path as the clip before painting the image

#### Scenario: Vector data is included when asked [eps-vector-data]

- **WHEN** Include Vector Data is on and the document has paths
- **THEN** the EPS contains PostScript path operators for those paths as well as the raster image

#### Scenario: Mode color spaces are written [eps-color-spaces]

- **WHEN** a grayscale, RGB or CMYK document is written, or a Lab, Multichannel, Indexed, Duotone or Bitmap source is converted for EPS
- **THEN** the EPS image operator uses the matching device color space and the decoded image matches the source after conversion

### Requirement: DCS writer

The DCS writer SHALL write EPS-based CMYK separations. DCS 1.0 SHALL write a
composite master file and one plate file per process color with the `.C`, `.M`,
`.Y` and `.K` extensions. DCS 2.0 SHALL write one EPS carrying all four process
colors. Each file SHALL be a valid EPS with its own DSC header and bounding box.
The master SHALL carry the selected EPS Preview (None, 1-bit TIFF or 8-bit
TIFF). The writer SHALL NOT write named spot inks because the document model
does not carry ink names.

#### Scenario: DCS 1.0 writes four plates and a master [dcs-1-plates]

- **WHEN** a CMYK document is written as DCS 1.0
- **THEN** the master and four plate files exist, each is a valid EPS, and each plate decodes one process color

#### Scenario: DCS 2.0 writes one separations file [dcs-2-single]

- **WHEN** a CMYK document is written as DCS 2.0
- **THEN** only the master file exists and it carries the four process-color images

#### Scenario: Spot inks are not invented [dcs-process-only]

- **WHEN** the document has extra channels
- **THEN** the writer writes process CMYK only and reports that named spot inks were omitted

#### Scenario: The DCS preview follows the option [dcs-preview]

- **WHEN** Preview is None, TIFF 1-bit or TIFF 8-bit for a DCS save
- **THEN** the master carries the chosen preview and the separations are unchanged

### Requirement: Save encoders refuse unsupported input

A save encoder SHALL return failure without panicking and without writing a
partial file when the pixel buffer is shorter than the declared size, when a
dimension is zero, or when the document mode has no representation in the chosen
format (for example JPEG compression of a CMYK TIFF if unsupported). It SHALL
bound its intermediate allocations to the image size and the chosen options.

#### Scenario: An invalid buffer is refused [raster-refuses-invalid]

- **WHEN** a writer is given zero dimensions or a buffer shorter than width × height × channels
- **THEN** it reports failure and creates no file

#### Scenario: An unrepresentable mode is refused [raster-refuses-mode]

- **WHEN** a writer is asked to encode a mode the chosen format cannot represent
- **THEN** it reports failure, writes nothing, and leaves the previous file untouched
