# Tasks

## 1. Tests

- [x] 1.1 Add the `pictura-render` styled-shaping and rendering tests named in `test-plan.md` (`crates/pictura-render/src/type_style/tests.rs`).
- [x] 1.2 Add `pictura-app::character_setting_round_trips_every_field` and `pictura-app::optical_kerning_crosses_the_bridge`.
- [x] 1.3 Add the `tst_character_paragraph_panels` cases named in the plan.

## 2. Model and codec

- [x] 2.1 Add `KerningMode::Optical`; make every match arm handle it (`type_paragraph`, `text_render`, codec writer as `AutoKerning true` with a `ponytail:`).
- [x] 2.2 Add `scale: f32` to `ShapedGlyph` and `PlacedGlyph`, carried by `layout_lines`.

## 3. Renderer

- [x] 3.1 `type_style`: feature list, `shape_styled` (caps, small caps, super/sub size, H/V scale, bold widening, kerning modes, optical), `RunStyle`, decorations, `opentype_support`.
- [x] 3.2 Route `layout_type`, `vertical_layout`, `line_advance`, `text_box_any`, and `type_pens` through `shape_styled`/`RunStyle`.
- [x] 3.3 Paint per-glyph scale, skew, embolden, and rise in `paint_layout_scaled` and `render_mapped`, then the decoration rectangles.
- [x] 3.4 Export `opentype_features(family)`.

## 4. Bridge and panel

- [x] 4.1 Bridge: kerning mode 1 = Optical; `type_font_opentype_features`; `type_api::openTypeFeatures`.
- [x] 4.2 Combo-field scrubbing on the icon and the line edit (panel event filter); drawn icons.
- [x] 4.3 Rebuild `CharacterPanel` to the CS6 layout with drawn icons, the toggle and OpenType button rows, the preset combos, and scrubbing.

## 5. Verify

- [x] 5.1 Build, run the plan's tests, `verify-fast.sh`, and `openspec validate --all --strict`; archive the change.
