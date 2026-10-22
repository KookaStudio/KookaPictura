# ui/edit-menu-commands Specification

## Purpose
Wires the Edit ▸ Fill / Stroke / Purge commands and the Image ▸ Trim /
Duplicate commands onto the shipped engine, each recording exactly one history
state.

## Requirements

### Requirement: Edit Fill fills the active pixel layer

`Edit ▸ Fill…` SHALL open a dialog offering the foreground colour or a built-in
pattern, a blend mode, an opacity (0–100 %), and Preserve Transparency. On
accept it SHALL fill the active visible pixel layer within the selection (or
the whole layer without one) through the paint engine and record exactly one
`"Fill"` state. It SHALL refuse, recording no state, without a document, without
a lone visible active pixel layer, when the layer's pixels are locked, or when
the fill changes nothing. It SHALL fill in any document colour mode: a
Grayscale document's layer takes the colour's luma, and a CMYK or Lab
document's layer takes the colour on its working copy. In an 8-bit, 16-bit or
32-bit document it SHALL fill the layer's pixels, so that a 16- or 32-bit
document's layer takes the colour as an 8-bit document's does. Preserve
Transparency SHALL keep every filled pixel's alpha as it was and SHALL leave
the layer's empty pixels alone.

#### Scenario: Fill the selection

- **WHEN** Edit ▸ Fill is accepted with the foreground colour and an active
  selection
- **THEN** the selected pixels take the colour and one undo restores the layer

#### Scenario: Preserve Transparency confines the fill

- **WHEN** Preserve Transparency is set and the layer has transparent pixels
- **THEN** only the already-opaque pixels are filled, the pixels that were
  partly transparent keep their alpha, and the empty pixels are not filled

#### Scenario: Fill fills a 16-bit document [em_fill_depth16]

- **WHEN** Edit ▸ Fill is accepted on a 16-bit document
- **THEN** the layer's pixels take the colour on the canvas and one `"Fill"`
  state is recorded

#### Scenario: Fill fills a Grayscale document [em_fill_gray]

- **WHEN** Edit ▸ Fill is accepted on a Grayscale document
- **THEN** the layer's pixels take the colour's luma and one `"Fill"` state is
  recorded

### Requirement: Edit Stroke outlines the selection

`Edit ▸ Stroke…` SHALL open a dialog offering a width, colour, position
(Inside / Center / Outside), blend mode, and opacity. On accept, with an active
pixel selection on a visible pixel layer, it SHALL lay a solid band along the
selection edge on the chosen side and record exactly one `"Stroke"` state. It
SHALL refuse, recording no state, without a selection or a valid target layer.
It SHALL lay the band in any document colour mode and, in an 8-bit or 16-bit
document, in the layer's pixels, exactly as `Edit ▸ Fill` does. The band SHALL
reach no further than the chosen width from the selection edge, and SHALL not
reach past the selection's edge into the rest of the layer.

#### Scenario: Stroke outside the selection

- **WHEN** Edit ▸ Stroke is accepted 2 px Outside on a rectangular selection
- **THEN** the pixels just outside the selection take the colour and the
  selection interior is untouched

#### Scenario: The band stops at the selection's edge [em_stroke_band]

- **WHEN** Edit ▸ Stroke is accepted 2 px Outside on a rectangular selection
  with room to its right
- **THEN** the pixels beyond the 2 px band keep the layer's own colour

#### Scenario: Stroke outlines on a 16-bit document [em_stroke_depth16]

- **WHEN** Edit ▸ Stroke is accepted on a 16-bit document
- **THEN** the band takes the colour on the canvas and one `"Stroke"` state is
  recorded

### Requirement: Edit Purge drops undo history and restore points

`Edit ▸ Purge ▸ Undo / Histories / All` SHALL each be enabled with a document
and operate on the active document's history: Undo SHALL drop the undo states
but keep the named restore points, Histories SHALL drop the restore points but
keep the undo states, and All SHALL drop both and clear the app's own clipboard
export. Each SHALL leave a single current state and record no history state.

#### Scenario: Purge Undo keeps restore points

- **WHEN** Purge ▸ Undo runs after a restore point was added
- **THEN** the undo stack collapses to the current state and the restore point
  remains

### Requirement: Image Trim crops to the content bounds

`Image ▸ Trim…` SHALL crop the canvas to the bounding box of the composite's
visible pixels and record exactly one `"Trim"` state. It SHALL refuse, recording
no state, when the content already fills the canvas, when the composite is
empty, or without a document.

#### Scenario: Trim removes a transparent border

- **WHEN** Image ▸ Trim runs on a document whose composite has a transparent
  border
- **THEN** the canvas takes the content bounds and one undo restores the
  original size

### Requirement: Image Duplicate copies the document to a new tab

`Image ▸ Duplicate…` SHALL prompt for a name and whether to duplicate the merged
layers only, then open a new tab holding a copy of the active document and
record exactly one `"Duplicate"` state on that tab. The suggested name SHALL be
the source file's base name plus `" copy"`, or `"Untitled copy"` when untitled.
It SHALL refuse without a source document.

#### Scenario: Duplicate keeps the layer stack

- **WHEN** Image ▸ Duplicate is accepted with "Duplicate Merged Layers Only"
  unchecked
- **THEN** the new tab holds the same layer stack as the source and its own
  single history anchor
