# Spec Delta

## MODIFIED Requirements

### Requirement: Type layers are identified by a preserved TySh block

An imported layer carrying a preserved `TySh` block SHALL supply a Text
candidate to the common content normalizer even when the block cannot be
parsed. With no higher-priority content candidate, its active content SHALL be
Text, and `layer_kind`, `layer_row_kind`, and the shared row-kind projection
SHALL return the string `type`. Neither a malformed text payload nor a raster
proxy SHALL change that identity. A higher-priority active content selected by
the common normalizer SHALL determine the row facts instead of a separate
`TySh` precedence chain. The established bridge labels for non-conflicting
layers, including `background` for a flagged pixel layer, SHALL be preserved.
The `TySh` bytes SHALL remain available in preservation storage and SHALL
re-emit byte-for-byte on an unchanged open-save. The existing type-lock import
rules SHALL remain unchanged.

#### Scenario: A layer with TySh reports type

- **WHEN** a non-group layer is imported with `TySh` and no higher-priority content candidate, including a malformed `TySh` or a pixel proxy
- **THEN** its active identity is Text and both row-kind projections return `type`

#### Scenario: A pixel layer without TySh is still pixel

- **WHEN** an ordinary layer has no special content and no `TySh` block
- **THEN** its kind is `pixel`, or `background` when flagged background

#### Scenario: TySh bytes round-trip

- **WHEN** a document whose layer carries a `TySh` block is written and read back without a content edit
- **THEN** the layer retains the same `TySh` key and payload bytes

#### Scenario: Conflicting type bytes do not override active content [lc_type_conflict]

- **WHEN** imported data contains a `TySh` block and a higher-priority group or adjustment candidate
- **THEN** the normalized active content determines the row facts, and the original `TySh` bytes remain preserved
