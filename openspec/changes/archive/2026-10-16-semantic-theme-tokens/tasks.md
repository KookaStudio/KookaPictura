# Tasks

## 1. Theme token layer

- [x] 1.1 Rename the 39 colour-token keys in `Theme::tokens`, their backing
  `Ramp` fields, and every `${key}` colour placeholder in `styleSheetFor`; and
  the `ramp.<field>` accesses in `rampFor`, `paletteFor`, `Theme::tokens`,
  `workspaceColor` (both overloads), and `panelColor` (design D5).
- [x] 1.2 Rename the 8 metric keys and the 3 pseudo-tokens (`${font-size-tab}`,
  `${font-size-input}`, `${select-chevron}`) in `Theme::tokens`,
  `styleSheetFor`, and `Theme::styleSheet` (design D6).

## 2. Literal-key consumers

- [x] 2.1 Update the `Theme::tokenColor("<key>")` literals in
  `panels/curve_widget.cpp`, `panels/histogram_panel.cpp`,
  `panels/navigator_panel.cpp`, `panels/spectrum_bar.cpp`,
  `acr/acr_controls.cpp`, and `acr/acr_tone_curve_panel.cpp`.
- [x] 2.2 Update `design_system_dialog_pages.cpp`: the token-key literals in the
  foundation groups and story key lists, and the token names quoted in the
  `note()` prose. Leave the display labels (`"hover"`, `"pressed"`,
  `"workspace"`) untouched (design D7).
- [x] 2.3 Update the token-key literals in `tests/tst_design_system.cpp`
  (including `isPaintOnlyToken`) and `tests/tst_design_system_dialog.cpp`,
  leaving every assertion and expected value unchanged.

## 3. Generated style guide

- [x] 3.1 Replace the `var(--oldkey)` set in the `STYLE` constant of
  `scripts/gen-styleguide.py`, then regenerate `design-system.html` (regeneration
  pending the build).

## 4. Tests and verification

- [x] 4.1 Build once, then run `ctest --test-dir build -R '^tst_design_system'
  --output-on-failure`, `./build/pictura --headless --dump-theme`, and
  `python3 scripts/gen-styleguide.py` (a second run must be byte-identical).
  Confirm the dump keys are the new names and every token value is unchanged.
- [x] 4.2 Grep the tree for each old key as `${old}` and as a quoted literal;
  confirm no token-key occurrence remains, and that the only survivors are the
  non-token words (`QMainWindow`/QSS selectors, `QPalette::Window`-style enums,
  the story labels, `tst_keyboard_shortcuts.cpp:363`).
- [ ] 4.3 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, the unified
  test report, file-size, guard, and openspec all pass.

## 5. Docs

- [x] 5.1 Record the rename in `docs/dev/STATE.md` (the commit carries
  `TASK-ALLOWS-DOCS` per the AGENTS.md docs rule); leave the earlier milestone
  entries that mention old token names as historical record.
