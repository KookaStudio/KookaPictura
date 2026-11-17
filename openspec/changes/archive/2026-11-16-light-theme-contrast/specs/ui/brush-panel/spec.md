# Spec Delta — ui/brush-panel

## ADDED Requirements

### Requirement: Tip preset glyphs follow the theme family

The round tip presets in the Brush panel SHALL draw their tip glyph and size
label in the active family's icon ink — light on the dark family and dark grey on
the light family — so the presets stay visible on either panel surface. The
panel SHALL regenerate those glyphs when the palette or style changes, so a live
family switch follows into the cached icons.

#### Scenario: Tip presets stay visible in the light family [bp_light_tip_ink]

- **WHEN** the light family is applied to an open Brush panel
- **THEN** every tip preset glyph and size label repaints in dark icon ink and
  contrasts with the light panel surface
