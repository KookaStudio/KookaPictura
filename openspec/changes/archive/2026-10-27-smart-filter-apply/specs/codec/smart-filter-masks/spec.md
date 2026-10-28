## ADDED Requirements

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
