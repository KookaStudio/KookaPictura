# codec/smart-filter-masks Specification

## ADDED Requirements

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
