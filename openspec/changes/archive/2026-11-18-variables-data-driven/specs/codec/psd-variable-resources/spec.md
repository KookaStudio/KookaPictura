# PSD variable resources

## Purpose

Preserves editable variable bindings and data sets across PSD and PSB save and reopen and reads the supported Photoshop resource format.

## ADDED Requirements

### Requirement: PSD resources resolve variables and data sets

The PSD and PSB readers SHALL resolve `MeSa` image resource `7000` as Adobe Variables 1.0 `variableSets` data and `7001` as `sampleDataSets` data. Layer references of the form `id('N')` SHALL resolve through stable PSD layer IDs. The supported traits SHALL be visibility, `textcontent`, and `fileref`. Each named `sampleDataSet` SHALL resolve its variable values by child element name. The reader SHALL retain valid bindings when a binding references a missing layer and report the unresolved binding. Only wire values with independently established semantics SHALL be interpreted as Photoshop values; explicitly distinguished Kooka-authored fallback values SHALL round-trip under their documented Kooka meanings without being presented as Photoshop-compatible. The observed `ratio` / `center` / `middle` / `clip=false` combination SHALL resolve as As Is with centered alignment and no clipping. Unconfirmed tokens or combinations SHALL be reported and preserved without assigning placement semantics. Malformed, unsupported, and ambiguous resource data SHALL leave the document openable and retain the affected raw bytes. Eager and lazy PSD open paths SHALL use the same variable metadata resolution after the layer tree is available.

#### Scenario: A Photoshop-style mixed template opens [vars_psd_read]

- **WHEN** a PSD contains supported `7000` and `7001` XML with linked layer IDs, visibility, Unicode text, and replacement file values
- **THEN** the application exposes the expected bindings, placement choices, and named values

#### Scenario: Bad resources and stale IDs are recoverable [vars_psd_preserve]

- **WHEN** a resource is truncated, carries unsupported XML, or refers to a missing or ambiguous layer ID
- **THEN** the file still opens, valid data is available where it can be resolved safely, and an unrelated save keeps the original resource block bytes

#### Scenario: Observed Photoshop ratio placement resolves [vars_psd_ratio]

- **WHEN** a Photoshop-style binding uses `ratio`, `center`, `middle`, and `clip=false` with source and frame dimensions that differ
- **THEN** the binding resolves as centered, unclipped As Is and preserves source pixel dimensions when applied

#### Scenario: Unknown placement values remain recoverable [vars_psd_unknown_placement]

- **WHEN** a Photoshop-style resource contains an unconfirmed placement token or option combination
- **THEN** the affected binding is reported as unresolved, its raw resource is preserved, and unrelated saves do not silently assign placement semantics

#### Scenario: Duplicate source IDs remain ambiguous [vars_psd_duplicate_ids]

- **WHEN** a template with layer comps contains two original `lyid` values equal to a variable `docRef`, and is opened, saved without related edits, and reopened
- **THEN** the reference is reported as ambiguous on both opens, never binds either layer, and original identity evidence and affected resources survive the save

### Requirement: Authored templates reopen with the same values

The writer SHALL use `MeSa` framing for authored `7000` and `7001` blocks. It SHALL maintain stable PSD layer IDs and the layer ID seed in documents with variables, including documents without layer comps. A save after a variable edit SHALL update the corresponding variable resources and preserve other resources. A save after removing all variables SHALL remove the corresponding authored variable resources. An unedited template SHALL retain the original variable resource bytes. Unsupported data in a block SHALL be preserved on unrelated saves. A variable edit that would overwrite unsupported data SHALL be refused with a reason. When original duplicate layer IDs make a reference ambiguous, unrelated saves SHALL retain the original IDs for the affected layers and the dependent raw resources; edits that would overwrite this evidence or dependent opaque comp data SHALL be refused with a reason. A variable-free document with no variable resources SHALL keep its established output.

#### Scenario: Authored templates round-trip through PSD and PSB [vars_psd_roundtrip]

- **WHEN** a template with all three variable types, linked targets, and custom placement is saved and reopened as PSD and PSB with no layer comps present
- **THEN** the original stable IDs and values resolve, the writer uses `MeSa`, and a subsequently assigned ID does not collide

#### Scenario: Resource edits compose with other metadata [vars_psd_rewrite]

- **WHEN** variables and another image resource are edited, saved and reopened, and all variables are then removed and saved
- **THEN** the first save contains both edits and the second removes authored Variables and Data Sets blocks while preserving unrelated resources
