# raster-export delta

## MODIFIED Requirements

### Requirement: The engine encodes WebP natively

The codec SHALL expose `pictura_codec::encode_webp(rgba: &[u8], width: u32,
height: u32) -> Option<Vec<u8>>` for lossless output and
`pictura_codec::encode_webp_lossy(rgba: &[u8], width: u32, height: u32,
quality: u8) -> Option<Vec<u8>>` for lossy output. Both SHALL encode tightly
packed RGBA8888 as a WebP container without Qt and SHALL preserve alpha. The
lossy encoder SHALL accept quality 0 through 100 and SHALL use the extended
`VP8X` container with an `ALPH` chunk when the image has transparency. Both
SHALL return `None` without panicking when `width` or `height` is zero or when
`rgba` is shorter than `width * height * 4`.

The application SHALL route a `WEBP` Save As target to the lossless encoder and
an Export As target to the encoder selected by its Lossless control, rescaling
through the boundary first when a percent scale is given. It SHALL write the
result atomically (temp file plus rename for Save As, protected output for
Export As), so a failed encode leaves no partial file and does not change the
document's path, modified state, or history.

#### Scenario: WebP export preserves alpha

- **WHEN** a document whose composite has a transparent pixel is exported to a `.webp` in either mode
- **THEN** the written WebP has that pixel transparent

#### Scenario: WebP save round-trips through the engine

- **WHEN** a document is saved to a `.webp` path and reopened
- **THEN** the reopened pixels match the flattened composite, because the write stays lossless and both the write and the read use the engine's WebP codec rather than Qt

#### Scenario: Lossy WebP is smaller than lossless [webp_lossy_smaller]

- **WHEN** the same composite is exported as lossy WebP at quality 90 and as lossless WebP
- **THEN** the lossy file is smaller and both decode to the target size

#### Scenario: Lossy quality changes the file [webp_lossy_quality]

- **WHEN** the same composite is exported as lossy WebP at a low quality and at a high quality
- **THEN** the low-quality file is smaller

#### Scenario: Lossy WebP preserves transparency [webp_lossy_alpha]

- **WHEN** an image with transparent pixels is encoded with `encode_webp_lossy` and decoded
- **THEN** the transparent pixels stay transparent

#### Scenario: A malformed buffer is refused

- **WHEN** either encoder is given a zero dimension or a buffer shorter than `width * height * 4`
- **THEN** it returns `None` and does not panic

## REMOVED Requirements

### Requirement: Export As format list and controls

## ADDED Requirements

### Requirement: Export As format list and WebP controls

The Export As Format list SHALL offer PNG, JPG, PNG-8, GIF, SVG, and WebP, and
the written suffix SHALL be `png`, `jpg`, `png`, `gif`, `svg`, and `webp`
respectively. PNG-8 SHALL be quantized to an indexed palette by the engine web
palette and written by Qt; GIF SHALL be encoded as GIF89a by the engine web
encoder; SVG SHALL be written by the engine SVG writer (`interop/svg-export`);
WebP SHALL be written by the engine's native WebP encoder. A Transparency
control SHALL be enabled for PNG, PNG-8, GIF, and WebP. A Quality control SHALL
be enabled for JPG and for WebP when its Lossless control is off. A Lossless
control SHALL be offered for WebP only and SHALL default to off, so WebP
defaults to lossy at quality 90. A control that does not apply to the chosen
format SHALL be disabled.

#### Scenario: PNG-8 writes an indexed image

- **WHEN** the user exports a colourful document as PNG-8
- **THEN** the written PNG decodes to the target size and holds no more than 256 distinct colours

#### Scenario: GIF writes a GIF89a file

- **WHEN** the user exports the document as GIF
- **THEN** the written file begins with the GIF89a signature and decodes to the target size

#### Scenario: Quality applies to JPG and lossy WebP [webp_quality_controls]

- **WHEN** the Format is JPG, or WebP with Lossless unchecked
- **THEN** the Quality control is enabled, and for WebP the Transparency control is enabled too

#### Scenario: WebP defaults to lossy quality 90 [webp_lossy_default]

- **WHEN** the Format is changed to WebP and the dialog is inspected
- **THEN** Lossless is unchecked, Quality reads 90 and is enabled, and the encoded file carries a VP8 stream

#### Scenario: WebP Lossless writes a VP8L stream [webp_lossless_control]

- **WHEN** the Format is WebP and Lossless is checked
- **THEN** the Quality control is disabled and the encoded file carries a VP8L stream

#### Scenario: The full format list is offered

- **WHEN** the Format list is opened
- **THEN** it contains PNG, JPG, PNG-8, GIF, SVG, and WebP
