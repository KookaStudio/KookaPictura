# Tasks

## 1. Lucide chevron assets

- [x] 1.1 Add `assets/icons/chevron.down.svg` and `assets/icons/chevron.up.svg`
  (Lucide, stroke `currentColor`), map both in `assets/icons/lucide-map.json`,
  and register them in `assets/pictura.qrc`; verify `python3 scripts/check-icon-provenance.py`
- [x] 1.2 Remove the fixed-colour `assets/ui/combo-caret{,-disabled}.svg`, their
  qrc entries, and the `assets/PROVENANCE.md` bullet

## 2. Tinted arrows in the app style

- [x] 2.1 In `theme.cpp`, draw the spin-box, combo-box, and menu-button arrow
  primitives as `icon("chevron.up"|"chevron.down", palette foreground)` in
  `SpinArrowStyle::drawPrimitive`, so the glyph follows the theme family and
  level; verify spin and combo arrows repaint after a family/level change
- [x] 2.2 Style the select with a flat `QComboBox::drop-down` cell (which also
  reserves the value's trailing space) and a `QComboBox::down-arrow` image; the
  chevron is a per-family baked asset (`assets/ui/select-chevron{,-light}.svg`)
  because the stylesheet engine cannot tint an SVG, selected via
  `${selectChevron}` in `Theme::styleSheet(family, level)`

## 3. Inset slider arrow

- [x] 3.1 Reparent the `NumericField` popup arrow into the value editor, place it
  by `layoutInset()` flush to the inner edge with the suffix clamped short of it,
  and grow `reserveWidth()` by the arrow; give it the chevron icon, a
  `:checked` active state driven by popup show/hide, and the default arrow cursor
- [x] 3.2 Add `numericFieldArrowIsInset` to `tst_design_system.cpp` (parent,
  geometry, text margin, chevron icon, active-while-open); verify
  `ctest --test-dir build -R tst_design_system`
- [x] 3.3 Update `tst_layers_panel_fixes.cpp`'s `blendComboArrowAndKeys` to
  assert the chevron shape (ink band inside the frame, narrowest at the bottom)
  rather than the old triangle heuristic; verify the test

## 4. Input family

- [x] 4.1 In `theme.cpp`, unify the line edit / spin box / combo box / text edit
  surface and states, neutralise the editable combo's inner editor, style the
  `#workspaceSwitcher` on the same surface, and add the `${inputFontSize}`
  substitution (base − 1) alongside `${tabFontSize}`
- [x] 4.2 Add the new selectors to `stylesheetDeclaresPrimitives` and update the
  removed `QComboBox::drop-down` / `#numericField` expectations in
  `tst_design_system.cpp`; verify `ctest --test-dir build -R tst_design_system`
- [x] 4.3 Restore the checkbox tick: add `assets/ui/checkbox-check.svg` and set
  `QCheckBox::indicator:checked { image: ... }`; verify a checked box draws the
  tick
- [x] 4.4 Give the non-spin inputs a `min-height` so the line edit, select, and
  spin box render at one height; verify the measured heights are equal

## 5. Integration

- [x] 5.1 Run `cmake --build build --parallel`, `ctest --test-dir build -R '^tst_'`,
  `./build/pictura --headless --self-test`, and `bash scripts/verify-full.sh`;
  verify all green, then update `docs/dev/STATE.md`
