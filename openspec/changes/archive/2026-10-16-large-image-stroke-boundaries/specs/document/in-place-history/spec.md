## ADDED Requirements

### Requirement: Private current state

The history SHALL hold the state at its cursor in planes that the live document
does not share, and capturing a state SHALL leave the live document's planes
unshared, so that the next write to the live document changes it in place
instead of copying a whole plane. Capture SHALL find the changed 64×64 tiles by
comparing the live document with the private state across all cores, and SHALL
copy only those tiles into the private state.

#### Scenario: Capture leaves the live document unshared [iph_unshared_capture]

- **WHEN** a paint commit is captured and the live document is then written
- **THEN** no live plane is shared with the history, and the restored states
  remain byte-identical to the captured ones

### Requirement: In-place undo with restored damage

Undo, redo and jump SHALL bring the live document to the target state by
applying the stored tiles to it in place when the target is in the same
geometry segment, and SHALL report the document rectangle whose composite
changed, or the whole document when the change cannot be bounded (a geometry or
document-level change). The canvas SHALL refresh only the reported rectangle.

#### Scenario: Undoing a stroke reports its tiles [iph_undo_damage]

- **WHEN** a brush stroke is committed and undone
- **THEN** the live document equals the pre-stroke state byte for byte and the
  reported rectangle covers the stroke's composite tiles, not the document

#### Scenario: Crossing a geometry change reports the whole document [iph_geometry_damage]

- **WHEN** an undo crosses a resize, crop, or layer-structure change
- **THEN** the live document equals the target state exactly and the reported
  rectangle is the whole document

### Requirement: One materialized copy per history

The anchor of the geometry segment that holds the cursor SHALL keep only
metadata, its pixels being the private state with the segment's deltas reverted,
so a document's pixels are retained by the history once rather than twice.
Every other anchor SHALL keep its full state.

#### Scenario: Reaching the oldest state from the private copy [iph_hollow_anchor]

- **WHEN** several strokes are captured after an open and undone back to the
  oldest state, then redone
- **THEN** every state is byte-identical to its capture and the oldest state was
  rebuilt from the private copy, not from a second retained copy
