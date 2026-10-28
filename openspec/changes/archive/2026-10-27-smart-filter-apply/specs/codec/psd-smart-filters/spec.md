## MODIFIED Requirements

### Requirement: Generic smart-filter attachment

The codec SHALL expose a generic operation that attaches a single `SmartFilter` to a layer. When the layer has a preserved `SoLd`/`SoLE` descriptor it SHALL insert a new `filterID` entry or merge into the existing one, refreshing the modelled `Fltr` keys while keeping unmodeled keys and `blendOptions`, and rewrite the preserved block consistently; when it is an embedded smart object without a preserved block it SHALL record the filter in `SmartObject::smart_filters`. It SHALL refuse a layer with no smart object or a non-embedded/legacy placed object without mutating it. The codec SHALL also expose an append variant that always adds a new entry, even when one with the same `filterID` exists, to both the preserved `filterFXList` and the typed `smart_filters`, so their indices stay aligned.

#### Scenario: Attaching into a preserved descriptor

- **WHEN** `pictura-codec::attach_inserts_into_a_preserved_sold_and_preserves_other_keys` attaches a smart filter to a layer carrying a preserved `SoLd`
- **THEN** the `filterFXList` gains or replaces the matching `filterID` entry and the typed `smart_filters` view matches

#### Scenario: Re-attaching over a modelled filter keeps unmodeled keys

- **WHEN** `pictura-codec::reattach_over_a_preserved_filter_keeps_unmodeled_keys` attaches a filter whose `filterID` already has an entry over a preserved descriptor
- **THEN** the entry is not duplicated, the new `Fltr` keys win, and unmodeled keys such as `Upri` and the entry's `blendOptions` survive

#### Scenario: Attaching to an embedded object without a preserved block

- **WHEN** `pictura-codec::attach_authors_then_write_read_round_trips` attaches a smart filter to an embedded object with no preserved block
- **THEN** the filter is recorded in the typed `smart_filters` list and authored on save

#### Scenario: Non-smart or legacy layers are refused

- **WHEN** `pictura-codec::attach_errors_without_a_smart_object` attaches to a target that has no smart object or is a non-embedded/legacy placed object
- **THEN** the attach is refused and the layer is unchanged

#### Scenario: Appending stacks a same-id filter [psf_append]

- **WHEN** `pictura-codec::append_stacks_same_id_preserved` and `pictura-codec::append_stacks_same_id_typed` append two filters with one `filterID`, over a preserved descriptor and without one
- **THEN** both entries are kept in order and the preserved list and the typed list have the same length
