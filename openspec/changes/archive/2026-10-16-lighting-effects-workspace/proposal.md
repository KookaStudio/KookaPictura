# Proposal: lighting-effects-workspace

## Why

Kooka's Lighting Effects is photorust's single lamp, edited on the generic
slot dialog. That dialog is a column of 19 sliders, a combo, and a crosshair
pad. CS6 instead opens a dedicated workspace (`FILT-103`): a Presets menu and
add-light buttons in the options bar, the picture with on-canvas light
controls, a Properties panel, and a Lights panel. It holds a rig of up to
sixteen Spot, Point, and Infinite lights. CS6's spot is an ellipse with its
hotspot at the far end, aimed by dragging. Kooka's spot is a round pool. The
second half of #225 asks to port photorust's Lighting Effects and give it a
dedicated dialog. The user asked for the menu entry and the effect to look
like Photoshop.

## What Changes

- `render::lighting_effects` moves to `render/lighting.rs` and takes a rig.
  `Lighting` holds the shared properties (Colorize, Exposure, Gloss, Metallic,
  Ambience, Texture, Height) and `lights: Vec<Light>` (1..=16). Each `Light`
  has a type, a visible flag, colour, intensity, hotspot, centre, angle, size,
  width, and elevation.
- A Spot is CS6's ellipse: semi-axes `size` and `width` along `angle`, with a
  hotspot ellipse that sits toward the far end and grows with Hotspot. A Point
  is a circle of radius `size`. An Infinite light comes from `angle` at
  `elevation`. Every light adds its diffuse term and highlight. Intensity is
  scaled so that about 50 is normal, as CS6's Help states. Sizes stay
  fractions of the half-diagonal, so the workspace proxy and the commit
  agree.
- The `lighting-effects` kind takes a variable slot list: 9 rig slots, then 13
  per light. The arity table lists a one-light rig (22). Any whole number of
  lights up to 16 maps, and anything else is refused.
- `Filter ▸ Render ▸ Lighting Effects…` and Last Filter Settings open
  `LightingEffectsDialog`, CS6's workspace:
  - The options bar has Presets (CS6's 17 styles plus Custom), add Spot /
    Point / Infinite, Reset, Preview, Cancel, and OK.
  - The picture shows on-canvas controls. Drag a Spot inside to move it,
    beyond the ellipse to turn it, and by its handles to stretch it. Drag its
    hotspot edge, a Point's ring, an Infinite light's end handle, or any
    light's Intensity ring. Alt-drag duplicates a light. Delete removes the
    selected light.
  - The Properties panel holds the light type, Color, Intensity, and Hotspot
    (greyed off a Spot), then Colorize, Exposure, Gloss, Metallic, Ambience,
    Texture, and Height (greyed until a texture is chosen).
  - The Lights panel has one row per light, with an eye and a type glyph, and
    a trash button.

## Capabilities

### Modified Capabilities

- `imaging/render-filters`: *Lighting Effects filter* becomes the CS6 light
  rig.
- `imaging/filter-app-ui`: *New filter-kind mapping* gives `lighting-effects`
  its variable rig arity.

### New Capabilities

- `imaging/filter-app-ui` gains *Lighting Effects workspace*.

## Impact

- `pictura-filters` `render` (new `render/lighting.rs` +
  `render/lighting_tests.rs`); `pictura-app` `filter_map.rs`,
  `filter_commands.cpp`, `frame_menus_filter.cpp`, and the new
  `lighting_rig.{h,cpp}`, `lighting_canvas.{h,cpp}`, and
  `lighting_effects_dialog.{h,cpp}`, plus `tst_lighting_effects_dialog`.
- **Output changes:** every Lighting Effects result changes. No golden baseline
  covers Lighting Effects. A `lighting-effects` slot list saved by an earlier
  build (19 slots) no longer maps.
- **Oracle:** none. CS6's lighting model is closed, and this one is tuned by
  eye against CS6 screenshots. Coverage is photorust's property tests,
  extended to the rig, plus Qt Test cases for the workspace. No new self-test
  checks (rule 11).
- **Not built:** saving and deleting custom presets, alpha channels as
  textures, and the RGB-only / GPU gate (`FILT-103` open questions).
