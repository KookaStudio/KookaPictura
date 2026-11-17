## MODIFIED Requirements

### Requirement: A document canvas holds no full-resolution image

A canvas presenting a document SHALL know the document by its size and present
it from tile-pyramid crops alone, holding no full-resolution image; a region
the view repaints SHALL only invalidate the crops. The canvas SHALL still yield
the document as one image on request, cropped whole from level 0, and its
single-channel view SHALL be built from the presented crop.

#### Scenario: The canvas keeps no copy of the document [cfr_no_image]

- **WHEN** the `tst_large_document` suite paints on a document
- **THEN** the canvas reports the document's size, holds no image, and the image
  it yields on request shows the painted pixels
