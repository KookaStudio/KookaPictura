# Proposal: color-replacement-tool

## Why

The Color Replacement tool (issue #15) was catalogued but disabled. photorust
(perfecto25/photorust) ships it as `replace::ColorReplacer` over its `sample`
matching helpers. This change ports it onto Kooka's paint engine and tool
framework (`docs/03-tools/color-replacement.md`).

## What Changes

- `pictura_paint::replace`: `ReplaceMode` (Hue / Saturation / Color /
  Luminosity), `Sampling` (Continuous / Once / Background Swatch), `Limits`
  (Discontiguous / Contiguous / Find Edges), `ReplaceOptions`, and the per-dab
  `ColorReplacer` (tolerance match, anti-aliased taper, per-dab contiguity
  flood, one replacement per pixel per stroke, transparent pixels kept).
- `pictura_paint::Stroke` gains `StrokeKind` and `begin_kind`: a `Replace`
  (or `Mixer`) stroke edits the layer at every dab instead of accumulating a
  coverage mask, so the Brush's live preview, commit, and cancel are reused.
- The W3C non-separable helpers (`lum`, `sat`, `set_lum`, `set_sat`) move
  from `pictura-render` to `pictura_core::nonseparable`, shared by the
  compositor and this tool (a pure move).
- `cxxqt_object/paint_tools.rs`: `begin_color_replacement`; the stroke records
  one `"Color Replacement Tool"` state through `end_paint`.
- `tool_colorreplacement.cpp` (Alt samples the foreground); options-bar row:
  Size, Hardness, Mode, Sampling, Limits, Tolerance (30 %), Anti-alias. The
  size ring and `[` / `]` cover all four `B` brushes. Catalog row enabled.
- C++ self-test `color_replacement_tool` (540); `shift_plain` (117) asserts the
  B cycle; `keys_shown` (116) probes the S group for a disabled member.

## Capabilities

### New Capabilities

- `tools/color-replacement-tool`: the Color Replacement tool and its engine.

## Impact

- `pictura-core` (`nonseparable.rs`), `pictura-render` (`blend.rs` imports it),
  `pictura-paint` (`replace.rs`, `stroke.rs`), `pictura-app` bridge and C++.
- No new dependency.

## Provenance

Ported from photorust's `core/src/replace.rs`, `core/src/sample.rs`, and
`shell/src/MainWindow.cpp` (`addColorReplaceOptions`)
(<https://github.com/perfecto25/photorust>). Behavioural parity only: the
per-channel distance and the Find Edges luminance step are approximations;
Opacity/Flow and document-mode gating are not modelled (documents are
normalised to RGB on open).
