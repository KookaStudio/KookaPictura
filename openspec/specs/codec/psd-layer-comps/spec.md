# codec/psd-layer-comps Specification

## Purpose
Reads and writes layer comps in PSD and PSB files through image resource 1065
and each layer's `shmd` state block, preserving untouched bytes and layer
identity.

## Requirements
### Requirement: Read the document comp list

`read_psd` SHALL parse image resource 1065 as a version-16 descriptor whose class
is `CompList` and whose `list` holds one `Comp` object per comp. Each comp SHALL
provide its display name from `Nm  `, its numeric id from `compID`, and its
recorded-class bit field from `capturedInfo` (visibility bit 1, position bit 2,
appearance bit 4; the bit assignment is the observed one, with bits 2 and 4
inferred pending the manual Photoshop check). A document without resource 1065
SHALL have no comps. A malformed, truncated, or unsupported comp resource SHALL
NOT fail the open; the document SHALL open without comps and the original
resource bytes SHALL be kept for a lossless re-save.

#### Scenario: A comp list is read [psdlc_read_list]

- **WHEN** a PSD with three comps named A, B, and C and captured info values 1, 3, and 7 is opened
- **THEN** the document reports three comps with those names and recorded classes

#### Scenario: No comp resource means no comps [psdlc_read_absent]

- **WHEN** a PSD has no resource 1065
- **THEN** the document opens with an empty comp list

#### Scenario: A malformed comp resource does not fail the open [psdlc_read_malformed]

- **WHEN** resource 1065 holds a truncated descriptor
- **THEN** the document opens without comps and the resource bytes are preserved for re-save

#### Scenario: A comment is read when present [psdlc_read_comment]

- **WHEN** a comp object carries a text comment under a recognized comment key
- **THEN** the comp reports that comment

### Requirement: Read per-layer captured state

`read_psd` SHALL read each layer's `shmd` additional-layer block and its `cmls`
descriptor. A `cmls` descriptor SHALL provide the layer id from `LyrI`, the
layer's base position from `origFXRefPoint`, and a list of state variants. Each
variant SHALL name the comps it belongs to through `compList` and MAY carry
visibility (`enab`), a blend mode (`blendOptions.Md`), a captured position
(`FXRefPoint`), a vector-mask offset (`vectorMask.Ofst`), and layer-specific
appearance (`layerSpecific`). A variant whose `compList` contains `0` SHALL be
treated as the document base state, not as a comp. Variants SHALL be attached to
the comp whose `compID` they name; a variant for an unknown comp SHALL be
ignored and preserved on re-save. A class not enabled by the matching comp's
`capturedInfo` SHALL NOT be applied even if the variant carries the key.

The `shmd` entry list SHALL use the observed 8-byte entry length; this length
rule is specific to the `shmd` block and does not change the general
tagged-block framing. The layer's position anchor SHALL be the preserved `fxrp`
value when present; otherwise `origFXRefPoint` SHALL supply it. Reading a file
SHALL update the in-memory `fxrp` value to that anchor when the file's
`origFXRefPoint` differs.

#### Scenario: Per-layer visibility deltas are read [psdlc_read_visibility]

- **WHEN** a layer's `cmls` holds a variant tagged with comp A and `enab = false`
- **THEN** comp A captures that layer as hidden

#### Scenario: Position and blend deltas are read [psdlc_read_position_blend]

- **WHEN** a variant carries `FXRefPoint` and `blendOptions.Md` and its comp records position and appearance
- **THEN** comp A captures that layer's position and blend mode

#### Scenario: The base variant is not a comp [psdlc_read_base]

- **WHEN** a variant's `compList` contains only `0`
- **THEN** no comp is created from it and it is not applied as one

#### Scenario: Unknown comp variants survive [psdlc_read_unknown_variant]

- **WHEN** a layer has a variant for a comp id not present in resource 1065
- **THEN** the open succeeds and the variant's bytes are preserved on an unedited save

#### Scenario: The position anchor is read [psdlc_read_anchor]

