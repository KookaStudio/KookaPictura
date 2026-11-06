# Tasks

## 1. Tests

- [x] 1.1 Add the tests in `test-plan.md`

## 2. Panel surface

- [x] 2.1 Name the Adjustments scroll area, viewport, and grid host
- [x] 2.2 Paint all three with `${surface}` in the theme

## 3. Verification

- [x] 3.1 Build with CMake and run `ctest --test-dir build -R '^tst_adjustments_panel$' --output-on-failure`
- [x] 3.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `openspec validate --all --strict`
- [x] 3.3 Run `bash scripts/verify-fast.sh` (only the pre-existing `pictura-render::oracle::imagemagick_fixtures_reproduce` `ramp_Divide.rgba` mismatch fails; every guard and the Qt layer pass)

## 4. Ship

- [ ] 4.1 Archive the change with `openspec archive adjustments-panel-surface` and open the PR with the manual rows from `test-plan.md` (none expected)
