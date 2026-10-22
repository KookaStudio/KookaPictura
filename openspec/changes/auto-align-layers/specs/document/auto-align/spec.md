# Spec Delta

## Purpose

Content-based registration of selected raster layers: estimate the transform that makes overlapping content line up, then apply it as one undoable layer edit.

## ADDED Requirements

### Requirement: Auto-Align Layers command

`Edit > Auto-Align Layers…` SHALL open a dialog that offers `Auto`, `Reposition Only`, `Perspective`, and `Scene Collage`, with `Auto` selected by default. `Cylindrical`, `Spherical`, and the `Lens Correction` option SHALL be shown as unavailable with an explicit reason and SHALL be refused without mutating the document if requested directly. The command SHALL be enabled only when at least two eligible layers are selected.

Eligible layers are raster layers with non-empty content that can be transformed. Groups, adjustment layers, fill/content layers, vector/type layers, Smart Objects, and the Background SHALL be skipped and SHALL NOT count toward the two-layer minimum. The Background MAY be the reference but SHALL never move. A position-locked non-reference layer SHALL be skipped rather than moved.

A locked layer among the selected layers SHALL be the reference and SHALL keep its transform. Without a locked layer the reference SHALL be the eligible layer whose content centre is nearest the centre of the final composition (the document canvas). Every other eligible layer SHALL be transformed to align with the reference.

The application SHALL compute every placement first and apply them to a candidate document, committing only when the whole registration succeeds. Accepting SHALL record exactly one `Auto-Align Layers` history state. Cancelling the dialog or the running registration SHALL leave every layer unchanged and record no state.

#### Scenario: Reposition Only aligns a known translation

- **WHEN** two eligible layers have the same content offset by a known translation and Reposition Only runs
- **THEN** the non-reference layer is translated by that offset within one pixel, introduces no rotation or scale, and the reference layer is unchanged

#### Scenario: A locked layer is the reference

- **WHEN** a locked layer is selected with other eligible layers
- **THEN** the locked layer's transform is unchanged and the other layers move to align with it

#### Scenario: The composition centre picks the reference without a lock

- **WHEN** no selected layer is locked and the layers are spread across the canvas
- **THEN** the reference is the eligible layer nearest the document centre, not merely the first in the layer stack

#### Scenario: Perspective aligns a synthetic quadrilateral

- **WHEN** two layers show the same content under a known projective warp and Perspective runs
- **THEN** the non-reference layer's transformed content corners match the reference within two pixels

#### Scenario: Scene Collage preserves shape

- **WHEN** Scene Collage runs on a rotated and translated copy of a rectangular layer
- **THEN** the applied map is a similarity with no shear or perspective terms and the rectangle remains a rectangle

#### Scenario: Auto is a reduced supported-mode heuristic

- **WHEN** Auto runs on a pair that Reposition Only cannot align well and Perspective can
- **THEN** Auto's residual is no worse than Reposition Only's and it uses one of the supported modes; it does not claim CS6's Perspective/Cylindrical choice

#### Scenario: Unsupported modes are refused

- **WHEN** Cylindrical, Spherical, or Lens Correction is requested
- **THEN** the call returns an unsupported error and no layer is modified

#### Scenario: Ineligible layers are skipped

- **WHEN** the selection contains only one eligible raster layer plus groups, adjustment layers, fill layers, vector/type layers, Smart Objects, or the Background
- **THEN** the command is disabled and no registration runs

#### Scenario: Cancel records nothing

- **WHEN** the user cancels the running registration before commit
- **THEN** no layer transform changes and no history state is added

#### Scenario: A mid-apply refusal leaves the stack unchanged

- **WHEN** one computed placement is refused at apply time
- **THEN** no layer changes are committed and no history state is added

### Requirement: Registration accuracy and determinism

For a pair of layers with overlapping content, the registration SHALL return a deterministic transform for the same inputs and parameters. A repeated run with unchanged inputs SHALL produce bit-identical layer content and transforms. The reference layer SHALL remain byte-identical.

#### Scenario: Repeated runs are identical

- **WHEN** the same Auto-Align invocation is run twice on unchanged layers
- **THEN** the resulting layer pixels and transforms are identical

#### Scenario: One history state and undo

- **WHEN** Auto-Align completes on two or more layers and the edit is undone
- **THEN** exactly one `Auto-Align Layers` state was recorded and undo restores every affected layer to its pre-alignment state