- **WHEN** a layer's `origFXRefPoint` differs from its current `fxrp` value
- **THEN** the in-memory anchor becomes `origFXRefPoint` and the codec can apply that position

### Requirement: Write the comp list

On save, a document with comps SHALL emit image resource 1065 as a version-16
`CompList` descriptor. The comps SHALL be written in panel order, each carrying
`Nm  ` for the name, `compID` for the id, `capturedInfo` for the recorded
classes, and a comment field. A document whose comp list became empty SHALL omit
resource 1065. A document whose comps were not edited SHALL re-emit the original
resource 1065 bytes unchanged, including unknown keys. Other resource rewrites
(ICC, paths, guides) SHALL NOT drop resource 1065.

#### Scenario: Comps round-trip through 1065 [psdlc_write_list]

- **WHEN** a document with two comps is written and read back
- **THEN** both comps keep their names, ids, recorded classes, and comments

#### Scenario: Deleting every comp removes the resource [psdlc_write_empty]

- **WHEN** the last comp is deleted and the document is saved
- **THEN** the output has no resource 1065 and no `cmls` comp entries, while other `shmd` entries remain

#### Scenario: An unedited comp resource is byte-identical [psdlc_write_untouched]

- **WHEN** a PSD with resource 1065 is opened and saved without changing comps
- **THEN** the output's resource 1065 bytes equal the input's

#### Scenario: Unknown comp keys survive an unedited save [psdlc_write_unknown]

- **WHEN** resource 1065 carries a key the codec does not model and no comp is edited
- **THEN** the output preserves that key and its bytes

#### Scenario: A path or ICC rewrite keeps comps [psdlc_write_resource_chain]

- **WHEN** a document with comps also changes a path or ICC resource and is saved
- **THEN** resource 1065 is still present and unchanged when the comps were not edited

### Requirement: Write per-layer captured state

On save, a document whose comps changed SHALL write each participating layer's
`shmd` block with a `cmls` descriptor containing `LyrI`, `origFXRefPoint`, and
one `layerSettings` variant per comp that captured that layer, tagged by
`compList`. Visibility SHALL be written as `enab`, blend mode as
`blendOptions.Md`, and position as `FXRefPoint`. A vector-mask position SHALL be
written as `vectorMask.Ofst`, and layer-specific appearance SHALL be written as
`layerSpecific`. Unknown keys inside a rewritten variant SHALL be preserved by
merging with the original variant that has the same `compList`. A base variant
tagged `0` SHALL be written with the layer's current recorded state. Other
entries in the layer's `shmd` block, including unknown entries and the preserved
timestamp entry, SHALL be preserved. A layer that no comp captured SHALL keep
its original `shmd` block unchanged, except when the store is dirty and empty:
then every layer's `cmls` entries are removed while its other `shmd` entries
remain.

#### Scenario: Captured state round-trips [psdlc_write_layer]

- **WHEN** a document with a visibility and appearance comp is written and read back
- **THEN** the comp captures the same layers with the same visibility and blend values

#### Scenario: Position round-trips [psdlc_write_position]

- **WHEN** a position-recording comp is written and read back
- **THEN** applying it in the reopened document restores the captured positions

#### Scenario: A vector-mask position round-trips [psdlc_write_vector_mask]

- **WHEN** a position-recording comp captures a shape or vector-mask layer, the document is saved, and the comp is applied after reopen
- **THEN** the vector-mask offset and outline return to the captured position

#### Scenario: A non-participating layer keeps its shmd [psdlc_write_nonparticipant]

- **WHEN** a layer is captured by no comp, the store is not dirty-empty, and the layer has an existing `shmd` block
- **THEN** its block is byte-identical after save

#### Scenario: A dirty-empty store removes cmls [psdlc_write_dirty_empty]

- **WHEN** the comp list is empty and dirty and the document is saved
- **THEN** every layer's `cmls` entry is gone while its other `shmd` entries remain

#### Scenario: The timestamp entry survives [psdlc_write_timestamp]

