# Spec Delta

## ADDED Requirements

### Requirement: Crop content pan presents through a level crop

While an active Modern crop box is being repositioned by an inside drag, the
canvas SHALL present the document content through the same zoom-selected
view-pyramid level it presents when idle, shifted by the content offset, and MUST
NOT rescale the full-resolution document. Each repaint MUST bound its crop to the
visible content plus the present's sampling margin (one presented-level pixel per
edge, plus whole-pixel rounding), and MUST NOT request the full-resolution
document. The panned frame SHALL be
pixel-identical to the idle present of the same content at the same zoom and
sampling filter; a content offset that is not a whole multiple of the presented
level's scale MAY be quantised to that level. A visible channel selection SHALL
keep being presented while the content moves.

#### Scenario: A content pan crops only what the viewport shows [crop_pan_level_present]

- **WHEN** an active Modern crop box is dragged from inside on a document much larger than the viewport
- **THEN** every crop the repaint requests is bounded by the visible viewport plus the sampling margin (one presented-level pixel per edge), and no full-document crop is requested

#### Scenario: A content pan frame matches the idle present [crop_pan_matches_idle]

- **WHEN** the same document content is presented once through a content offset and once through the equivalent view pan of the same direction and distance in document space
- **THEN** the two rendered frames are pixel-identical at 100 % zoom and at or below 50 % where the present uses a stored level

#### Scenario: A single-channel view keeps moving with the content [crop_pan_single_channel]

- **WHEN** exactly one channel is visible and an active Modern crop box is dragged from inside
- **THEN** the canvas keeps presenting that channel in greyscale while the content moves, instead of presenting the composite
