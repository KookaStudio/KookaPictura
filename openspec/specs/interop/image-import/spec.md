# image-import Specification

## Purpose
A Qt-free header probe and engine document/layer construction from packed RGBA8888 for importing raster images.

## Requirements

### Requirement: Qt-free image header probe and allocation budget

The codec SHALL expose `pictura_codec::probe_image(bytes: &[u8], budget:
ImageBudget) -> Result<ImageProbe, ImportError>`. It SHALL parse the declared
format, width, height, and bit depth from
the container header of common raster images (PNG, JPEG, GIF, BMP, TIFF, and
WebP). The probe SHALL be pure and Qt-free: it MUST NOT decode pixel data, MUST
NOT depend on a decoder, and MUST NOT allocate from the declared dimensions.
`ImageBudget` SHALL carry a maximum dimension and a maximum estimated allocation
in bytes. The probe SHALL return `Err(ImportError)` — without allocating from the
declared size — when the declared width or height exceeds the maximum dimension,
when the estimated RGBA allocation (`width * height * 4`) exceeds the maximum
allocation, when the container is unrecognized, or when the header is truncated.
`ImportError` SHALL carry the source description, the declared dimensions, the
bytes read, and which limit was exceeded. Because the header is
attacker-controlled, the probe result is advisory: the application SHALL enforce
the same budget against the actual decoded allocation.

#### Scenario: A recognized header yields declared metadata

- **WHEN** `probe_image` is called on a valid PNG, JPEG, GIF, BMP, TIFF, or WebP header
- **THEN** it returns `Ok(ImageProbe)` with that format and the header's declared width, height, and bit depth

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

### Requirement: Engine builds an RGB/8-bit document from packed RGBA8888

The engine SHALL provide `pictura_core::Document::from_rgba(name: &str, width:
u32, height: u32, rgba: &[u8]) -> Document`. It SHALL return a document whose
`width` and `height` are the
arguments, whose mode is RGB, and whose depth is 8-bit. Its `composite` SHALL be
seeded from `rgba` in the packed RGBA8888 layout the app already uses (one pixel
per four bytes, byte order R, G, B, A), and it SHALL contain exactly one pixel
layer named `name` with `rect = (0, 0, width, height)` and planar channels `0`,
`1`, `2`, and `-1` populated from `rgba`. The document SHALL carry no smart
object, no adjustment, and no other layer. A buffer shorter than
`width * height * 4` SHALL NOT panic.

#### Scenario: Size, mode, and depth are established

- **WHEN** `from_rgba` is called with width `w`, height `h`, and a packed buffer
- **THEN** the document reports `width = w`, `height = h`, RGB mode, and 8-bit depth

#### Scenario: Exactly one pixel layer holds the pixels

- **WHEN** `from_rgba` returns
- **THEN** the document has exactly one layer, named `name`, with `rect = (0, 0, w, h)`, no smart object, and planar color and alpha channels populated from `rgba`

#### Scenario: RGBA fidelity and alpha are preserved

- **WHEN** a pixel's packed RGBA value is read back from the layer's channels
- **THEN** the red, green, blue, and alpha values equal the input, alpha included

#### Scenario: A malformed buffer does not panic

- **WHEN** `from_rgba` is called with a buffer shorter than `width * height * 4`
- **THEN** it returns a document without panicking

### Requirement: Engine appends a raster layer from packed RGBA8888

The engine SHALL provide `pictura_render::add_raster_layer_from_rgba(doc: &mut
Document, name: &str, width: u32, height: u32, rgba: &[u8]) -> String`. It SHALL
append a new raster pixel
layer at the top of `doc.layers`, named `name`, with `rect = (0, 0, width,
height)`, `visible` true, `BlendMode::Normal`, `opacity` 255, and planar channels
`0`, `1`, `2`, and `-1` populated from the packed RGBA8888 `rgba`. It SHALL NOT
change any existing layer, the document size, or the document's other content,
and SHALL return the new layer's path using the same path-string convention as
the other `layer_ops` (`"0"`, `"2/1"`, …). A zero `width` or `height` SHALL
return an empty string without mutating the document.

