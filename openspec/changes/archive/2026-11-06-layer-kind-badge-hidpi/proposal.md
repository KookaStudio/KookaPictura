# Proposal

## Why

On a high-density display the Layers panel's lower-right kind badge — the shape
badge on a shape row and the smart-object page badge on a smart-object row — is a
blurry, illegible blob. `LayerRowDelegate::paint` renders the badge from
`pictura::icon(id).pixmap(side, side)`, the 1.0 device-pixel-ratio overload, and
draws it into a logical-size rect, so the compositor upscales a low-resolution
pixmap. The delegate's own `paintAsset` (the eye and chevron) already renders at
the painter's device pixel ratio.

## What Changes

- Render the shape and smart-object corner badges from a pixmap at the painter's
  device pixel ratio, through a shared `paintKindBadge` helper built on
  `paintAsset`, so both stay crisp at 1x and 2x.
- Add a `kindBadgeRect` delegate accessor and a device-pixel-ratio parameter to
  the panel's `rowImageForTest` seam so a Qt test can paint a row at 2x.
- Add a Qt test that paints a shape row and a smart-object row at 2x and asserts
  each badge matches a crisp 2x reference rather than a 1x pixmap upscaled to 2x.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: adds a requirement that the delegate's lower-right kind
  badge (shape and smart-object) renders at the painter's device pixel ratio,
  with a scenario covering a 2x paint.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h` (shared badge helper,
  `kindBadgeRect` accessor; both badges route through it).
- `crates/pictura-app/cpp/panels/layers_panel.h` and `layers_panel_test.cpp`
  (`rowImageForTest` gains a device-pixel-ratio parameter).
- `crates/pictura-app/cpp/tests/tst_panels_parity.cpp` (new Qt test).
- No document, codec, compositing, or dependency changes.
