# imaging/liquify-history Specification

## Purpose
Undo and redo of individual Liquify actions inside the dialog, before OK.

## ADDED Requirements

### Requirement: Liquify keeps a bounded per-action undo history

The Liquify session SHALL keep an undo history of mesh snapshots covering the
distortion and the freeze mask, one snapshot per brush stroke and one per
whole-mesh action (Reconstruct, Restore All, Mask None, Mask All, Invert All,
and Load Last Mesh). The history SHALL be bounded to the most recent actions.
A new action after an undo SHALL discard the redo history. The history SHALL
NOT create a document history state; OK SHALL still commit exactly one filter
state and Cancel SHALL discard the session and its history.

#### Scenario: Strokes undo and redo one at a time [lq_undo_stroke]

- **WHEN** two Forward Warp strokes A and B are drawn and Ctrl+Z is pressed
- **THEN** B is removed and A remains; Ctrl+Shift+Z restores B; a new stroke
  after an undo discards the redo path

#### Scenario: The freeze mask is part of the undo step [lq_undo_mask]

- **WHEN** Mask All, Invert All, or a Freeze Mask stroke is undone
- **THEN** the freeze mask returns to its state before that action

#### Scenario: Empty undo and redo do nothing [lq_undo_empty]

- **WHEN** Ctrl+Z or Ctrl+Shift+Z is pressed with no action to undo or redo
- **THEN** the mesh, preview, and document are unchanged, and a later OK still
  records one filter history state

#### Scenario: The undo history is bounded [lq_undo_bounded]

- **WHEN** more actions are recorded than the history limit
- **THEN** only the most recent actions can be undone
