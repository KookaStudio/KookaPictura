# Proposal

## Why

The X/Y field slider popups span the full typed range of ±1,000,000 px. A single
slider step changes the position by roughly 11,000 px, so a tiny drag off centre
throws the object across the canvas. The user wants about 100 px per slider step.

## What Changes

- Give the X/Y popup sliders a coarse span of 20,000 px (±10,000) centred on 0,
  so the slider step is 100 px.
- Keep the typed field range at ±1,000,000 px, so precise or far-away positions
  still work.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/transform-options-bar`: the X/Y slider span.

## Impact

- `crates/pictura-app/cpp/panels/numeric_field.*`: an optional `sliderSpan`
  narrower than the typed range.
- `crates/pictura-app/cpp/options_bar_transform.cpp`: X/Y configure the span.
- Tests: `tst_transform_options`.
- No new dependencies or document fields.
