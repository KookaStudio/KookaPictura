# Spec Delta

## MODIFIED Requirements

### Requirement: Qt-free image header probe and allocation budget

The codec SHALL expose `pictura_codec::probe_image(bytes: &[u8], budget:
ImageBudget) -> Result<ImageProbe, ImportError>`. It SHALL parse the declared
format, width, height, and bit depth from
the container header of common raster images (PNG, JPEG, GIF, BMP, TIFF, WebP,
and PICT). The probe SHALL be pure and Qt-free: it MUST NOT decode pixel data,
MUST NOT depend on a decoder, and MUST NOT allocate from the declared
dimensions. `ImageBudget` SHALL carry a maximum dimension and a maximum
estimated allocation in bytes. The probe SHALL return `Err(ImportError)` —
without allocating from the declared size — when the declared width or height
exceeds the maximum dimension, when the estimated RGBA allocation
(`width * height * 4`) exceeds the maximum allocation, when the container is
unrecognized, or when the header is truncated. `ImportError` SHALL carry the
source description, the declared dimensions, the bytes read, and which limit was
exceeded. Because the header is attacker-controlled, the probe result is
advisory: the application SHALL enforce the same budget against the actual
decoded allocation.

#### Scenario: A recognized header yields declared metadata

- **WHEN** `probe_image` is called on a valid PNG, JPEG, GIF, BMP, TIFF, WebP, or PICT header
- **THEN** it returns `Ok(ImageProbe)` with that format and the header's declared width, height, and bit depth

#### Scenario: A PICT header is recognized [image-import-pict]

- **WHEN** `probe_image` is called on a PICT file with the version opcode at its file-header or headerless offset
- **THEN** it returns `Ok(ImageProbe)` with `ImageFormat::Pict` and the picture frame's width and height

#### Scenario: A declared dimension over the maximum is refused

- **WHEN** a header declares a width or height greater than the budget's maximum dimension
- **THEN** it returns `Err(ImportError)` naming the dimension limit and the declared dimensions, without allocating from the declared size

#### Scenario: An estimated allocation over budget is refused

- **WHEN** a header's `width * height * 4` exceeds the budget's maximum allocation
- **THEN** it returns `Err(ImportError)` naming the allocation limit

#### Scenario: An unrecognized container is reported as unknown

- **WHEN** `probe_image` is called on bytes whose header matches no recognized container
- **THEN** it returns `Err(ImportError::UnknownContainer)` and does not panic

#### Scenario: A truncated header is refused

- **WHEN** `probe_image` is called on a recognized container truncated before its dimensions are readable
- **THEN** it returns `Err(ImportError)` and does not panic

### Requirement: The application decodes images to RGBA8888 at the Qt boundary

The application SHALL decode a supported raster file to packed RGBA8888 using
Qt (`QImage`/`QImageReader`/`QImage::fromData` or their cxx-qt binding) and SHALL
NOT pass a Qt type into the engine. The engine SHALL decode WebP and PICT
natively (see the WebP and PICT requirements), so the Qt decoder is
authoritative for every format except WebP and PICT. Qt's runtime decoders are
the authoritative format allow-list for the non-WebP, non-PICT formats;
consulting `QImageReader::supportedImageFormats()` is optional. The header probe
is a fast-path guard only: it can pre-empt a recognized header that exceeds the
budget, but a container the probe does not recognize SHALL still be attempted
through Qt. The decode edge SHALL refuse — returning failure without
constructing engine structures — when the file is missing or unreadable, when
neither the engine nor Qt can decode it, when a *recognized* header probe
refuses it, or when the actual decoded allocation (`decoded width * height * 4`)
exceeds the budget. The decode edge SHALL copy the decoded RGBA rows to the
engine buffer in bulk — a row or the whole frame at a time — and MUST NOT copy
one byte at a time across the bridge. Only frame 0 of a multi-frame file SHALL
be imported. For v1 an undecodable file SHALL be refused, not replaced by a
placeholder.

#### Scenario: A supported file decodes to packed RGBA8888

- **WHEN** the decode edge is given a supported image file
- **THEN** it returns packed RGBA8888 bytes matching the decoded width and height

#### Scenario: A PICT file decodes without Qt [image-import-pict-decode]

- **WHEN** the decode edge is given a PICT file whose probe names `ImageFormat::Pict`
- **THEN** the engine PICT decoder supplies the packed RGBA8888 pixels and Qt is not asked to decode it

#### Scenario: A Qt-decodable container unknown to the probe is imported

- **WHEN** the decode edge is given a valid image whose container the probe does not recognize but Qt can decode
- **THEN** Qt decodes it, the actual decoded allocation is capped, and the import succeeds

#### Scenario: An undecodable file is refused

- **WHEN** the decode edge is given a file neither the engine nor Qt can read
- **THEN** it returns failure and no document or layer is constructed

#### Scenario: The actual decoded allocation is capped

- **WHEN** a file's declared header is within budget but its decoded allocation exceeds the budget
- **THEN** the decode edge refuses before any engine structure is constructed

#### Scenario: The decoded rows are copied in bulk

- **WHEN** a decoded image is handed to the engine
- **THEN** its rows are copied with bulk copies and the import's cost does not grow with a per-byte copy loop

#### Scenario: Only the first frame is imported

- **WHEN** a multi-frame GIF, APNG, or WebP is decoded
- **THEN** only frame 0 is imported
