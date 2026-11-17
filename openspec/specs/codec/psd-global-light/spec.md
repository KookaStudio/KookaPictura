# psd-global-light Specification

## Purpose
Reads and writes the document-level global lighting resources (1037 angle,
1049 altitude) so effects that use the global light round-trip with a PSD.
## Requirements
### Requirement: Global light is read from its image resources

`pictura-codec` SHALL expose the document's global light as an angle and an
altitude in degrees. The angle SHALL come from image resource `1037` and the
altitude from image resource `1049`, each read as a four-byte big-endian signed
integer. A missing resource SHALL contribute Photoshop's default of 30 degrees
for that value, so a document with neither resource reads 30/30. A malformed
resource (fewer than four data bytes) SHALL contribute the default rather than
failing the open, and the parser SHALL NOT panic or allocate without bound.

#### Scenario: Both resources decode [pgl_read]

- **WHEN** a document carries resource `1037` holding `45` and resource `1049` holding `60`
- **THEN** its global light reads angle `45` and altitude `60`

#### Scenario: Missing or malformed resources take the default [pgl_default]

- **WHEN** a document carries no global-light resources, or a `1037` resource with two data bytes
- **THEN** the missing value reads `30` and opening the document does not fail or panic

### Requirement: Global light is written only when it changes

A PSD/PSB save SHALL write the document's angle to resource `1037` and its
altitude to resource `1049`, replacing each existing resource in place or
appending it. When either value differs from the stored pair, the save SHALL
write both resources. A document whose values still match the stored resources SHALL
keep its image-resource section byte-identical, including a section that carries
neither resource. Every other resource block and any unparsed trailing bytes
SHALL be preserved.

#### Scenario: An edited light is written and preserved [pgl_write]

- **WHEN** a document with a global angle of 30 and altitude of 30 is set to angle 90 and altitude 45 and saved
- **THEN** the written section holds `1037` = 90 and `1049` = 45, and an unrelated XMP resource keeps its bytes

#### Scenario: An untouched document round-trips its bytes [pgl_roundtrip]

- **WHEN** a document with no global-light resources is saved without editing the light
- **THEN** it writes no `1037` or `1049` resource and its image-resource bytes match the input

#### Scenario: Matching resources round-trip their bytes [pgl_roundtrip_matching]

- **WHEN** a document whose section already holds `1037` = 45 and `1049` = 60 is saved without editing the light
- **THEN** both resources keep their values and the image-resource section is byte-identical
