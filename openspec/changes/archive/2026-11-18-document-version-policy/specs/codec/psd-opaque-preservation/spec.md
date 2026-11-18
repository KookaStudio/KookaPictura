# PSD opaque preservation delta

## MODIFIED Requirements

### Requirement: Opaque PSD blocks survive an open-save round trip

Reading a PSD SHALL retain, byte-for-byte, every section and block the engine
does not interpret, and a default raw codec write SHALL re-emit those bytes so that an
open→save round trip loses nothing. The preserved data SHALL include the
color-mode-data section, the image-resource section (except that a document
normalized from a non-sRGB ICC profile drops only the embedded profile resource,
see `psd-icc-conversion`), the global layer mask, the trailing global
additional-layer information, each layer's original blend key when it is not a
recognized mode, each layer's blending-ranges bytes, each layer's unknown
additional-layer-info tagged blocks, each layer's unmodeled channels (for
example the `-3` real user mask), and each layer mask's trailing parameter bytes.

#### Scenario: Image resources and color-mode data round-trip

- **WHEN** a document read from a file carries a non-empty image-resource or color-mode-data section, was not ICC-normalized, and is written back using the default raw codec writer
- **THEN** reading the output yields the same resource and color-mode-data bytes

#### Scenario: An ICC-normalized document preserves every other resource

- **WHEN** a document normalized from a non-sRGB ICC profile is written using the default raw codec writer and re-read
- **THEN** every image resource except the profile resource is byte-identical and the profile resource is absent

#### Scenario: Unknown tagged blocks round-trip
- **WHEN** a layer carries additional-layer-info blocks the engine does not model
- **THEN** those blocks are present with identical keys and payloads after read→write→read

#### Scenario: An unmodeled channel round-trips
- **WHEN** a layer carries a `-3` real-user-mask channel
- **THEN** the channel's bytes are present after read→write→read

#### Scenario: An unknown blend key round-trips
- **WHEN** a layer's blend key is not one of the recognized modes
- **THEN** the original 4-byte key is written back after read→write→read

#### Scenario: Blending ranges and mask parameters round-trip
- **WHEN** a layer carries non-empty blending ranges or a mask block longer than the fixed fields
- **THEN** those bytes are present after read→write→read

## ADDED Requirements

### Requirement: Version-aware writes preserve unrelated native data

In addition to the existing codec preservation guarantees, version-aware PSD and PSB writes SHALL preserve unrelated fully framed resources byte-for-byte, subject to existing managed-resource and ICC output policies. Version processing SHALL update only the Pictura-owned version record, its XMP properties, and supported 1057 fields as defined by their respective requirements. A foreign resource sharing the private numeric ID SHALL remain opaque.

#### Scenario: Version updates leave unrelated bytes intact [dvp_unrelated_bytes]
- **WHEN** a version-aware save updates version metadata in a file carrying other XMP properties, unknown native blocks, a foreign resource with the private ID, and valid framed unrelated resources
- **THEN** the unrelated properties, opaque blocks, foreign resource retain their original bytes

#### Scenario: Raw writes retain incomplete resource tails [dvp_raw_tail]
- **WHEN** the default raw codec writes a document with an unparsed resource tail
- **THEN** it retains the tail under the existing preservation and ICC rules; version-aware writing refuses the ambiguous section
