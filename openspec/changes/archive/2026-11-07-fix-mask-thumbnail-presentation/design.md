# Design

## Context

A generated 120×400 PSD reproduces the right-edge checkerboard strip in the
current application. The Rust preview is 7×24 opaque grayscale. The row's taller
slot yields a 10×32 document card; fitting the already rounded preview again
leaves a column uncovered. Pixel inspection of a captured production window
finds alternating 200/255 checkerboard pixels in that column beside coverage 128.

The delegate also paints a separate white rectangle from cached MaskTargetRole,
independently of the existing selection-aware ActiveThumbRole brackets. Clearing
the actual edit target does not necessarily rebuild that cached row role.

## Goals / Non-Goals

Fix opaque mask-card rendering and the requested selection appearance. Preserve
square-card fitting, raster/color thumbnail options, mask data and modifier clicks.

## Decisions

Draw the opaque mask image directly into its document-aspect card, whose geometry
already represents the document. Keep aspect fitting for square cards. Draw the
standard black card outline and retain the existing white brackets as the only
white target indicator; remove the independent cached-target outline.

Test PSD opening through the existing generated-fixture seam, paint actual rows
at multiple thumbnail sizes and display scales, and exercise real thumbnail clicks.
Retain coverage of mask/vector links, disabled crosses and solo modifiers.

## Risks / Trade-offs

Integer rounding means the source image and destination card can differ by a
pixel in aspect. Mapping the two document grids directly avoids uncovered edges;
square cards keep their separate fitting behavior. The original PSD was requested
but the defect is independently reproduced with a generated fixture.
