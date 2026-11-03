## MODIFIED Requirements

### Requirement: In-place undo with restored damage

Undo, redo and jump SHALL bring the live document to the target state by
applying the stored tiles to it in place when the target is in the same
geometry segment, and SHALL report the document rectangle whose composite
changed, or the whole document when the change cannot be bounded (a change of
document size, mode, depth or profile). The canvas SHALL refresh only the
reported rectangle.

#### Scenario: Undoing a stroke reports its tiles [iph_undo_damage]

- **WHEN** a brush stroke is committed and undone
- **THEN** the live document equals the pre-stroke state byte for byte and the
  reported rectangle covers the stroke's composite tiles, not the document

#### Scenario: Crossing a geometry change reports the whole document [iph_geometry_damage]

- **WHEN** an undo crosses a resize or crop
- **THEN** the live document equals the target state exactly and the reported
  rectangle is the whole document

#### Scenario: Crossing a layer-structure change reports what changed [iph_structure_damage]

- **WHEN** an undo crosses a layer-structure change at the same document size
- **THEN** the live document equals the target state exactly and the reported
  rectangle covers the changed composite tiles, empty when none changed
