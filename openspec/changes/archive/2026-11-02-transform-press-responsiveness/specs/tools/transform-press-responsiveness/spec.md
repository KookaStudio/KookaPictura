## ADDED Requirements

### Requirement: Starting a transform stays layer-sized

Starting a transform on a layer whose appearance stays inside its bounds SHALL
composite and convert only those bounds. Layer bounds SHALL limit the moving image
of a rendered layer, such as type or a masked layer, and the backdrop SHALL be a
patch of those bounds with the target hidden, drawn over the live canvas and
replacing what the canvas shows there, transparency included. The preview SHALL
show the same pixels as a backdrop of the whole document. A layer with effects,
an adjustment, or children, and a lifted selection, SHALL keep the whole-document
backdrop.

#### Scenario: The moving image equals the full render [tpr_layer_region]

- **WHEN** a masked layer inside a larger document is prepared for a transform
- **THEN** its moving image and origin match the same bounds cropped from a render
  of the whole document

#### Scenario: The backdrop patch equals the full backdrop [tpr_patch_exact]

- **WHEN** a small layer over another layer is prepared for a transform
- **THEN** the backdrop is a patch of the layer's bounds at the layer's origin, and
  its pixels match that area of the whole document composited with the layer hidden

#### Scenario: Effects and lifts keep the whole backdrop [tpr_full_backdrop]

- **WHEN** a layer with effects is prepared, or the session is a lifted selection
- **THEN** the backdrop covers the whole document

#### Scenario: The patch hides the layer on the canvas [tpr_canvas_patch]

- **WHEN** a handle of a small layer over transparency is pressed and dragged away
  from where the layer was
- **THEN** the layer's old place shows the transparency checkerboard, the scaled
  layer is drawn, and the rest of the canvas is unchanged

### Requirement: Reading one composite pixel stays one read

Reading a pixel of the document composite SHALL read that pixel from its band and
SHALL NOT materialize the composite, so a read after a region edit costs the same
on any document size.

#### Scenario: A read leaves a diverged composite banded [tpr_sample_banded]

- **WHEN** a pixel of a composite with a forked band is read
- **THEN** the read returns the forked band's value and nothing materializes

### Requirement: A preview leaves the canvas as it was

A Move drag or transform of a layer with a backdrop patch SHALL keep drawing the
live canvas, so the document outside the patch and the moving layer keeps its
pixels and quality at every zoom. A whole-document backdrop SHALL be sampled as
the canvas is.

#### Scenario: A drag keeps the pixels outside the layer [tpr_canvas_kept]

- **WHEN** a Move drag and a handle drag run on a small layer over a detailed
  document at 58 % zoom
- **THEN** the backdrop is a patch of the layer's bounds, and the canvas outside
  the layer and the pointer is identical to the canvas before the press
