## MODIFIED Requirements

### Requirement: The application decodes images to RGBA8888 at the Qt boundary

The application SHALL decode a supported raster file to packed RGBA8888 using
Qt (`QImage`/`QImageReader`/`QImage::fromData` or their cxx-qt binding) and SHALL
NOT pass a Qt type into the engine. The engine SHALL decode WebP natively
(see the requirement below), so the Qt decoder is authoritative for every format
except WebP. Qt's runtime decoders are the authoritative format allow-list for
the non-WebP formats; consulting `QImageReader::supportedImageFormats()` is
optional. The header probe is a fast-path guard only: it can pre-empt a
recognized header that exceeds the budget, but a container the probe does not
recognize SHALL still be attempted through Qt. The decode edge SHALL refuse —
returning failure without constructing engine structures — when the file is
missing or unreadable, when neither the engine nor Qt can decode it, when a
*recognized* header probe refuses it, or when the actual decoded allocation
(`decoded width * height * 4`) exceeds the budget. The decode edge SHALL copy the
decoded RGBA rows to the engine buffer in bulk — a row or the whole frame at a
time — and MUST NOT copy one byte at a time across the bridge. Only frame 0 of a
multi-frame file SHALL be imported. For v1 an undecodable file SHALL be refused,
not replaced by a placeholder.

#### Scenario: A supported file decodes to packed RGBA8888

- **WHEN** the decode edge is given a supported image file
- **THEN** it returns packed RGBA8888 bytes matching the decoded width and height

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

## ADDED Requirements

### Requirement: The engine decodes WebP natively

The codec SHALL expose `pictura_codec::decode_webp(bytes: &[u8], budget:
ImageBudget) -> Option<(Vec<u8>, u32, u32)>`. It SHALL decode a `RIFF....WEBP`
container — VP8 (lossy), VP8L (lossless), or VP8X (extended, with alpha) — to
tightly packed RGBA8888 with the decoded width and height, without Qt. It SHALL
refuse, returning `None` without panicking, when the bytes are not a WebP
container, when the header is malformed, when the declared width or height
exceeds the budget's maximum dimension, or when the declared allocation
(`width * height * 4`) exceeds the budget's maximum allocation. It SHALL check
those limits against the header before allocating the decode buffer and SHALL
expand a decode with no alpha channel to RGBA8888. For an animated container it
SHALL decode the first frame. The application's decode edge SHALL route a
container the probe names as WebP to this decoder and SHALL fall back to the Qt
decoder only when it returns `None`.

#### Scenario: A WebP file decodes without Qt

- **WHEN** `decode_webp` is given a valid VP8, VP8L, or VP8X container within budget
- **THEN** it returns packed RGBA8888 whose bytes match the decoded width and height

#### Scenario: A lossless WebP round-trips exactly

- **WHEN** a packed RGBA8888 image is encoded with `encode_webp` and decoded back with `decode_webp`
- **THEN** the decoded bytes equal the input bytes, alpha included

#### Scenario: An opaque WebP expands to RGBA

- **WHEN** `decode_webp` decodes a WebP whose header declares no alpha channel
- **THEN** every returned pixel has alpha 255

#### Scenario: A non-WebP or malformed container is refused

- **WHEN** `decode_webp` is given bytes that are not a `RIFF....WEBP` container or whose header is malformed
- **THEN** it returns `None` and does not panic

#### Scenario: An over-budget WebP is refused before allocating

- **WHEN** a WebP header declares a dimension over the budget's maximum or an allocation over the budget's maximum
- **THEN** `decode_webp` returns `None` without allocating the decode buffer
