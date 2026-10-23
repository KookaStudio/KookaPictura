# Spec Delta

## ADDED Requirements

### Requirement: Baseline Shift moves path type

Path type SHALL apply the character run's Baseline Shift opposite the path
normal: a positive shift moves the glyphs and caret stops opposite the normal
by the run's rise (up for a horizontal path), and a negative shift moves them
along the normal. The rendered alpha bounds and the layer's text bounds SHALL
follow the shifted glyph positions.

#### Scenario: Baseline Shift moves path glyphs and the caret [tp_baseline_shift]

- **WHEN** the same path text is rendered with Baseline Shift 0 and with a
  positive shift
- **THEN** every glyph and caret stop moves by that distance opposite the path
  normal, and the alpha bounds move with them
