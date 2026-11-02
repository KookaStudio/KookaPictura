# document/layer-duplicate-responsiveness Specification

## Purpose
Keeps Duplicate Layer and undo responsive on very large documents: plane reads
and content-bounds scans stay independent of the band count, and an edit that
cannot change the picture does not pay for a composite or a Move preview bake.

## Requirements

### Requirement: Plane reads do not scale with the band count

A contiguous read of a fractured plane SHALL decide whether any band has
diverged without visiting the bands. The decision SHALL stay correct across a
band write, a clone, and a re-fracture.

#### Scenario: Divergence is tracked across clones and fractures [ldr_diverged]

- **WHEN** a band of a fractured plane is written, the plane is cloned, and the plane is later fractured at another width
- **THEN** the clone reads the written bytes
- **AND** the re-fractured plane reads the same bytes through its base allocation

### Requirement: Content bounds scan rows

The bounds of a layer's visible pixels SHALL be computed from the rows of its
transparency plane and SHALL NOT materialize a plane whose bands have diverged.
The result SHALL equal a per-pixel scan.

#### Scenario: Bounds match a per-pixel scan [ldr_bounds_exact]

- **WHEN** the content rect of a layer with an arbitrary alpha pattern is computed
- **THEN** it equals the box found by testing every pixel

#### Scenario: A diverged plane is not materialized [ldr_bounds_no_materialize]

- **WHEN** the content rect of a layer whose alpha plane has a forked band is computed
- **THEN** the materialization counter does not change

### Requirement: A duplicate that cannot change the picture does not recomposite

Duplicating one layer SHALL keep the stored composite when the copy sits
directly above an 8-bit RGB source that is visible, Normal, at full opacity and
fill, fully opaque, unclipped, and carries nothing that renders around it, with
no clipped layer directly above the copy and no knockout in the document. Any
other duplicate SHALL recomposite.

#### Scenario: A neutral duplicate preserves the composite [ldr_neutral_exact]

- **WHEN** a layer the rule accepts is duplicated, at the root or inside a group, including a copy of a Background copy
- **THEN** a composite of the document is byte-identical to the composite before

#### Scenario: Every exclusion recomposites [ldr_excluded]

- **WHEN** the source breaks any one condition of the rule, or several layers are duplicated
- **THEN** the duplicate is not reported as content-neutral

#### Scenario: The edit skips the display rebuild [ldr_edit_inserted]

- **WHEN** a neutral duplicate is reported to the edit transaction
- **THEN** the outcome is an inserted edit, and a non-neutral duplicate keeps its structural outcome

### Requirement: Undo does not rebuild the Move preview

Restoring a history state SHALL NOT rebuild the Move tool's preview cache. Only
switching to the Move tool SHALL warm it, and a press SHALL rebuild a stale one.

#### Scenario: Undo leaves the warm alone [ldr_undo_no_warm]

- **WHEN** the Move tool is active on a movable layer and an edit is undone
- **THEN** the preview cache is not rebuilt, and the next press rebuilds it

### Requirement: Releasing a Background gives it an opaque alpha plane

A copy of a Background, and a Background converted to a layer, SHALL carry an
alpha plane of 255 for every pixel of its rect.

#### Scenario: The released layer is opaque [ldr_release_alpha]

- **WHEN** a Background is duplicated
- **THEN** the copy has an alpha plane the size of its rect holding 255 throughout
