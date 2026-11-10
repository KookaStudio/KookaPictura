# document/auto-align-layers Specification

## Purpose
Aligns selected layers by matching their overlapping content, using the seven
CS6 Auto-Align projections, a locked-then-centre-most reference layer, and an
optional Lens Correction pass.

## Requirements

### Requirement: Auto-Align projections

`Edit > Auto-Align Layers` SHALL align the selected eligible layers by matching
overlapping content and SHALL support all seven CS6 projections: Auto,
Perspective, Cylindrical, Spherical, Scene Collage, Reposition Only, and Lens
Correction. Reposition Only SHALL produce a translation with no rotation or
scale. Perspective and Lens Correction SHALL allow a projective transform.
Scene Collage SHALL allow only a similarity transform so shapes are preserved.
Cylindrical and Spherical SHALL estimate the transform in their projected
space. Auto SHALL run Perspective and Cylindrical and keep the projection whose
overlap score is higher. A run SHALL apply the transforms to the non-reference
layers only.

#### Scenario: Reposition Only recovers a translation [auto_align_reposition]

- **WHEN** two layers share overlapping content and one is offset by a known
  integer translation, and the `auto_align` unit tests plan Reposition Only
- **THEN** the plan holds a pure translation, no rotation or scale, and applying
  it lines the content up within one pixel

#### Scenario: Perspective recovers a homography [auto_align_perspective]

- **WHEN** a synthetic layer is warped by a known homography and the
  `auto_align` unit tests plan Perspective
- **THEN** the estimated map sends the source corners to the known targets
  within a small pixel tolerance and the aligned overlap is better than the
  unaligned overlap

#### Scenario: Scene Collage preserves shape [auto_align_scene_collage]

- **WHEN** a synthetic layer is rotated and scaled and the `auto_align` unit
  tests plan Scene Collage
- **THEN** the plan's transform is a similarity (equal scale on both axes, no
  projective terms) and a circle in the moving layer stays a circle after
  alignment within tolerance

#### Scenario: Cylindrical and Spherical project wide fields [auto_align_cylindrical_spherical]

- **WHEN** two wide synthetic panoramas overlap and the `auto_align` unit tests
  plan Cylindrical and Spherical
- **THEN** both plans estimate a transform in the projected space, the overlap
  score improves over no alignment, and the resulting maps are finite over the
  source rect

#### Scenario: Auto keeps the better projection [auto_align_auto]

- **WHEN** the `auto_align` unit tests plan Auto for a synthetic pair
- **THEN** Auto evaluates Perspective and Cylindrical, returns one of them, and
  its overlap score is not worse than either candidate

### Requirement: Reference layer and eligibility

The reference layer SHALL be a locked layer when one is selected, otherwise the
eligible layer whose content centre is nearest the union of the selected
content. The reference SHALL not be transformed. Groups, adjustment layers,
fill layers, vector/shape layers, type layers, Smart Objects, clipped layers,
and the Background SHALL be skipped. A position-locked layer SHALL never move.
Auto-Align SHALL need at least two eligible layers with content. Every
invocation SHALL record one history state, and cancelling SHALL leave the
document unchanged.

#### Scenario: A locked layer is the reference [auto_align_reference]

- **WHEN** the `auto_align` unit tests select a position-locked layer and two
  ordinary layers with overlapping content
- **THEN** the locked layer's rect and pixels are unchanged and the other layers
  move to match it

#### Scenario: The centre-most layer is the reference [auto_align_centre_reference]

- **WHEN** no selected layer is locked and the `auto_align` unit tests plan an
  alignment of three layers
- **THEN** the reference is the eligible layer whose content centre is nearest
  the union centre

#### Scenario: Ineligible layers are skipped [auto_align_skips]

- **WHEN** the `auto_align` unit tests select a group, an adjustment layer, a
  Smart Object, a shape layer, and two raster layers
- **THEN** only the two raster layers are aligned, the others are unchanged, and
  a selection with fewer than two eligible layers reports that Auto-Align does
  not apply

#### Scenario: One history state and one undo [auto_align_one_state]

- **WHEN** the `tst_auto_align` test triggers Auto-Align Layers on two layers
  and then undoes
- **THEN** exactly one "Auto-Align Layers" state is added and one undo restores
  every layer's rect and pixels

#### Scenario: Cancelling leaves the document unchanged [auto_align_cancel]

- **WHEN** the `tst_auto_align` test starts a run, cancels it, and finishes
- **THEN** no history state is added and the document is bit-identical

### Requirement: Lens Correction

When Lens Correction is chosen, Auto-Align SHALL correct each eligible layer
for geometric distortion and, when requested, vignette removal before matching.
It SHALL use the installed lens profile that matches the document's EXIF camera
and lens. When no profile matches, or the document carries no usable EXIF, it
SHALL leave the pixels unchanged and report that the correction was skipped.

#### Scenario: A matching profile corrects before matching [auto_align_lens_correction]

- **WHEN** the `auto_align` unit tests pass a synthetic lens profile and request
  geometric distortion and vignette removal
- **THEN** the plan carries the correction, applying it changes the layer's
  corner pixels as the profile predicts, and matching runs on the corrected
  content

#### Scenario: A missing profile skips the correction [auto_align_lens_skipped]

- **WHEN** the `tst_auto_align` test runs Lens Correction on a document with no
  matching profile
- **THEN** the document's pixels are unchanged apart from the alignment, and the
  run reports that lens correction was skipped

### Requirement: Dialog, menu, and worker

`Edit > Auto-Align Layers…` SHALL be enabled when at least two eligible layers
are selected and no modal transform is active. The dialog SHALL offer the seven
projection radios and the Lens Correction sub-options Vignette Removal and
Geometric Distortion, enabled only for Lens Correction, and SHALL remember the
last choice for the session. The alignment SHALL run on a worker with progress
0–1000 and a Cancel button, and SHALL commit as one edit when it finishes.

#### Scenario: The menu enables with eligible layers [auto_align_menu]

- **WHEN** the `tst_auto_align` test selects one, two, and three eligible layers
  and opens the Edit menu
- **THEN** the command is disabled with one layer and enabled with two or more,
  and disabled when a modal transform is active

#### Scenario: The dialog defaults and restores [auto_align_dialog]

- **WHEN** the `tst_auto_align` test constructs the dialog, changes the
  projection and sub-options, and constructs another with the saved choice
- **THEN** Auto is the default, the Lens Correction sub-options are disabled
  unless Lens Correction is selected, and the saved choice is restored

#### Scenario: Progress and cancellation reach the worker [auto_align_progress]

- **WHEN** the `tst_auto_align` test starts a delayed run, observes progress and
  running state, and cancels
- **THEN** the run reports progress, stops after cancellation, and commits
  nothing

#### Scenario: A stale run is dropped [auto_align_stale]

- **WHEN** the `tst_auto_align` test starts a delayed run and edits the document
  before it finishes
- **THEN** the finished plan is discarded, no history state is added, and the
  run reports that the document changed