#### Scenario: The layer is appended topmost and native-size

- **WHEN** `add_raster_layer_from_rgba` is called with size `w × h`
- **THEN** a new layer with `rect = (0, 0, w, h)` is the topmost entry of `doc.layers` and the returned path resolves to it

#### Scenario: The pixels are planar and faithful

- **WHEN** the appended layer's channels are sampled at a coordinate
- **THEN** the color and alpha equal the packed RGBA8888 input

#### Scenario: A zero dimension is refused

- **WHEN** `width` or `height` is zero
- **THEN** the function returns an empty string and the document is unchanged

#### Scenario: The appended layer is convertible to a smart object

- **WHEN** the returned path is passed to `convert_to_smart_object`
- **THEN** it returns true and the layer carries an embedded smart object

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

### Requirement: Open imports supported raster images as a new document

The application SHALL expose `PictureView::open_image(path: &QString) -> bool`.
On success it SHALL build a document through
`pictura_core::Document::from_rgba` using the file's base name, replace the view's
document and displayed image with it, reset the edit state, capture exactly one
history state labelled `"Open"`, leave the view's path empty so the document is
untitled, and mark the document unmodified. It SHALL record a display name on the
view so the tab title reads `<file name> (Mode/Bits)`, where the file name
includes the file extension, rather than a generated `Untitled-N` name. It SHALL
select the imported layer as the view's active layer. A fully opaque decoded image
SHALL become exactly one layer named `Background` with `background = true`, a lock
state of `TRANSPARENCY | POSITION`, and the redundant opaque transparency channel
dropped; an image with any non-opaque pixel SHALL remain a regular alpha layer
named from the file stem. It SHALL return `false` without mutating the view on any
refusal. `File > Open` SHALL offer an `Images (…)` filter beside the existing
`Photoshop files (*.psd *.psb)` filter. A PSD/PSB file SHALL continue through the
native `read_psd` path and MUST NOT be routed through Qt.

#### Scenario: A supported image opens as an untitled document

- **WHEN** `open_image` is called on a supported raster file
- **THEN** it returns true, the view holds a document sized to the image with the imported pixels, exactly one `"Open"` history state is added, and the view's path is empty

#### Scenario: The imported tab keeps a display name [lim_display_name]

- **WHEN** `open_image` succeeds for a file such as `image01.jpg`
- **THEN** the view's display name is the file name including its extension and
  its tab title reads `image01.jpg (Mode/Bits)` rather than `Untitled-N`

#### Scenario: An opaque image becomes the locked Background [lim_opaque_background]

- **WHEN** `open_image` decodes a file every pixel of which is fully opaque
- **THEN** the document has exactly one layer named `Background` with the
  background flag set and a lock state of transparency and position only, and no
  separate transparency channel is required

#### Scenario: The imported layer becomes active [lim_activate]

- **WHEN** `open_image` succeeds
- **THEN** the imported layer is the view's active layer and is the single layer a
  subsequent tool edit targets

#### Scenario: A non-opaque image stays a regular alpha layer [lim_transparent_layer]

- **WHEN** `open_image` decodes a file with at least one non-opaque pixel
- **THEN** the document has one regular alpha layer named from the file stem and
  is not a locked Background

#### Scenario: A refusal mutates nothing

- **WHEN** `open_image` is called with a missing, undecodable, or over-budget file
- **THEN** it returns false, the view's document is unchanged, and no history state is added

#### Scenario: The dialog accepts images and keeps PSD native

- **WHEN** `File > Open` is invoked
- **THEN** the dialog offers an `Images (…)` filter and a Photoshop filter, and a chosen `*.psd`/`*.psb` file is opened through `read_psd` rather than Qt

### Requirement: Place imports supported raster images as an embedded smart object

