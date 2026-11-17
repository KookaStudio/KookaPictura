# Tasks

## 1. Tests

- [x] 1.1 Add `tst_design_system::contrastShadeFollowsFamily`: the helper lightens
  under the dark family and darkens under the light family.
- [x] 1.2 Add `tst_panels_parity::layerRowTextStaysLegibleInLightFamily`: a
  selected row under the light family has a non-white surface and a contrasting
  name glyph.
- [x] 1.3 Add `tst_panels_parity::channelRowsFollowLightFamily`: the selected
  channel row steps darker than the unselected rows under the light family.
- [x] 1.4 Add `tst_brush_dynamics::tipPresetsFollowLightFamily`: a live switch
  repaints the tip preset glyphs from light to dark ink.
- [x] 1.5 Add a live-palette option to `LayersPanel::rowImageForTest` so the
  light-family row test paints with the applied palette.

## 2. Theme helper

- [x] 2.1 Add `Theme::contrastShade(color, steps)` beside `Theme::shade`.

## 3. Panels

- [x] 3.1 Derive the Layers row, gutter, selection, and Blend If chip surfaces
  with `contrastShade`.
- [x] 3.2 Derive the Channels row surfaces with `contrastShade` and re-style on
  `PaletteChange` / `StyleChange`.
- [x] 3.3 Draw the Brush tip glyphs and size labels in the `icon` token and
  regenerate them on `PaletteChange` / `StyleChange`.

## 4. Gates

- [x] 4.1 Build and run `tst_design_system`, `tst_layers_panel`,
  `tst_panels_parity`, and `tst_brush_dynamics`.
- [x] 4.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 4.3 Archive with `openspec archive light-theme-contrast`.
