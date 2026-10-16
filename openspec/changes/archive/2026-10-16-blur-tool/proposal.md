# Proposal: blur-tool

## Why

The Blur tool (issue #26) was catalogued but disabled. photorust's
`core/src/focus.rs` softens pixels under a brush with a 3×3 Gaussian; the port
follows `docs/03-tools/smudge-blur-sharpen.md`.

## What Changes

- `pictura_paint::focus`: `BlurOptions` (Strength, `BlurMode` — Normal,
  Darken, Lighten, Hue, Saturation, Color, Luminosity) and the per-dab
  `BlurBrush`: each dab moves the pixels under the tip toward their 3×3
  Gaussian average (premultiplied, so a layer's edge softens without a dark
  rim), reading a snapshot so the dab does not smear, and working on what the
  last dab left so dwelling deepens the blur. `Stroke::begin_blur` runs it,
  optionally reading the composite (Sample All Layers), and refuses 16/32-bit
  documents; Lock Transparency keeps coverage.
- `cxxqt_object/paint_tools.rs`: `begin_blur`; the live stroke records one
  `"Blur"` state through `end_paint`.
- `tool_blur.cpp` (since folded into `tool_retouch.cpp`) drag handler; the bar (`options_bar_paint.cpp`, since `options_bar_retouch.cpp`) has the brush
  tip, Mode, Strength 50 %, and Sample All Layers. Catalog row enabled; Blur
  joins the brush size ring and `[` / `]`.
- Qt Test `tst_retouch_tools::blurTool` (first the self-test `blur_tool`, code 554, now retired); the guard (98)
  now probes Sharpen.

## Capabilities

### New Capabilities

- `tools/blur-tool`: the Blur tool.

## Impact

- `pictura-paint` (`focus.rs`, `stroke.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/focus.rs` and its retouch stroke
(<https://github.com/perfecto25/photorust>). Behavioural parity only: Adobe's
kernel is closed. Sharpen, the same engine with its sign flipped, is left to
its own issue. Ceilings (`ponytail:`): no pressure-driven Strength.
