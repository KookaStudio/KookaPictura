# psd-smart-objects Specification

## Purpose
Resolves and round-trips embedded smart-object blocks and Pictura Raw crs settings, with Adobe interop fixtures.

## Requirements

### Requirement: Embedded smart-object source resolution
The system SHALL resolve, for a layer carrying a smart-object config descriptor,
the document-level linked source record whose `uuid` matches the descriptor, and
SHALL expose the matched record's embedded payload, filename, and filetype. A
matched record whose kind is external or alias SHALL be reported as
non-embedded without reading any path from disk.

#### Scenario: Embedded source resolved
- **WHEN** a layer's smart-object descriptor `uuid` matches an embedded linked record
- **THEN** `read_psd` exposes that record's payload bytes, filename, and filetype on the layer

#### Scenario: External source is not read
- **WHEN** the matching linked record is external
- **THEN** the source is reported as non-embedded and no file is read from disk

#### Scenario: Unresolved object does not fail
- **WHEN** a layer carries a smart-object descriptor with no matching linked record
- **THEN** `read_psd` succeeds and the block stays preserved as unresolved

### Requirement: Smart-object blocks round-trip byte-for-byte
The system SHALL preserve through read and write the smart-object config
descriptors (`SoLd`/`SoLE`, legacy `plLd`), the document-level linked records
(`lnkD`/`lnk2`/`lnk3`/`lnkE`), the embedded payload, and the `uuid` link, so that
a read→write→read round-trip reproduces them exactly.

#### Scenario: Embedded smart object survives
- **WHEN** a PSD with an embedded smart object is read and then written
- **THEN** the config descriptor, linked record, payload, and `uuid` are unchanged after re-reading

#### Scenario: Payload integrity
- **WHEN** the embedded payload is non-empty
- **THEN** its bytes after read→write→read equal the original

#### Scenario: External record preserved
- **WHEN** a document carries an external linked record
- **THEN** read→write→read preserves the record without dereferencing it

#### Scenario: Unrecognized descriptor version is preserved
- **WHEN** a layer carries a `SoLE` descriptor or an unrecognized descriptor block version
- **THEN** the preserved block re-emits unchanged and no error is raised

### Requirement: Embedded smart-object authoring
Given an embedded source payload and a composite, the writer SHALL emit a
smart-object layer with a config descriptor and a linked source record whose
`uuid` matches, and SHALL place the payload as the record's embedded data. When
authoring a new object with no input descriptor, the emitted descriptor SHALL be
`SoLd` with outer version 4 and descriptor block version 16, so Photoshop CS6 can
reopen it. The written file SHALL be readable by the codec and by an independent
PSD reader as a smart object carrying the same payload.

#### Scenario: Author and re-read
- **WHEN** a document with an embedded source is written and read back
- **THEN** the smart object resolves with the same payload and filename

#### Scenario: Independent reader agrees
- **WHEN** the written file is parsed by `psd-tools`
- **THEN** it is reported as a smart object whose embedded data equals the payload

#### Scenario: No payload corruption
- **WHEN** the payload is a fixed-seed random byte string
- **THEN** its hash after write and read is unchanged

### Requirement: Pictura Raw crs settings are preserved

The system SHALL expose the `crs:` XMP settings associated with an embedded
source on read as a typed view of the fixed property set (`Exposure2012`,
`Contrast2012`, `Highlights2012`, `Shadows2012`, `Whites2012`, `Blacks2012`,
`Clarity2012`, `Vibrance`, `Saturation`, `Temperature`, `Tint`), each an
optional finite `f64`. Properties outside that set SHALL remain only inside the
raw packet. An unedited document SHALL re-emit the embedded payload and the
preserved `lnk*` section byte-exact on write.

The system SHALL provide `set_crs_property(document, uuid, name, value)` that
replaces one fixed-set property in the packet, updates the smart object's
payload and typed view, and rewrites the matching embedded payload inside the
document's preserved linked-record section so a subsequent write emits the new
value. A name outside the fixed set, a missing uuid, a non-embedded object, or
a packet that cannot be safely patched SHALL return an error and MUST NOT
mutate the document.

