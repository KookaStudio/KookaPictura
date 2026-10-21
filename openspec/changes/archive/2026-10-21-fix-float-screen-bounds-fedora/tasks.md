# Tasks

## 1. Self-test check

- [x] 1.1 In `crates/pictura-app/cpp/selftest_shell_round4.cpp`, rewrite the `float_screen_bounds` check: drop the `frameRect` band precondition and the "sits outside the frame" assertion, drop the `resize(700, 500)` setup and its restore, keep the window at the screen origin with its position restored afterwards, and assert that a float moved past the screen's bottom-right corner comes back inside the screen's available geometry and on the screen's bottom-right edge; verified the diff leaves the check's exit code 437 and its `ST_BEGIN` site in place (`check-selftest-budget: OK (508/508)`).
- [x] 1.2 Verified the check still has teeth: with the clamp in `PanelColumn::moveFloat` inset by one pixel, the check fails (`on_screen=1 on_screen_edge=0`, `FAIL core float_screen_bounds 437`); with the clamp pointed at the main window's rect instead of the screen, the suite fails at the sibling check 136 (`clipped=0`); the clamp was restored and the suite is green again.

## 2. Verification

- [x] 2.1 Built the app and ran `./build/pictura --headless --self-test`; verified the run prints `pictura self-test: PASS core float_screen_bounds float_screen_bounds torn=1 on_screen=1 on_screen_edge=1 sb=799 sr=799 fb=799 fr=799`, the summary is `passed=472 failed=0 skipped=0`, and the process exits 0.
- [x] 2.2 Ran `python3 scripts/check-test-plan.py` (1 change checked, no errors) and `openspec validate --all --strict` (282 passed, 0 failed); verified `scripts/guard.sh` and `scripts/check-file-size.sh` pass.
- [x] 2.3 Archived the change with `openspec archive fix-float-screen-bounds-fedora`; verified `openspec/changes/` holds no active change and `scripts/check-openspec-archived.sh` passes.
