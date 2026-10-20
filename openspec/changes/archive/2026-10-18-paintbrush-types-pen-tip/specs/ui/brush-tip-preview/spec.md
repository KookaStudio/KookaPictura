# ui/brush-tip-preview Specification

## Purpose
The on-canvas pen-tip model preview shown while a pencil brush is selected.

## ADDED Requirements

### Requirement: Pen-tip preview overlay

While the Brush or Pencil tool is active and the current brush tip's preview
model is the pen nib, the active document's canvas SHALL show a 2D pen-tip
preview at its top-left: a gray panel with a dashed ground line, a light
barrel, and a black nib touching the line. The overlay SHALL be hidden for any
other tip, any other tool, and when no document is open. The overlay SHALL be
draggable within the canvas and SHALL NOT start a paint stroke. The preview
SHALL be a painted 2D approximation, not a 3D render.

#### Scenario: A pencil preset shows the pen tip

- **WHEN** the Brush tool is active and `2B Pencil` is chosen
- **THEN** the active canvas shows the pen-tip overlay at its top-left

#### Scenario: A non-pencil preset hides the pen tip

- **WHEN** `Soft Round 5` is chosen
- **THEN** the pen-tip overlay is hidden

#### Scenario: Another tool hides the pen tip

- **WHEN** the Move tool is selected while `2B Pencil` is current
- **THEN** the overlay is hidden; switching back to the Brush shows it

#### Scenario: The overlay drags without painting

- **WHEN** the overlay is dragged 20 px right and down, then far past the
  canvas edge
- **THEN** it moves by that amount, stays inside the canvas, and the document
  pixels are unchanged

#### Scenario: The preview follows the active document

- **WHEN** two documents are open with a pencil preset current and the second
  document is activated
- **THEN** only the active document's canvas shows the overlay
