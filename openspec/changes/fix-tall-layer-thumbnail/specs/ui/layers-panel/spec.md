# Spec Delta

## MODIFIED Requirements

### Requirement: Layer thumbnail preserves the canvas aspect ratio

A layer row thumbnail SHALL be drawn at the document's aspect ratio inside the
row's thumbnail box, letterboxed rather than stretched into a square, so a wide
or tall document is not distorted. The thumbnail image itself SHALL also carry
the document's aspect ratio: a thumbnail built for Entire Document placement
SHALL size its output from the document aspect (`size` on the document's long
edge and the proportional length on the short edge), placing the layer's content
at its document-relative position, so a full-width layer has no transparent
padding strip and is not squashed. A shape layer's thumbnail SHALL likewise
carry the document aspect. The thumbnail outline, the transparency checkerboard,
the active bracket, and the thumbnail hit-target SHALL all use the letterboxed
rect. A group's folder glyph keeps its square slot.

#### Scenario: A non-square document is not stretched [lpr_thumb_aspect]

- **WHEN** a regular layer of a document whose width and height differ is shown
- **THEN** the drawn thumbnail's aspect ratio matches the document, with the
  unused part of the row's thumbnail box left empty

#### Scenario: A full-width layer has no transparent strip [lpr_thumb_source_aspect]

- **WHEN** a layer spanning the full width of a tall document is shown with
  Entire Document thumbnail contents
- **THEN** the thumbnail image has the document's aspect ratio and its rightmost
  column is layer content rather than transparent padding
