## ADDED Requirements

### Requirement: Selection consumers read tiles or rows

Every consumer of a selection's coverage SHALL read it through the plane's
tile or row access: the clipboard (Copy, Copy Merged, Cut, Clear, Paste Into,
Paste Outside), Layer via Copy and via Cut, Lift for a transform, moving a
selection inside a layer mask, Patch, Content-Aware Move, filters and
adjustments limited to a selection (the blend back and the Auto adjustments'
coverage), Apply Image and smart filters masked by the selection, Fit
Centered, and the application's selection reads. Refine Edge, whose
whole-canvas work arrays dwarf the selection, is excluded. None of them SHALL materialize a tiled
selection into one contiguous copy, and each SHALL give the same bytes as the
dense path it replaces.

#### Scenario: Clipboard reads a tiled selection [tsr_clipboard]

- **WHEN** Copy, Copy Merged, Clear and Paste Into run with a tiled soft
  selection
- **THEN** no plane is materialized, and every clip, layer and mask equals the
  result for the same selection held flat

#### Scenario: Lift, via and mask moves read a tiled selection [tsr_layer_ops]

- **WHEN** Lift, Layer via Copy, Layer via Cut and a selection move inside a
  layer mask run with a tiled selection
- **THEN** no plane is materialized, and the document equals the result for
  the same selection held flat

#### Scenario: Healing reads a tiled selection [tsr_healing]

- **WHEN** Patch (source and destination) and Content-Aware Move run with a
  tiled selection
- **THEN** no selection is materialized, and the layer equals the result for
  the same selection held flat

#### Scenario: Filters and Auto adjustments limited to a selection [tsr_filter]

- **WHEN** a filter and Auto Tone run on a layer through a tiled soft
  selection
- **THEN** no plane is materialized, and the layer equals the result for the
  same selection held flat

#### Scenario: Fit Centered reads tiles [tsr_select]

- **WHEN** a tiled selection is fitted into a larger and a smaller canvas
- **THEN** no plane is materialized, each result is tiled, and it equals the
  result for the same selection held flat

#### Scenario: The existing tests still pass [tsr_parity]

- **WHEN** the existing clipboard, mask-edit, healing, filter and adjustment
  tests run
- **THEN** they pass with their assertions unchanged

### Requirement: Whole-selection exports keep one output buffer

A bridge function that hands a whole selection to the shell SHALL build its
one output buffer by reading the selection's tiles, and SHALL NOT leave a
cached contiguous copy of the selection behind.

#### Scenario: Coverage export leaves no mirror [tsr_exports]

- **WHEN** the live selection's coverage and an alpha channel's coverage are
  exported, and a colour row's coverage is exported
- **THEN** the selection and channel exports equal the source's bytes without
  materializing a plane, the colour row export covers the document, and the
  selection and the channel are still tiled afterwards

### Requirement: Selection writes keep the selection tiled

Edits that write a selection's coverage in place SHALL write through the
plane's row or rectangle writers, so the selection stays tiled.

#### Scenario: An in-place selection write stays tiled [tsr_writers]

- **WHEN** a tiled selection is dragged by its outline and the drag is
  committed
- **THEN** it is still tiled, no plane is materialized, and its bytes equal
  the dense shift of the origin

### Requirement: The heap guard counts zero selection materializations

A heap-guard test SHALL run one operation from each migrated group on a tiled
selection under nextest and SHALL assert that the materialization counter did
not move.

#### Scenario: The guard reports zero [tsr_guard]

- **WHEN** the selection-readers guard runs under nextest
- **THEN** Copy, Clear, Lift, a filter through a selection, Patch, a coverage
  export and an in-place move report 0 materializations
