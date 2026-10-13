# psd-smart-filters Specification

## Purpose
Round-trips smart-filter blocks byte-for-byte, including Pictura Raw Filter settings, checked with fixtures.

## Requirements

### Requirement: Smart-filter blocks round-trip byte-for-byte
The system SHALL preserve through read and write a smart-object layer's
`filterFX` list inside its `SoLd` descriptor, the document-level filter-effects
blocks (`FEid`/`FXid`), and the filter mask (`FMsk`), so that a read→write→read
round-trip reproduces them exactly.

#### Scenario: filterFX survives
- **WHEN** a smart-object layer carries a `filterFX` list
- **THEN** read→write→read preserves the list entries, their order, and their bytes

#### Scenario: Filter effects and mask survive
- **WHEN** a document carries `FEid`/`FXid` or `FMsk` blocks
- **THEN** read→write→read preserves those blocks

#### Scenario: A filter we do not model is preserved
- **WHEN** a smart filter's `filterID` is not the Pictura Raw filter
- **THEN** its options descriptor bytes are preserved unchanged

### Requirement: Pictura Raw Filter settings are read and written
The system SHALL expose, for a smart filter whose `filterID` is 2683, its options
descriptor named `Fltr`, and SHALL re-emit it on write. The settings model SHALL
target the earliest CC Camera Raw Filter (Photoshop CC v14, ACR 8, process
version PV2012), and `Fltr` keys SHALL be read with the mapping recorded in
`docs/dev/camera-raw-cc-notes.md`. Keys the system does not model, including the
later-CC `Upri`, `GuUr`, `Rtch`, `REye`, and `LCs ` keys, SHALL be
preserved unchanged. Dehaze (`Dhze`) SHALL be modeled rather than preserved.

#### Scenario: Settings exposed
- **WHEN** a layer carries a Pictura Raw filter with a `Fltr` descriptor
- **THEN** read exposes the filter options keyed by their short names

#### Scenario: Settings edit round-trips
- **WHEN** a Pictura Raw filter option is changed through the API
- **THEN** the written `Fltr` reflects the change and unmodeled keys are retained

#### Scenario: Filter identity is preserved
- **WHEN** a Pictura Raw filter is written
- **THEN** its `filterID` is 2683 and its display name is present in the entry

### Requirement: Smart-filter interop is fixture-validated
The system SHALL round-trip the supplied Photoshop CC reference PSD containing a
Camera Raw Filter so that the smart object, the `filterFX` block, the `Fltr`
settings, and the `FEid`/`FMsk` blocks are preserved. The interop acceptance
SHALL be the `psd-tools` oracle parsing the written file; a manual Photoshop CC
reopen MAY be recorded as a deferred follow-up where Photoshop is available.

#### Scenario: Camera Raw Filter fixture round-trips
- **WHEN** `test_with_smart_object02.psd` is read and written
- **THEN** the smart-filter blocks and `Fltr` settings parse identically in the output

#### Scenario: Automated interop acceptance
- **WHEN** the written file is parsed by psd-tools
- **THEN** the smart-filter blocks, `filterID` 2683, and the `Fltr` settings match the source fixture

#### Scenario: Sixteen-bit variant is deferred
- **WHEN** a 16-bit fixture is produced
- **THEN** it is added to the oracle once roadmap G4 depth support lands

### Requirement: Smart-filter group enable flag round-trips

The smart-object codec SHALL read the `filterFXStyle` group enable flag (`enab`) into `SmartObject::smart_filters_enabled` (defaulting to enabled when absent) and SHALL author it when writing `filterFX`, so a document whose group is disabled re-reads as disabled and a document whose group is enabled re-reads as enabled. The flag SHALL be preserved independently of the per-filter `enab`.

#### Scenario: A disabled group round-trips

- **WHEN** a smart object with a disabled `filterFXStyle` group is read and written
- **THEN** reading the written document reports the group disabled

#### Scenario: A missing flag defaults to enabled

- **WHEN** a `filterFXStyle` without the `enab` key is read
- **THEN** the group is treated as enabled

### Requirement: Generic smart-filter attachment

The codec SHALL expose a generic operation that attaches a single `SmartFilter` to a layer. When the layer has a preserved `SoLd`/`SoLE` descriptor it SHALL insert a new `filterID` entry or merge into the existing one, refreshing the modelled `Fltr` keys while keeping unmodeled keys and `blendOptions`, and rewrite the preserved block consistently; when it is an embedded smart object without a preserved block it SHALL record the filter in `SmartObject::smart_filters`. It SHALL refuse a layer with no smart object or a non-embedded/legacy placed object without mutating it.

#### Scenario: Attaching into a preserved descriptor

- **WHEN** a smart filter is attached to a layer carrying a preserved `SoLd`
- **THEN** the `filterFXList` gains or replaces the matching `filterID` entry and the typed `smart_filters` view matches

#### Scenario: Re-attaching over a modelled filter keeps unmodeled keys

- **WHEN** a filter whose `filterID` already has an entry is attached again over a preserved descriptor
- **THEN** the entry is not duplicated, the new `Fltr` keys win, and unmodeled keys such as `Upri` and the entry's `blendOptions` survive

#### Scenario: Attaching to an embedded object without a preserved block

- **WHEN** a smart filter is attached to an embedded object with no preserved block
- **THEN** the filter is recorded in the typed `smart_filters` list and authored on save

#### Scenario: Non-smart or legacy layers are refused

- **WHEN** the target has no smart object or is a non-embedded/legacy placed object
- **THEN** the attach is refused and the layer is unchanged

### Requirement: Smart-filter list editing on the preserved descriptor

The codec SHALL expose operations to remove the smart filter at a given index
from a layer, to move an entry to another position, and to clear every entry.
Each operation SHALL rewrite the layer's preserved `SoLd`/`SoLE`
`filterFX.filterFXList` in place, keeping every other descriptor key, and SHALL
keep the typed `SmartObject::smart_filters` view in step. When a layer has no
preserved block the operation SHALL change only the typed view so the writer
authors the block on save. Removing the last entry SHALL drop the whole
`filterFX` object from the descriptor. An out-of-range index SHALL return an
error and SHALL NOT mutate the layer.

#### Scenario: A deleted entry disappears and survives a write

- **WHEN** a smart filter is deleted from a layer whose preserved descriptor carries two entries and the document is written and read back
- **THEN** the removed filter no longer appears in the re-read list and the surviving entry keeps its bytes and its position

#### Scenario: Deleting the last filter drops the filterFX object

- **WHEN** the only smart filter is deleted
- **THEN** the typed list is empty and a re-read document authors no `filterFX` list

#### Scenario: Reordering changes the applied order and persists

- **WHEN** the filter at index 0 is moved to index 1
- **THEN** the typed list order changes, the preserved `filterFXList` order changes to match, and the new order is unchanged by a write and re-read

#### Scenario: Clearing removes every filter

- **WHEN** clear is called on a layer with one or more smart filters and the document is written and read back
- **THEN** the re-read layer has an empty `smart_filters` list

#### Scenario: An out-of-range index is refused

- **WHEN** delete or reorder is called with an index outside the list
- **THEN** the call returns an error and the typed and preserved lists are unchanged
