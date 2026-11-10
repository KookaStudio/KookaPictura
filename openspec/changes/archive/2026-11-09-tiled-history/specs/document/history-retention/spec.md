## MODIFIED Requirements

### Requirement: Region-delta undo retention

The history SHALL store the oldest state as a materialized base and each later
state as references to the 256² tiles whose bytes differ from the previous
state, for the layer channel planes, the document composite, and the layer mask
planes, plus the state's selection and non-pixel metadata. The retained history
memory SHALL scale with the edited area rather than with the document area. The
single materialized base MAY share unchanged tiles with the live document;
tiles that differ SHALL be held by reference, not copied. Smart-object filter
masks and native-depth `Samples` are not tracked planes and SHALL remain outside
this contract. The public `History` API (`capture`, `undo`, `redo`, `jump`,
`can_undo`/`can_redo`, the label/index/depth/count accessors, and the
named-snapshot accessors) and the 20-state and 10-snapshot bounds SHALL be
unchanged.

#### Scenario: Small edits retain small deltas [hr_bounded_retention]

- **WHEN** many small edits are captured on a large document
- **THEN** the retained delta bytes are bounded by the changed tile area and the
  history keeps a single materialized base, not one full state per edit

#### Scenario: A geometry change stores a full state [hr_geometry_anchor]

- **WHEN** a captured state changes the tracked-plane count, stride, or height
  (resize, crop, flatten, or a layer/mask structure change)
- **THEN** that state is stored as a full snapshot and becomes a new anchor, and
  tile deltas only span a stable geometry
