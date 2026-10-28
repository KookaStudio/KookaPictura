# imaging/digimarc-watermark Specification

## Purpose
TBD - created by archiving change digimarc-watermark. Update Purpose after archive.

## Requirements

### Requirement: An in-tree approximate watermark payload

The `pictura_filters::digimarc` module SHALL carry a watermark payload of a
watermark type (`Image ID` or `Transaction ID`), a creator ID (`0..=8_388_607`),
and an ID (`0..=16_777_215`) as a 66-bit codeword — an 8-bit magic byte, a 2-bit
durability field, a 1-bit type, the creator ID, the ID, and an 8-bit CRC — tiled
across the image's colour planes with a deterministic ±1 chip. The scheme SHALL
be the project's own and MUST NOT be presented as, or claim compatibility with,
Digimarc Barcode.

Embedding a payload and then reading the same pixels SHALL return the same
payload. A codeword whose magic or CRC does not validate SHALL be rejected, and
an image that carries no carrier SHALL read as no watermark.

#### Scenario: Embed then read round-trips the payload [dw_roundtrip]

- **WHEN** a payload is embedded at each durability and the resulting pixels are read
- **THEN** the reader returns the same type, creator ID, ID, and durability

#### Scenario: A corrupted carrier fails the checksum [dw_crc_reject]

- **WHEN** an embedded image is perturbed so the codeword no longer validates
- **THEN** the reader returns no watermark rather than a wrong payload

#### Scenario: A plain image carries no watermark [dw_absent]

- **WHEN** an image with no embedded carrier is read
- **THEN** the reader returns no watermark

### Requirement: A deterministic spread-spectrum carrier

The carrier SHALL be deterministic: embedding the same payload at the same
durability on the same pixels SHALL produce byte-identical pixels, and the
carrier SHALL be zero-mean over each cell so it preserves the local average. The
carrier SHALL only touch the colour planes; a buffer's transparency plane MUST
stay bit-identical. A higher durability SHALL produce a larger detection margin
than a lower one on the same image.

#### Scenario: The carrier is deterministic and channel-neutral [dw_carrier]

- **WHEN** the same payload and durability are embedded twice, and a four-channel
  buffer is embedded
- **THEN** the two results are byte-identical, every changed colour sample moves by
  no more than the durability's amplitude, and the alpha plane is unchanged

#### Scenario: Higher durability yields a stronger detection margin [dw_durability]

- **WHEN** the same payload is embedded at Low and at High durability
- **THEN** the margin reported for High is greater than the margin reported for Low

### Requirement: Embed Watermark is a Filter-menu command with a CS6-shaped dialog

`Filter ▸ Digimarc ▸ Embed Watermark` SHALL be an enabled command whose dialog
exposes Watermark Type (Image ID / Transaction ID), Creator ID, Image ID, and
Watermark Durability (Low / Medium / High). OK SHALL apply the filter to the
active pixel layer and record exactly one history state labelled `Filter: Embed
Watermark`; Cancel SHALL leave the document unchanged. The command SHALL use the
shared filter dialog's live preview.

#### Scenario: The command is enabled and opens its dialog [dw_embed_menu]

- **WHEN** a document with a pixel layer is open
- **THEN** the Embed Watermark action is enabled and dispatching it opens the
  Embed Watermark dialog

#### Scenario: The dialog exposes the CS6 controls [dw_embed_controls]

- **WHEN** the Embed Watermark dialog is shown
- **THEN** it has a Watermark Type choice, Creator ID and Image ID number fields,
  and a Watermark Durability radio group with Low, Medium, and High

#### Scenario: OK commits one history state and Cancel changes nothing [dw_embed_commit]

- **WHEN** the dialog is accepted
- **THEN** the layer's pixels change, one `Filter: Embed Watermark` history state
  is recorded, and the payload reads back; when it is rejected instead, the
  pixels are unchanged and no state is recorded

### Requirement: Read Watermark reports the payload of a watermarked layer

`Filter ▸ Digimarc ▸ Read Watermark` SHALL be an enabled command that reads the
active pixel layer and reports the decoded payload. When a watermark is present
the report SHALL name the type, creator ID, and ID; when it is absent the report
SHALL say so.

#### Scenario: The command is enabled for a pixel layer [dw_read_menu]

- **WHEN** a document with a pixel layer is open
- **THEN** the Read Watermark action is enabled

#### Scenario: A watermarked layer reports its payload [dw_read_found]

- **WHEN** a payload is embedded on the active layer and Read Watermark runs
- **THEN** the report names the embedded type, creator ID, and ID

#### Scenario: An unwatermarked layer reports no watermark [dw_read_none]

- **WHEN** Read Watermark runs on a layer with no carrier
- **THEN** the report states that no watermark was found
