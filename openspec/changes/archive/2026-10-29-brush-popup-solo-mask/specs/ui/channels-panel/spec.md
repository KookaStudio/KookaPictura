## ADDED Requirements

### Requirement: Mask thumbnail is bounded by its output

The Channels panel SHALL build the active layer's mask-row thumbnail in time
bounded by the thumbnail's own pixel count, not by the document's pixel count:
it SHALL sample the mask at the thumbnail grid rather than materialising a
document-sized plane and downsampling it. The thumbnail's pixels SHALL be
unchanged by this: each output pixel SHALL be the mask's coverage at that
pixel's nearest-neighbour source position over the document, using the same
off-rect `default_color` and data-less white as the mask plane, so the mask row
stays correct while it is refreshed after every edit.

#### Scenario: The thumbnail does not scan the document [cp_mask_thumb_bounded]

- **WHEN** the Channels panel builds the mask row's thumbnail for a large document
- **THEN** the mask is sampled once per thumbnail pixel, not once per document
  pixel

#### Scenario: The thumbnail pixels are unchanged [cp_mask_thumb_unchanged]

- **WHEN** the mask thumbnail is built for a document larger than the mask's own
  rectangle
- **THEN** every thumbnail pixel equals the nearest-neighbour sample of the
  mask's coverage over the document
