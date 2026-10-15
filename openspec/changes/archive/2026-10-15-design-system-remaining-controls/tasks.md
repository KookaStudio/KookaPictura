# Tasks

## 1. Remaining primitive styles

- [x] 1.1 Add one tokenised global QSS rule group for `QGroupBox`, `QRadioButton`, `QProgressBar`, `QHeaderView`, and a general `QSplitter` handle, covering their states with values from `Theme::tokens` and object names only where a surface differs; verify the stylesheet strings via `tst_design_system`
- [x] 1.2 Add the five selectors to `stylesheetDeclaresPrimitives` and instance/state checks to `primitivesCarryTheirContract` in `crates/pictura-app/cpp/tests/tst_design_system.cpp`; verify `ctest --test-dir build -R tst_design_system`

## 2. Custom-painted widgets read tokens

- [x] 2.1 Add the paint-only `graphFill` and `graphAxis` colour tokens to `Theme::tokens` and the `Ramp` (derived per level); verify they appear in `Theme::tokens` and that `tst_design_system`'s `stylesheetSubstitutesEveryToken` skips them as paint-only
- [x] 2.2 Route `panels/curve_widget.cpp`, `panels/spectrum_bar.cpp`, `panels/histogram_panel.cpp`, and `panels/navigator_panel.cpp` through the mapped tokens and delete the theme greys; verify `rg 'QColor\([0-9]' crates/pictura-app/cpp/panels/{curve_widget,spectrum_bar,histogram_panel,navigator_panel}.cpp` leaves only allow-listed data greys
- [x] 2.3 Add a paint assertion that grabs each converted widget and compares a sample pixel to the token value; verify `ctest --test-dir build -R tst_design_system`

## 3. Literal guard

- [x] 3.1 Add `scripts/check-colour-literals.py` and a baseline `scripts/colour-literal-allowlist.txt` seeded from the tree's data-colour sites, and call the check from `scripts/guard.sh`; verify a deliberately injected grey literal fails the check and the tree as landed passes

## 4. Integration

- [x] 4.1 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`; verify both are green and update `docs/dev/STATE.md`
