# codec/pict-format Specification

## Purpose
Read and write Apple QuickDraw PICT files as a bitmap interchange: the
bitmap/pixmap records Photoshop CS6 opens, the QuickTime JPEG record, safe
refusal of vector-only or unsupported pictures, and a deterministic extended
version 2 writer with the CS6 resolution and compression choices.

## Requirements

### Requirement: PICT reader decodes bitmap and pixmap records

The codec SHALL expose a Qt-free PICT decoder that reads version 1 and extended
version 2 pictures, with or without the 512-byte file header. It SHALL decode
the bitmap records `BitsRect`, `BitsRgn`, `PackBitsRect`, `PackBitsRgn`,
`DirectBitsRect` and `DirectBitsRgn`: 1-bit bitmaps, 1/2/4/8-bit indexed
pixmaps with a colour table, 16-bit RGB555 direct pixels, and 32-bit direct
pixels with or without alpha. It SHALL decode unpacked rows, the drop-pad-byte
form, PackBits over pixel bytes, PackBits over component planes, and the
per-scanline PackBits form. It SHALL composite several image records in order
and SHALL decode a `CompressedQuickTime` (`$8200`) JPEG record.

#### Scenario: A 32-bit direct record decodes [pict-read-direct32]

- **WHEN** a version 2 picture holds a `DirectBitsRect` with 32-bit direct pixels
- **THEN** the decoded pixels and dimensions match the encoded image

#### Scenario: A 16-bit direct record decodes [pict-read-direct16]

- **WHEN** a version 2 picture holds a 16-bit RGB555 `DirectBitsRect`
- **THEN** each pixel's five-bit channels are expanded to the expected eight-bit values

#### Scenario: An indexed record decodes through its colour table [pict-read-indexed]

- **WHEN** a `PackBitsRect` holds 8-bit indices and a colour table
- **THEN** each output pixel is the colour-table entry for its index

#### Scenario: A one-bit bitmap decodes [pict-read-bitmap]

- **WHEN** a `BitsRect` or `PackBitsRect` holds a one-bit bitmap
- **THEN** zero maps to the background colour and one to the foreground colour

#### Scenario: A version 1 picture decodes [pict-read-v1]

- **WHEN** a version 1 picture holds a one-bit bitmap
- **THEN** its one-byte opcodes are read and the bitmap decodes

#### Scenario: Several records composite [pict-read-multiple]

- **WHEN** a picture holds more than one image record at different rectangles
- **THEN** each record replaces the canvas at its destination rectangle in file order

#### Scenario: A QuickTime JPEG record decodes [pict-read-jpeg]

- **WHEN** a picture holds a `CompressedQuickTime` record with a JPEG stream
- **THEN** the JPEG's pixels are decoded and placed at the record's rectangle

#### Scenario: Alpha survives a 32-bit record [pict-read-alpha]

- **WHEN** a 32-bit direct record has four components
- **THEN** the decoded alpha matches the encoded alpha

#### Scenario: PackBits rows decode [pict-read-packbits]

- **WHEN** a record uses PackBits with literal and repeated runs
- **THEN** the runs expand to the exact row bytes

### Requirement: PICT reader refuses unsupported or malformed input safely

The decoder SHALL be bounds-checked and SHALL NOT panic on attacker-supplied
bytes. It SHALL charge the declared canvas and each image record to
`ImageBudget` before allocating. It SHALL return a typed error for a truncated or
inconsistent header or record, a picture with no bitmap record, an unsupported
pixel format or packing type, an uncompressed QuickTime record (`$8201`), a
malformed embedded JPEG, more than 4096 image records, or a size over budget.

#### Scenario: A truncated header is refused [pict-refuse-malformed]

- **WHEN** the bytes end inside the header, a record or its pixel data
- **THEN** the decoder returns a malformed error without panicking

#### Scenario: A vector-only picture is refused [pict-refuse-vector]

- **WHEN** the picture holds QuickDraw drawing records but no bitmap record
- **THEN** the decoder returns an unsupported error naming the missing bitmap

#### Scenario: An unsupported record is refused [pict-refuse-unsupported]

- **WHEN** a record uses an unsupported pixel format, packing type, pattern or
  uncompressed QuickTime form
- **THEN** the decoder returns an unsupported error naming the feature

#### Scenario: An oversized picture is refused [pict-refuse-oversized]

- **WHEN** the declared canvas or a record exceeds the budget's dimension or allocation limit
- **THEN** the decoder refuses before allocating the canvas

#### Scenario: A malformed JPEG is refused [pict-refuse-jpeg]

- **WHEN** a `CompressedQuickTime` record's payload is not a decodable JPEG
- **THEN** the decoder returns a JPEG error without panicking

### Requirement: PICT writer encodes extended version 2

The codec SHALL expose a PICT writer that emits extended version 2 with a
512-byte application header, the resolution header, a clip region and one
`DirectBitsRect`. Resolution 32 SHALL write 32-bit direct pixels with component
PackBits and SHALL carry alpha when the composite has a transparent pixel.
Resolution 16 SHALL write unpacked 16-bit RGB555 and SHALL flatten alpha over
white. The writer SHALL be deterministic and SHALL go through the existing
atomic save path.

#### Scenario: 32-bit output round-trips [pict-write-32]

- **WHEN** an opaque image is written at 32 bits and the result is decoded
- **THEN** the pixels and dimensions match the source

#### Scenario: 16-bit output round-trips [pict-write-16]

- **WHEN** an image is written at 16 bits and the result is decoded
- **THEN** the pixels match the source within the five-bit channel quantisation

#### Scenario: Alpha is written at 32 bits [pict-write-alpha]

- **WHEN** an image with a transparent pixel is written at 32 bits and decoded
- **THEN** the decoded pixel is transparent

#### Scenario: Repeated encoding is byte-identical [pict-write-deterministic]

- **WHEN** the same image and options are written twice
- **THEN** the two byte sequences are identical

### Requirement: PICT writer honors the resolution and compression choices

The writer SHALL write no compression as the PackBits form and SHALL write JPEG
Low, Medium, High and Maximum as a `$8200` QuickTime JPEG at quality 3, 6, 9
and 12 on the CS6 0-12 scale. It SHALL flatten alpha over white for JPEG. It
SHALL refuse an image wider than 8191 pixels at 32 bits, wider than 16383 at 16
bits, or larger than 32767 pixels in either dimension.

#### Scenario: JPEG output round-trips [pict-write-jpeg]

- **WHEN** an image is written with each JPEG level and the result is decoded
- **THEN** the `$8200` record is present, the image decodes, and a lower level is
  no larger than a higher level

#### Scenario: Oversized output is refused [pict-write-limits]

- **WHEN** the requested width or height exceeds the format's field or the
  chosen resolution's row limit
- **THEN** the writer refuses without producing a file
