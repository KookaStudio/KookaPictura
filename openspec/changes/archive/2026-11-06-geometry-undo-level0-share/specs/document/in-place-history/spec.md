## ADDED Requirements

### Requirement: A segment break keeps an unchanged composite

When a capture starts a new geometry segment because tracked planes moved, the
history SHALL compare the live document's composite with the current state's
across all cores, stopping at the first differing band. When they are
byte-identical, the live document SHALL take the current state's composite plane
as its own, so the two share it. The capture MUST NOT keep a second full-plane
copy of an unchanged composite. An undo, redo or jump that crosses such a state
SHALL report an empty composite rectangle without comparing any composite
pixels.

#### Scenario: A reorder that leaves the picture unchanged shares the composite [iph_segment_keeps_composite]

- **WHEN** a layer reorder that leaves the composite byte-identical is captured
  and then undone
- **THEN** the live document's composite plane is the captured state's, and the
  undo reports an empty rectangle without scanning the composite

#### Scenario: A reorder that changes the picture keeps its own composite [iph_segment_changed_composite]

- **WHEN** a reorder that changes the composite is captured and then undone
- **THEN** the undo reports the changed composite tiles and the live document
  matches the target state byte for byte
