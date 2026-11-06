# Spec Delta

## ADDED Requirements

### Requirement: Kind badges render at the device pixel ratio

The delegate's lower-right kind badge — the shape badge on a shape row and the
smart-object badge on a smart-object row — SHALL be rendered from its SVG at the
painter's device pixel ratio and drawn at the badge's logical size, so it stays
crisp on a high-density display instead of being upscaled from a logical-size
pixmap. The badge's corner, size, base fill, and the row's other content SHALL be
unchanged.

#### Scenario: A kind badge stays crisp at 2x [lpr_kind_badge_hidpi]

- **WHEN** a shape row and a smart-object row are painted into a 2x device
- **THEN** each badge's pixels match the badge rendered from a 2x pixmap more
  closely than the badge rendered from a 1x pixmap upscaled to 2x
