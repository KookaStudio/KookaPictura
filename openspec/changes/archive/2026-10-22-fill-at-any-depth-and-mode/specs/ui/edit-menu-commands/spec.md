# Spec Delta

## MODIFIED Requirements

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
document's layer takes the colour as an 8-bit document's does.

#### Scenario: Fill the selection

- **WHEN** Edit ▸ Fill is accepted with the foreground colour and an active
  selection
- **THEN** the selected pixels take the colour and one undo restores the layer

#### Scenario: Preserve Transparency confines the fill

- **WHEN** Preserve Transparency is set and the layer has transparent pixels
- **THEN** only the already-opaque pixels are filled

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
document, in the layer's pixels, exactly as `Edit ▸ Fill` does.

#### Scenario: Stroke outside the selection

- **WHEN** Edit ▸ Stroke is accepted 2 px Outside on a rectangular selection
- **THEN** the pixels just outside the selection take the colour and the
  selection interior is untouched

#### Scenario: Stroke outlines on a 16-bit document [em_stroke_depth16]

- **WHEN** Edit ▸ Stroke is accepted on a 16-bit document
- **THEN** the band takes the colour on the canvas and one `"Stroke"` state is
  recorded
