# codec/smart-filter-masks Specification

## Purpose
Reading, preserving, and deleting a smart filter stack's mask: the
document-level `FEid`/`FMsk` blocks and the `SmartObject::filter_mask` view.

## Requirements

### Requirement: The filter mask is read from the document blocks

The codec SHALL parse the document-level `FEid` (`FilterEffects`) and `FMsk`
blocks out of `Document::layer_section_extra` and attach the coverage to the
smart-object layer whose SoLd `placed` uuid matches (falling
back to `Idnt`), as
`SmartObject::filter_mask` (a `PsdRect` and a coverage plane). A document with
no such block SHALL leave `filter_mask` as `None`. The blocks SHALL remain
preserved verbatim so an unedited document re-saves byte-for-byte.

#### Scenario: A mask is read [sfm_read]

- **WHEN** `filter_effects::tests::filter_mask_reads_from_the_feid_block` and
  `filter_effects::tests::a_mask_attaches_to_its_smart_object_by_uuid` reads a
  document whose `FEid` carries a coverage channel for a smart object's uuid
- **THEN** that object's `filter_mask` holds the rect and the decoded coverage

#### Scenario: A document without a mask has none [sfm_absent]

- **WHEN** `filter_effects::tests::a_document_without_a_mask_has_none` reads a
  document with no `FEid`/`FMsk` block
- **THEN** every smart object's `filter_mask` is `None`

#### Scenario: An unedited mask round-trips [sfm_roundtrip]

- **WHEN** `filter_effects::tests::a_mask_survives_a_document_round_trip` reads and
  writes a document carrying a mask
- **THEN** the re-read document's mask rect and coverage equal the original's

### Requirement: The filter mask can be deleted

The codec SHALL expose an operation that removes the filter mask of the
smart-object layer at a uuid from the preserved `layer_section_extra`: it SHALL
copy every other global tagged block byte-for-byte, drop the matching `FEid`
entry, and drop the `FMsk` block when no `FEid` entry remains. It SHALL return
`None` (leaving the bytes untouched) when nothing matches or the block is
malformed.

#### Scenario: Deleting drops the blocks [sfm_delete]

- **WHEN** `filter_effects::tests::deleting_a_filter_mask_drops_the_blocks`
  removes a mask and re-reads the written document
- **THEN** the mask is `None` and no other block is changed

#### Scenario: An unrelated uuid is left alone [sfm_delete_absent]

- **WHEN** `filter_effects::tests::removing_an_unrelated_uuid_changes_nothing`
  removes a mask for a uuid no entry carries
- **THEN** the call returns `None`

### Requirement: Filter mask is authored on save

The writer SHALL author every smart object's `filter_mask` into the
document-level `FEid` block on save, keyed by the uuid the re-read object
carries: the authored uuid when its `SoLd` is authored this save, otherwise its
`placed` uuid, falling back to `Idnt`. An existing entry whose rect and coverage
equal the mask SHALL be kept byte-for-byte; a changed or missing entry SHALL be
replaced or appended alone, keeping every other entry. An `FMsk` block SHALL be
added only when the document has none. The `filterFXStyle.filterMaskEnable`
flag SHALL be settable on a preserved descriptor. Photoshop interop of an
authored entry is unverified.

#### Scenario: An authored mask reads back [sfm_author]

- **WHEN** `pictura-codec::authored_mask_reads_back` writes a document whose
  smart object has a mask and no `FEid` block, and reads it back
- **THEN** the re-read object's mask rect and coverage equal the authored ones

#### Scenario: An unchanged mask keeps its bytes [sfm_author_unchanged]

- **WHEN** `pictura-codec::unchanged_mask_bytes_preserved` writes a document
  read with a mask it did not edit
- **THEN** the `FEid` and `FMsk` bytes are unchanged

#### Scenario: A changed mask replaces only its entry [sfm_author_changed]

- **WHEN** `pictura-codec::changed_mask_replaces_entry_only` edits one of two
  masked objects' masks and writes
- **THEN** the edited entry carries the new coverage and the other entry's
  bytes are unchanged

#### Scenario: A converted object is keyed by its authored uuid [sfm_author_converted]

- **WHEN** `pictura-codec::converted_object_keyed_by_authored_uuid` writes a
  converted smart object whose `SoLd` is authored this save
- **THEN** the `FEid` entry is keyed by the authored uuid and the mask attaches
  on read

#### Scenario: An authored mask round-trips as PSB [sfm_author_psb]

- **WHEN** `pictura-codec::authored_mask_round_trips_psb` writes the document as
  PSB and reads it back
- **THEN** the mask attaches with the same coverage

#### Scenario: The mask enable flag round-trips [sfm_author_flag]

- **WHEN** `pictura-codec::filter_mask_enable_flag_round_trips` sets the flag on
  a preserved descriptor, writes, and reads back
- **THEN** the typed flag and the preserved `filterMaskEnable` agree

#### Scenario: Photoshop reopens an authored mask [sfm_author_photoshop]

- **WHEN** a document with an authored mask is opened in Photoshop
- **THEN** the mask shows on the Smart Filters row
