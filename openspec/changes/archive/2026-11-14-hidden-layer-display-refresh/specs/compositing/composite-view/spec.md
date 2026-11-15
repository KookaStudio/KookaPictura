## ADDED Requirements

### Requirement: A visibility edit refreshes every changed node

A visibility edit that sets exactly a listed set of paths visible and hides every
other node (solo, its restore, and Show All Layers) SHALL refresh the union of the
display reach of every node whose visibility changes, not the reach of the listed
paths. When a changed node's reach cannot be bounded, the edit SHALL fall back to
a full recomposite. The present at every cached pyramid level MUST equal a full
recomposite of the document after the edit, and a lazily opened file's merged
preview MUST be dropped before the refresh.

#### Scenario: Solo hides the unlisted nodes everywhere [cv_solo_hide]

- **WHEN** Alt-click solo keeps a small ordinary layer visible and hides the
  Background and every other layer
- **THEN** the canvas at the selected present level equals a full recomposite,
  including the pixels outside the soloed layer where the Background was

#### Scenario: Restoring the solo snapshot shows the hidden nodes everywhere [cv_solo_restore]

- **WHEN** the solo snapshot is restored after an Alt-click solo
- **THEN** the canvas at the selected present level equals a full recomposite
  with the previously hidden nodes visible again

#### Scenario: Hiding at fit zoom updates the selected level [cv_hidden_at_fit]

- **WHEN** a layer, including the Background, is hidden while the canvas presents
  a reduced pyramid level at fit zoom
- **THEN** the presented crop equals a full recomposite at that level, and a
  lazily opened file no longer shows the merged preview's layer
