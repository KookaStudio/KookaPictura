# bit-depth-sample-model Specification

## Purpose
A sample-typed pixel buffer that retains source-depth samples and converts between depths under the documented rules.

## Requirements

### Requirement: PixelBuffer is sample-typed with an 8-bit default

The engine's planar image buffer SHALL be generic over its sample type with `u8`
as the default, so a bare `PixelBuffer` denotes `PixelBuffer<u8>`, its `data`
field holds a refcounted `Plane<u8>`, and existing 8-bit indexing is unchanged.
The buffer SHALL support at least the `u8`, `u16`, and `f32` sample types,
keeping the planar layout `data.len() == width * height * channels`.

#### Scenario: A bare PixelBuffer is 8-bit

- **WHEN** code constructs `PixelBuffer::new(2, 2, 3)` and reads `data`
- **THEN** the value is a `Plane<u8>` of length 12 and the type is `PixelBuffer<u8>`

#### Scenario: A wider sample buffer is expressible

- **WHEN** code constructs a `PixelBuffer<u16>` of width 2, height 2, and 3 channels
- **THEN** its `data` is a `Plane<u16>` of length 12

### Requirement: Retained source samples are stored as typed samples

The codec SHALL store its retained native samples as decoded typed samples of the
document's bit depth rather than as the PSD byte image. The composite and
document-extra store `SourcePlanes` SHALL hold `u8` at the 8-bit Lab store, `u16`
at depth 16, and `f32` at depth 32, with the width and height it was decoded for.
The per-layer store `SourceChannels` SHALL hold the same typed samples with the
layer rect and channel ids it was decoded for. The read path SHALL decode the PSD
byte image into these samples and the write path SHALL re-encode them.

#### Scenario: A depth-16 read retains u16 samples

- **WHEN** a depth-16 RGB PSD is read
- **THEN** `Document.source_planes` holds `u16` samples of the composite color
  channels and document extra channels, not the big-endian byte image

#### Scenario: A depth-32 read retains f32 samples

- **WHEN** a depth-32 RGB PSD is read
- **THEN** `Document.source_planes` holds `f32` samples of the composite color
  channels and document extra channels

#### Scenario: A depth-16 layer retains u16 channel samples

- **WHEN** a depth-16 document carries a layer
- **THEN** the layer's `source_channels` store holds `u16` samples keyed by
  channel id

### Requirement: Sample conversions follow the depth rules

The model SHALL expose one conversion for narrowing a native sample to 8-bit
and one for widening an 8-bit sample to the native depth, and these SHALL be the
single implementation used by the codec. Narrowing SHALL be `sample >> 8` at
depth 16 and `clamp(trunc(sample * 256), 0, 255)` at depth 32. Widening SHALL be
`v * 257` at depth 16 and the 8-bit value scaled to `[0, 1]` at depth 32.

#### Scenario: Narrowing matches the byte rule

- **WHEN** the converters narrow a 16-bit `0`, `255`, `256`, `32768`, `65535`
- **THEN** the results are `0`, `0`, `1`, `128`, `255` and narrowing a 32-bit
  `1.5` or `-0.5` yields `255` or `0`

#### Scenario: Widening matches the byte rule

- **WHEN** the converters widen the 8-bit `0`, `1`, `255` to 16-bit
- **THEN** the results are `0`, `257`, `65535`, and widening to 32-bit yields
  `0.0`, `1.0/255.0`, `1.0`

### Requirement: The typed store round-trips the source bytes exactly

For an unedited 16/32-bit document the write path SHALL encode the retained
typed samples back to the same bytes the read path decoded (byte-identical for
big-endian `u16` and IEEE-754 big-endian `f32`), so the existing source-depth
round-trip and its `psd-tools` oracle are unchanged. A plane edited in the
8-bit working copy SHALL continue to be widened from its 8-bit bytes as before;
a plane edited in the retained native store SHALL be re-encoded at the source
depth from the store's own samples.

#### Scenario: Encode inverts decode

- **WHEN** an arbitrary native sample image is decoded to samples and re-encoded
- **THEN** the bytes are identical to the input for both depth 16 and depth 32

#### Scenario: The depth oracle is unchanged

- **WHEN** the existing depth and color-mode oracles run after this change
- **THEN** they pass with no golden change

#### Scenario: A natively edited plane is re-encoded at the source depth

- **WHEN** a depth-16 document's layer is edited in its retained native store
  and saved
- **THEN** the written layer plane holds the store's samples and not the 8-bit
  widening of the layer's 8-bit channels

### Requirement: Sample stores are copy-on-write

The retained typed sample store SHALL be copy-on-write. Cloning a sample store
SHALL share its sample allocation without copying samples. A write through a
mutable sample access SHALL fork the allocation only for the store being
written; a store that is not uniquely held MUST NOT be mutated in place. The
8-bit, 16-bit, and 32-bit variants SHALL share the same semantics, including the
8-bit Lab retained store.

#### Scenario: A cloned sample store shares its allocation

- **WHEN** a sample store of any depth is cloned
- **THEN** the clone and the original report the same sample allocation and equal bytes

#### Scenario: Writing a shared store forks only that store

- **WHEN** one clone of a shared sample store is written through its mutable samples
- **THEN** the written clone holds the new bytes, the other clone keeps the old bytes, and they no longer share an allocation

#### Scenario: A unique store writes in place

- **WHEN** a sample store with no other owner is written
- **THEN** it keeps its allocation and the write does not copy the samples

#### Scenario: Document clone shares retained samples

- **WHEN** a document with a 16-bit or 32-bit retained sample store is cloned
- **THEN** the document and its clone share the sample allocation until one of them is written