#### Scenario: Settings are exposed on read

- **WHEN** an embedded source carries a packet with `crs:Exposure2012="+0.50"`
- **THEN** the typed view's exposure is `Some(0.5)` and the raw packet is retained

#### Scenario: An unedited document round-trips byte-exact

- **WHEN** a document with `crs:` settings is written without calling the edit API
- **THEN** the written linked-record section is byte-identical to the input

#### Scenario: Editing exposure round-trips through write

- **WHEN** `set_crs_property` sets `Exposure2012` to `1.25` on an embedded source
  and the document is written and read back
- **THEN** the typed view's exposure is `Some(1.25)` and bytes outside the
  patched packet span are unchanged

#### Scenario: An unknown property name is rejected without mutation

- **WHEN** `set_crs_property` is called with a name outside the fixed set
- **THEN** it returns an error and the document's linked-record section is unchanged

#### Scenario: A missing uuid is rejected without mutation

- **WHEN** `set_crs_property` is called with a uuid that matches no layer
- **THEN** it returns an error and the document is unchanged

### Requirement: Adobe Photoshop interop is fixture-validated
The system SHALL round-trip Photoshop-produced reference PSDs containing an
embedded smart object so that the config descriptor, linked record, and payload
are preserved. The interop acceptance SHALL be the `psd-tools` oracle parsing
the written file; a manual Photoshop CC reopen MAY be recorded as a deferred
follow-up where Photoshop is available. Reference fixtures SHALL be
self-produced and their provenance recorded.

#### Scenario: Photoshop embedded smart object round-trips
- **WHEN** `test_with_smart_object01.psd` is read and written
- **THEN** the smart object parses identically in the output and its payload is unchanged

#### Scenario: Shared source across layers
- **WHEN** two layers reference the same linked record by `uuid`
- **THEN** both resolve to the same payload and the output preserves the single record

#### Scenario: Automated interop acceptance
- **WHEN** the written file is parsed by psd-tools
- **THEN** the smart object, its uuid, and its embedded payload match the source fixture

### Requirement: The Pictura stack-mode marker round-trips

The writer SHALL author a Pictura-private layer block, key `PiSt`, carrying a
one-byte version (`1`) and a one-byte mode code, when a layer's smart object has
an active stack mode, and SHALL emit at most one `PiSt` block per layer: a
preserved `PiSt`, recognized or not, SHALL be replaced when the model authors a
marker. The reader SHALL set `SmartObject.stack_mode` from a `PiSt` block with a
recognized version and mode code and SHALL treat that block as modelled rather
than re-emitting it as an unknown block. A `PiSt` block with an unrecognized
version or code SHALL NOT set a mode and SHALL be preserved in the layer's
unmodelled blocks, but SHALL be re-emitted only when no mode is modelled.
Clearing the mode SHALL remove the authored block on the next write. The block
SHALL NOT alter the layer's raster proxy, and the existing `SoLd`/`lnk*` bytes
SHALL be unchanged.

#### Scenario: A written mode reads back [cso_stack_round_trip]
- **WHEN** a document whose layer carries an active stack mode is written and read back
- **THEN** the layer reports the same mode and the payload and config descriptor are unchanged

#### Scenario: Clearing removes the marker [cso_stack_cleared]
- **WHEN** the mode is cleared and the document is written and read back
- **THEN** the layer reports no mode and the output carries no `PiSt` block

#### Scenario: An unknown marker is preserved [cso_stack_unknown]
- **WHEN** a layer carries a `PiSt` block with an unrecognized version or mode code
- **THEN** the layer reports no mode and the block is re-emitted unchanged

#### Scenario: A modelled mode replaces an unknown marker [cso_stack_replaced]
- **WHEN** a layer carries an unrecognized `PiSt` block and a mode is applied and written
- **THEN** the output carries exactly one `PiSt` block, the modelled marker

#### Scenario: The proxy is untouched [cso_stack_proxy]
- **WHEN** a mode is applied to a converted object and the document is written
- **THEN** the layer's colour and alpha channel bytes equal the pre-mode bytes
