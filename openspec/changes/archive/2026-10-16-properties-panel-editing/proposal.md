# Proposal: properties-panel-editing

## Why

Issue #70: port the Properties panel's adjustment editing from photorust
(`shell/src/panels/PropertiesPanel.{h,cpp}`). Kooka's panel only named the
active adjustment layer; there was no parameter get/set and no edit session.
(The issue assumed `Adjustment::value` / `set_value` exist in Kooka; they do
not, and no encoder existed for Levels, Exposure, Vibrance, or Black & White.)

## What Changes

- `pictura-render` `adjustment_params.rs` (new): per-kind page descriptors —
  key, label, slider range / check / choice / colour, group, value, default —
  for Brightness/Contrast, Levels, Exposure, Vibrance, Hue/Saturation (Master),
  Color Balance (Shadows / Midtones / Highlights), Black & White, Photo Filter,
  Channel Mixer (per output), Selective Color (per colour, Method), Posterize,
  Threshold, and Curves (composite + R / G / B); a note for Invert, Gradient
  Map, Color Lookup, and fill layers.
- Edits patch only the bytes (or descriptor item) holding the parameter and
  must still decode, so unmodelled data survives (Hue/Saturation's Colorize
  and ranges, Brightness/Contrast's legacy and Lab bytes, Levels' per-channel
  records, Black & White's tint, unknown descriptor items). Curves is
  re-encoded from its points. `reset_adjustment` restores the defaults.
- `cxxqt_object/adjustment_edit.rs` (new bridge): the page, live set / curve /
  reset edits that refresh the canvas without a history state, and a commit
  that records one "Modify `Title` Layer" state.
- `panels/properties_panel` rewritten: builds the controls from the page
  (sliders with number fields, checks, menus, colour swatch, Curves editor with
  its channel menu, a group menu), edits live, and commits once per gesture (a
  slider release, a toggle / menu / colour change, or a 500 ms pause); CS6's
  footer — Clip to Layer, Reset, Toggle Visibility, Delete. Any other layer gets
  a read-only summary (kind, size, position, blend, opacity, fill, mask,
  locks) — **a post-CS6 page** (CS6 shows no layer properties for a pixel
  layer), ported because #70 asks for photorust's; `docs/` is unchanged.
- Tests: `adjustment_params` unit tests (data preservation, clamping, refusal,
  descriptors, groups, Photo Filter v2 / v3, Curves, Reset) and the Qt Test
  `tst_properties_panel` (extended; its plain-document case now expects the
  read-only page).

## Capabilities

### New Capabilities

- `ui/properties-panel-editing`: adjustment editing in the Properties panel.

## Impact

- `pictura-render`, `pictura-app` (bridge, C++). No new dependency.

## Provenance

Descriptor-driven controls, ranges, and the commit-per-gesture session from
photorust; CS6 behaviour from `PAN-006`. Ceiling (`ponytail:`): no adjustment
Presets menu, mask page, Previous State toggle, Auto-Select items; Channel
Mixer's Monochrome, Hue/Saturation's Colorize / ranges, Black & White's tint,
Gradient Map, and Color Lookup are not editable here.
