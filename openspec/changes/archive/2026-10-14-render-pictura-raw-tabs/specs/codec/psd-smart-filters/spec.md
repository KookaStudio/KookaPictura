# Spec Delta

## MODIFIED Requirements

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