The application SHALL expose `PictureView::place_image(path: &QString) -> QString`.
On success it SHALL append a raster layer through
`pictura_render::add_raster_layer_from_rgba`, convert that layer with the existing
`pictura_render::convert_to_smart_object`, recomposite, record exactly one history
state labelled `"Place"`, and return the new layer's path. The placed layer SHALL
keep the decoded pixels as a raster proxy and render from that proxy; its embedded
smart-object payload SHALL be the authored PSD proxy, not the original image
bytes. Any refusal SHALL return an empty string and record no history state.
`File > Place…` SHALL offer an `Images (…)` filter. A PSD/PSB file SHALL continue
through `place_smart_object` and MUST NOT be routed through Qt.

#### Scenario: A supported image is placed as a smart object

- **WHEN** `place_image` is called on a supported raster file with a document open
- **THEN** it returns the new layer's path, a new topmost layer carrying an embedded smart object exists, and exactly one `"Place"` history state is added

#### Scenario: The placed layer renders from its raster proxy

- **WHEN** a solid-color image is placed into a document
- **THEN** the composite shows the image's pixels within the placed layer's rect

#### Scenario: A refusal records nothing

- **WHEN** `place_image` is called with no open document or a missing, undecodable, or over-budget file
- **THEN** it returns an empty string, the document is unchanged, and no history state is added

#### Scenario: PSD stays native for Place

- **WHEN** `File > Place…` is given a `*.psd`/`*.psb` file
- **THEN** it routes through `place_smart_object` and not through the image decode edge

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

### Requirement: Raster imports apply the embedded ICC profile

The application's raster decode edge SHALL extract the embedded ICC profile from a JPEG (`APP2` `ICC_PROFILE`), TIFF (tag `34675`), or PNG (`iCCP`) file before constructing the document, and SHALL classify it as sRGB or non-sRGB by its `Description` tag. A non-sRGB profile SHALL be applied under the incoming-profile policy for that colour model:

- `Preserve` SHALL keep the decoded colour bytes, set `Document.document_icc` to the embedded profile bytes, and keep the file's `source_icc` empty.
- `Convert` SHALL convert the decoded colour bytes from the embedded profile to the sRGB working space with the configured conversion options, set `source_icc` to the embedded bytes, and leave `document_icc` empty.
- `Off` SHALL keep the decoded colour bytes and leave both profile fields empty.

An untagged raster, an sRGB-tagged raster, or a raster whose profile cannot be decoded SHALL be handled as an untagged sRGB import with no profile fields set. A placed raster image SHALL be converted to the host document's sRGB working space before it is added as a layer, because the engine's working pixels are sRGB. The profile decision is the same for `Open` and for `Place`; the warning flow is specified in `color/profile-warnings`.

#### Scenario: A JPEG with an Adobe RGB profile is preserved

- **WHEN** a JPEG with an Adobe RGB profile is opened with the Preserve policy
- **THEN** the document's pixels equal the decoded RGB values, `document_icc` holds the Adobe RGB bytes, and the view reports a preserved embedded profile

#### Scenario: A TIFF with an Adobe RGB profile is converted

- **WHEN** a TIFF with an Adobe RGB profile is opened with the Convert policy
- **THEN** the document's pixels are the lcms2 conversion to sRGB, `source_icc` holds the embedded bytes, and `document_icc` is empty

#### Scenario: A PNG with a non-sRGB profile is handled by policy

- **WHEN** a PNG with a non-sRGB iCCP profile is opened with the Off policy
- **THEN** the document's pixels equal the decoded RGB values and both profile fields are empty

#### Scenario: An untagged raster imports as sRGB

- **WHEN** a JPEG, TIFF, or PNG with no embedded profile is opened
- **THEN** the document imports as sRGB with no profile fields set and no warning

#### Scenario: A malformed profile is ignored

- **WHEN** a raster file carries an ICC profile that does not parse
- **THEN** the import succeeds with the decoded pixels and no profile fields set

#### Scenario: A placed raster is converted to the host working space

- **WHEN** a non-sRGB raster is placed into an sRGB document
- **THEN** the placed layer's pixels are the lcms2 conversion to sRGB and the host document's history gains one Place state
