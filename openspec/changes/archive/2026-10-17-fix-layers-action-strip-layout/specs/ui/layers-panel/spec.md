# Spec Delta

## ADDED Requirements

### Requirement: Layers panel action strip geometry

The Layers panel's seven-button action strip SHALL keep a constant 2 px gap
between adjacent buttons and SHALL stay horizontally centred in the panel as the
panel width changes. Each strip button SHALL keep its intrinsic width. Surplus
panel width SHALL be absorbed by the space at the strip's leading and trailing
ends, not by the inter-button gaps or the buttons. The strip's button order,
object names, icons, tooltips, click behaviour, and drop targets SHALL remain
unchanged.

#### Scenario: The strip keeps its fixed gap and stays centred [lpc_strip_geometry]

- **WHEN** the Layers panel is shown and then widened while its overall width
  stays above the panel's minimum width
- **THEN** every adjacent pair of strip buttons keeps the same 2 px gap, each
  button keeps the same width it had before the resize, and the distance from
  the panel's left edge to the first button equals the distance from the last
  button to the panel's right edge within one pixel
