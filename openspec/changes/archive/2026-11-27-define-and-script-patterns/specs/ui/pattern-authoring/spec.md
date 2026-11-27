# Pattern authoring

## Purpose

Create reusable patterns from image selections and apply patterned fills with geometric layouts.

## ADDED Requirements

### Requirement: Define a reusable pattern

Edit > Define Pattern SHALL accept a nonempty, hard rectangular selection in an open document, ask for a name, and append its selected image pixels as a named pattern to the application-wide current list. Cancellation, invalid selections and persistence failure SHALL leave the list and document unchanged. Definition SHALL NOT add document history. Defined patterns SHALL survive restart and be available through supported pattern consumers, including Paint Bucket, Pattern Stamp, Pattern Fill layers and shape fills. Pattern identity SHALL use the existing content-derived document pattern ID.

#### Scenario: Define and reuse [pattern_define]

- **WHEN** a user defines a named pattern from a hard rectangular selection and uses it in a pattern consumer after restart
- **THEN** the chosen tile matches the selected pixels and no history state was created by definition

#### Scenario: Reject invalid selection or cancel [pattern_define_cancel]

- **WHEN** the selection is empty, nonrectangular, feathered, or the name dialog is cancelled
- **THEN** no pattern is added and document pixels and history remain unchanged

#### Scenario: Reuse in pattern consumers [pattern_consumers]

- **WHEN** a user defines a pattern and selects it in Paint Bucket, Pattern Stamp, Pattern Fill and shape fill
- **THEN** each consumer uses the defined tile, and the layer and shape fills embed a resolvable pattern

#### Scenario: Persistence failure or stale dialog [pattern_define_failure]

- **WHEN** preset persistence fails, or the active document or selected region changes while the name dialog is open
- **THEN** no pattern is appended and document pixels and history remain unchanged

### Requirement: Pattern selection in Edit Fill

Edit > Fill SHALL offer the current application-wide pattern list, including imported and defined entries, and SHALL retain the selected tile for an applied fill even if the list subsequently changes. Ordinary pattern fill SHALL preserve selection, blending, opacity, transparency and undo semantics. New recorded pattern fills SHALL include a portable tile snapshot and script choice for replay independent of subsequent library changes; legacy index-only recorded fills SHALL remain replayable.

#### Scenario: Fill with a current pattern [pattern_fill_preset]

- **WHEN** a user selects a defined or imported tile and accepts an ordinary pattern fill within a selection
- **THEN** the selected tile repeats in the selection, undo restores the original pixels, and later library edits do not rewrite the result

#### Scenario: Empty current list [pattern_fill_empty]

- **WHEN** the current pattern list is empty
- **THEN** pattern fill and scripted controls cannot be applied and no document change is made

#### Scenario: Replay a pattern fill [pattern_fill_replay]

- **WHEN** ordinary and scripted pattern fills are recorded and the source preset is renamed or removed before replay, or a legacy index-only fill is replayed
- **THEN** the new recordings reproduce their recorded tile and script, and the legacy fill resolves its original built-in index

### Requirement: Scripted pattern layouts

The Fill dialog SHALL offer Scripted Patterns with Brick Fill, Cross Weave, Random Fill, Spiral and Symmetry Fill when a pattern is available. These geometric layouts SHALL be deterministic approximations of the named effects, with fixed seed and bounded work. Brick SHALL stagger alternating rows, Cross Weave SHALL alternate perpendicular orientations, Random SHALL scatter placements, Spiral SHALL progress outward along a spiral, and Symmetry SHALL mirror placements about a center. Operations exceeding the admitted work or allocation bounds SHALL be refused without a partial edit. A scripted fill SHALL honor selection, blend mode, opacity, preserve transparency, and one-step undo. Cancelling SHALL make no change.

#### Scenario: Distinct reproducible layouts [pattern_scripts]

- **WHEN** a suitable asymmetric tile and sufficiently large destination are filled twice with each of the five scripts
- **THEN** each script reproduces its own result and exhibits its specified geometry

#### Scenario: Small and excessive scripted fills [pattern_script_bounds]

- **WHEN** a script fills a small region or its requested work exceeds the admitted bounds
- **THEN** the small region completes deterministically, and an excessive operation is refused without a partial edit or history state

#### Scenario: Scripted fill respects normal editing [pattern_script_edit]

- **WHEN** a script is applied with a selection, opacity and preserve transparency and then undone
- **THEN** only eligible pixels change under those settings, and undo restores the prior image

#### Scenario: Cancel scripted fill [pattern_script_cancel]

- **WHEN** a scripted fill dialog is cancelled
- **THEN** pixels and history remain unchanged
