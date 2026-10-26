# imaging/lens-correction-smart-filter Specification

## Purpose
Lens Correction as a non-destructive Smart Filter on a Smart Object: the
descriptor codec, the smart-object apply, the composite render, and the dialog
re-edit. The descriptor is Pictura-only (see the change design): it re-opens in
Pictura and preserves keys it does not model, but claims no cross-app interop.

## Requirements

### Requirement: Lens Correction smart filter descriptor

The codec SHALL read and write a Lens Correction smart filter's settings as a
descriptor of the dialog's 16 slots under `Fltr`, with a Pictura-only
`filterID` and display name, and SHALL return each slot's opening value when
its key is missing or malformed. Attaching SHALL go through the generic
smart-filter attach, so re-attaching over a preserved descriptor keeps the
entry's unmodeled `Fltr` keys and `blendOptions`, and a filter id this codec
does not model is preserved untouched.

#### Scenario: Slots round-trip through the descriptor [lc_smart_descriptor]

- **WHEN** `pictura-codec::slots_round_trip_through_the_descriptor` encodes a
  full slot list and decodes it again
- **THEN** the decoded slots equal the encoded ones

#### Scenario: Missing keys decode to the opening slots [lc_smart_defaults]

- **WHEN** `pictura-codec::missing_keys_decode_to_the_opening_slots` decodes an
  empty descriptor and one missing a single key
- **THEN** every missing slot takes the descriptor's opening value

#### Scenario: Attaching records the filter [lc_smart_attach]

- **WHEN** `pictura-codec::attach_records_the_filter` attaches slots to an
  embedded smart object and reads the filter back
- **THEN** the typed filter list carries the Lens Correction id and name and
  the decoded slots round-trip

#### Scenario: Re-attaching keeps unmodeled keys [lc_smart_attach_keeps]

- **WHEN** `pictura-codec::reattaching_keeps_unmodeled_keys` re-attaches over a
  preserved descriptor whose entry has an extra `Fltr` key
- **THEN** the extra key and `blendOptions` survive and the modelled keys are
  the new values

#### Scenario: A foreign filter id is left alone [lc_smart_foreign]

- **WHEN** `pictura-codec::a_foreign_filter_id_is_left_alone` reads a smart
  object whose filter id is not the Lens Correction id
- **THEN** its options bytes are unchanged

### Requirement: Lens Correction smart filter rendering

The smart-filter chain SHALL apply a Lens Correction smart filter, building its
parameters from the stored slots with the profile resolved from the bundled set
and gated by the Auto-tab checkboxes. A slot list that does not map SHALL make
the filter undecodable, leaving the layer on its proxy path.

#### Scenario: The filter renders the chain [lc_smart_render]

- **WHEN** `pictura-render::lens_correction_renders_the_chain` applies the
  chain to a buffer with a Lens Correction filter
- **THEN** the result equals `pictura_filters::lens_correction` with the same
  parameters

#### Scenario: Unmappable slots fall back [lc_smart_fallback]

- **WHEN** `pictura-render::unmappable_slots_are_refused` decodes a filter
  whose profile id is past the bundled set and one with a bad slot count
- **THEN** the decode returns nothing, so the layer is not smart-filtered

### Requirement: Lens Correction commit on a Smart Object

A `lens-correction` commit on an embedded Smart Object SHALL attach a Lens
Correction smart filter and re-render the layer's proxy rather than filter it
destructively, recording one history state and the kind's 16 slots for Last
Filter and Last Filter Settings; a plain raster layer SHALL keep the
destructive commit. Every commit path SHALL keep the 8-/16-bit RGB and
Grayscale gate.

#### Scenario: A Smart Object gains the filter [lc_smart_commit]

- **WHEN** `pictura-render::smart_apply_attaches_the_filter` and
  `tst_lens_correction::smartObjectAddsASmartFilter` apply on an embedded smart
  object
- **THEN** a Lens Correction smart filter is attached, the composite shows the
  corrected pixels, and one history state is recorded with the 16 slots

#### Scenario: A raster layer stays destructive [lc_smart_raster]

- **WHEN** `tst_lens_correction::rasterLayerStaysDestructive` applies on a
  plain raster layer
- **THEN** no smart object is created and the layer's pixels change

#### Scenario: The depth gate still holds [lc_smart_gate]

- **WHEN** `tst_lens_correction::smartObjectRefusesOffTheGate` runs the commit
  on a 32-bit document
- **THEN** the filter is refused with a message naming "8- or 16-bit"

### Requirement: Lens Correction smart filter re-edit

Opening the dialog on a Smart Object carrying a Lens Correction smart filter
SHALL prefill the stored settings, and the session's source SHALL be the smart
object's source so a re-edit never re-applies the baked correction; the next
commit SHALL replace the stored settings rather than add a second entry.

#### Scenario: The dialog prefills the stored settings [lc_smart_prefill]

- **WHEN** `pictura-render::stored_slots_read_back` and
  `tst_lens_correction::smartObjectPrefillsOnReEdit` reopen the dialog after a
  commit
- **THEN** the stored slots are the committed ones and the dialog's controls
  show them

#### Scenario: A re-edit does not double-apply [lc_smart_no_double]

- **WHEN** `pictura-app::session_source_is_the_unfiltered_smart_source` builds
  a session source for a smart object whose proxy is already corrected
- **THEN** the source is the unfiltered embedded source, not the baked proxy

#### Scenario: The commit replaces the entry [lc_smart_replace]

- **WHEN** `pictura-render::smart_apply_replaces_the_entry` applies a second
  time with different slots
- **THEN** the layer still has one Lens Correction filter, holding the new
  slots
