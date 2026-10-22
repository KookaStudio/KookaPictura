# Spec Delta

## ADDED Requirements

### Requirement: Merge Down honours the source layer's blend mode

Merge Down SHALL composite the source layer over the target layer through the
document compositor, so the source layer's own blend mode and opacity govern
the composite; the merge SHALL NOT replace the source's blend mode with
`Normal`. The source's blend mode SHALL affect the merged pixels. The merge
path SHALL be diffed against the ImageMagick blend operators for the 19 modes
ImageMagick implements with the same formula, so the guarantee is checked
against an external implementation rather than asserted.

#### Scenario: a Multiply source composites as Multiply [merge_down_source_blend]

- **WHEN** a source layer whose blend mode is `Multiply` is merged down over a
  target layer
- **THEN** the merged node's pixels equal the `Multiply` composite of the two
  layers, and the ImageMagick `Multiply` reference for the same pair, within
  that mode's tolerance

#### Scenario: every ImageMagick-compatible mode is checked [merge_down_all_modes]

- **WHEN** the merge oracle runs over the solid and ramp scenes
- **THEN** for each of the 19 modes ImageMagick implements with the same
  formula, the merged node equals the committed ImageMagick reference within
  that mode's tolerance
