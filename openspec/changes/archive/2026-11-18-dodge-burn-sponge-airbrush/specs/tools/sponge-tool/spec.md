## MODIFIED Requirements

### Requirement: Sponge tool

A Sponge drag SHALL move the covered pixels of the active pixel layer toward
grey (Desaturate) or away from it (Saturate) by an amount set by Flow and the
tip's coverage, keeping each pixel's luminance and alpha and applying the
effect once per pixel per stroke. With Vibrance on the effect SHALL ease off
where the colour is already vivid (Saturate) or already nearly grey
(Desaturate). In an RGB document a grey pixel SHALL be left unchanged. In a
Grayscale document the Sponge SHALL move grey levels away from (Saturate) or
toward (Desaturate) middle grey, increasing or decreasing contrast. A stroke
SHALL record exactly one "Sponge" history state when pixels changed, and a
layer whose pixels are locked SHALL be refused.

#### Scenario: Draining and lifting colour

- **WHEN** the `tst_retouch_tools` Sponge test drags across a red at the defaults, then again in Saturate mode
- **THEN** each drag records one "Sponge" state, the first drains colour, the second lifts it, and pixels outside the tip are unchanged

#### Scenario: Grayscale contrast [sponge_grayscale_contrast]

- **WHEN** the Sponge drags across a Grayscale document in Saturate mode
- **THEN** dark greys get darker and light greys get lighter, moving away from middle grey
