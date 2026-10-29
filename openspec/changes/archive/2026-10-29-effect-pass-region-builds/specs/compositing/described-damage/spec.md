# Spec Delta

## ADDED Requirements

### Requirement: A paint refresh covers the effect and feather reach

A paint stroke's present, commit, and cancel rectangle SHALL be its changed
pixels grown by the painted layer's paint-change reach (outer and inner effects)
and, for a mask stroke, the mask feather reach. This refines "the stroke's
changed rectangle" in the bounded pixel-operation requirement: the halo and
feather ramp the stroke changes are part of its changed rectangle. The PRESENT
rectangle MUST stay incremental: it MUST NOT grow with earlier dabs of the same
stroke. The COMMIT and CANCEL refresh the pending present for a stroke that
presented as it went, or the union of the stroke's dabs for a previewed or
cancelled stroke; the history hint SHALL cover the union. All three rectangles
MUST be clipped to the canvas. A
reduced-resolution preview present is exempt: it carries the level-space extent,
and the exact commit repairs the composite. The stored composite after a commit
or cancel MUST equal a full recomposite of the document, and every pixel outside
the grown rectangle MUST keep its previous value.

#### Scenario: A styled-layer dab refreshes the halo

- **WHEN** a dab is painted on a layer carrying a drop shadow, outer glow, inner
  shadow, inner glow, stroke, satin, or bevel and the dab is committed
- **THEN** the refreshed rectangle includes the effect's reach, the halo moves
  with the dab, and `doc.composite` equals a full recomposite

#### Scenario: A mask dab refreshes the feather ramp

- **WHEN** a dab is painted on a feathered raster mask and the dab is committed
- **THEN** the refreshed rectangle includes the feather reach, the ramp outside
  the dab is refreshed, and `doc.composite` equals a full recomposite

#### Scenario: A cancelled styled stroke restores the halo

- **WHEN** a stroke on a styled layer or a feathered mask is cancelled
- **THEN** the cancel refreshes the grown rectangle, so the pre-stroke halo and
  ramp are restored without a full-document recomposite

#### Scenario: The grown rectangle is clipped to the canvas

- **WHEN** a dab near the canvas edge is committed
- **THEN** the refresh is clipped to the canvas and no out-of-canvas pixel is
  touched