- **WHEN** a layer's `shmd` block carries its preserved timestamp entry and the comps are rewritten
- **THEN** the timestamp entry is still present after save

#### Scenario: Unknown variant keys survive a dirty rewrite [psdlc_write_unknown_variant]

- **WHEN** a variant that is being rewritten carries an unknown key and the same `compList` identifies the original variant
- **THEN** the unknown key and its bytes are present after save

### Requirement: Layer identity round-trips

The codec SHALL read a layer's stable identity from the `lyid` additional-layer
block as a big-endian 32-bit integer and SHALL write the identity supplied by
the document for every layer that needs one. The document allocator assigns
identities lazily when a comp first needs them; the codec SHALL pass resource
1044's seed to that allocator and SHALL update resource 1044 to a value greater
than every written identity whenever identities were added or changed. A
comp-free document is not normalized: a PSD with no comps and no `lyid` blocks
SHALL save byte-identically. PSD and PSB SHALL use the same identity semantics.

#### Scenario: lyid values survive a round-trip [psdlc_id_roundtrip]

- **WHEN** a document with `lyid` blocks is written and read back
- **THEN** every layer reports the same identity it had before

#### Scenario: An assigned identity is written [psdlc_id_write_assigned]

- **WHEN** the document allocator assigns an identity to a layer and the document is saved
- **THEN** the layer carries a `lyid` block on disk

#### Scenario: The 1044 seed is advanced [psdlc_id_seed]

- **WHEN** a save writes a newly assigned identity greater than the existing 1044 seed
- **THEN** resource 1044 is at least that identity

#### Scenario: A comp-free file is unchanged [psdlc_id_compfree]

- **WHEN** a PSD with no comps and no `lyid` blocks is opened and saved without comp edits
- **THEN** the output is byte-identical to the input

#### Scenario: A PSB keeps comp identity [psdlc_id_psb]

- **WHEN** a PSB with comps is written and read back
- **THEN** its comps resolve the same layers and apply without warnings

#### Scenario: A duplicate does not inherit captured state [psdlc_id_duplicate]

- **WHEN** a captured layer is duplicated and the document is saved and reopened
- **THEN** the copy has no captured comp entry and the original still does

### Requirement: Comp state that is not a comp is preserved

An apply-then-save SHALL refresh the base `cmls` variant tagged `0` from the
current layer state. Warning state SHALL be stored in project-owned descriptor
keys (the captured id list and the cleared-id set) and SHALL survive
save/reopen; Photoshop may ignore those keys without losing the standard comp
data.

#### Scenario: The base variant refreshes after apply [psdlc_base_variant_refresh]

- **WHEN** a comp is applied and the document is saved without other comp edits
- **THEN** the compID-0 base variant reflects the applied layer state

#### Scenario: Warning data survives save and reopen [psdlc_warning_persist]

- **WHEN** a comp has a warning and cleared identities, the document is saved, and it is reopened
- **THEN** the comp reports the same warning and clears

### Requirement: Comp persistence never fails the open or the save

Malformed comp data SHALL be tolerated: an unreadable resource 1065, an
unreadable `shmd`/`cmls` block, a descriptor value of an unmodeled type, or an
unknown comp reference SHALL NOT return an error, panic, or discard other
document data. A save SHALL preserve the original comp bytes when the comp model
could not interpret them.

#### Scenario: A truncated cmls does not fail the open [psdlc_malformed_cmls]

- **WHEN** a layer's `cmls` descriptor is truncated
- **THEN** the document opens, the layer is otherwise intact, and the block is preserved

#### Scenario: An unmodeled descriptor type is skipped [psdlc_unknown_type]

- **WHEN** a comp or cmls descriptor contains a value type the codec does not model
- **THEN** the descriptor is preserved for re-save and the open succeeds

#### Scenario: A failed parse does not lose the comp bytes [psdlc_preserve_failed]

- **WHEN** a document whose comp resource failed to parse is saved without comp edits
- **THEN** the original resource and per-layer comp blocks are re-emitted unchanged
