# Tasks

## 1. Tests

- [x] 1.1 Add the tests in `test-plan.md`

## 2. Footer message handling

- [x] 2.1 Hide the readouts while a temporary message is shown and restore them on `messageChanged` (`updateStatus`, `buildStatusBar`)
- [x] 2.2 Make `NotificationStatusBar` the single message painter (themed panel, then pill and text once)
- [x] 2.3 Measure the banner rect against the bar's direct children

## 3. Verification

- [x] 3.1 Build with CMake and run `ctest --test-dir build -R '^tst_notification_status_bar$' --output-on-failure`
- [x] 3.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `openspec validate --all --strict`
- [x] 3.3 Run `bash scripts/verify-fast.sh` (the only failure is the pre-existing `pictura-render::oracle::imagemagick_fixtures_reproduce` `ramp_Divide.rgba` mismatch, unrelated to this C++-only change; every guard and the Qt layer pass)

## 4. Ship

- [ ] 4.1 Archive the change with `openspec archive footer-message-replaces-readouts` and open the PR with the manual rows from `test-plan.md` (none expected)
