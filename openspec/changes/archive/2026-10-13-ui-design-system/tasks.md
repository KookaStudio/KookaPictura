# Tasks

## 1. Token contract

- [x] 1.1 Add `ThemeToken` and `Theme::tokens(int level)` in `theme.{h,cpp}`, moving every `ColorToken` and derived shade plus the metrics (panel/chrome border, group divider, slot footprint and icon maxima, shade step) into the accessor, and rebuild `styleSheet` from it; verify `cargo clippy` is untouched, the app builds, and the self-test still passes
- [x] 1.2 Add the `--dump-theme` argument to `main.cpp` that prints the all-levels JSON (colors + metrics) on stdout and exits 0 before the frame is built; verify `./build/pictura --headless --dump-theme` emits valid JSON and exits 0
- [x] 1.3 Cover the accessor in `tst_design_system` (tokens shape, derived tokens present, stylesheet equals token values); verify `ctest -R tst_design_system`

## 2. Primitive stylesheet

- [x] 2.1 Add the canonical primitive rules to the global QSS (text button rest/hover/pressed/checked/disabled/focus, icon/toggle button focus, text input hover/focus, numeric field, combo, check box, slider, panel/document tabs, menu, panel header) with every value from tokens; verify the stylesheet strings via `tst_design_system`
- [x] 2.2 Add `ColorSwatchWidget` (`panels/color_swatch_widget.{h,cpp}`) with `setColor`/`color`/`setActive`, palette-painted fill and frame, the `color` dynamic property, and register it in `CMakeLists.txt`; verify it instantiates and round-trips colour/active in `tst_design_system`
- [x] 2.3 Give each primitive its object name where a surface is scoped (`numericField`, `colorSwatch`, panel/document tab bars) and assert the object-name/state contract in `tst_design_system`; verify `ctest -R tst_design_system`

## 3. Consistency pass

- [x] 3.1 Fold the static literal stylesheets into object-name-scoped global QSS and delete the calls: `optionsBrushTip`, `statusOptionsButton`, the panel/drop indicators (new `accent` token), `imageSizePreview`, `brushTipPreview`, `replaceColorMaskPreview`, the gallery-effects indicator, the tool flyout menu padding, and the bold title/tab labels; verify the app builds and `rg setStyleSheet` shows no static colour left
- [x] 3.2 Route the data-driven swatch sites through `ColorSwatchWidget` (`adjustment_dialog`, `filter_param_controls`, `layer_style_dialog`, `lighting_effects_dialog`, `stroke_dialog`, `replace_color_dialog`, `gradient_editor_dialog`, `canvas_size_dialog`, `color_panel`, `preferences_dialog`) and update the `tst_filter_menu` swatch assertion to the widget's colour; verify `ctest -R 'tst_(filter_menu|image_adjustments|replace_color|layer_style|lighting_effects_dialog)'`

## 4. Generated style guide

- [x] 4.1 Add `scripts/gen-styleguide.py` that runs the binary's dump, emits CSS custom properties per level, writes `design-system.html` at the repo root, and is idempotent; verify two consecutive runs leave the file byte-identical
- [x] 4.2 Generate and commit `design-system.html` rendering every colour token, metric, primitive, and state across all four levels; verify the page opens and references only generated variables

## 5. Integration checks

- [x] 5.1 Register `tst_design_system` in `crates/pictura-app/cpp/tests/CMakeLists.txt` and run the full suite; verify `ctest --test-dir build -R '^tst_'` is green
- [x] 5.2 Run `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` and `openspec validate --all --strict`; verify both are green, and update `docs/dev/STATE.md`
