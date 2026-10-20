# Test Plan

The first scenario is exercised by the automated `--headless --self-test`;
`check-test-plan.py` cannot name a self-test check, so it is recorded as a
manual row with the self-test command as the evidence. The other two are
build-log checks whose output depends on the toolchain. The manual checks below
are the acceptance evidence and are listed in the PR body as the reviewer
checklist.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vh_float_screen_bounds` | Build the app, then run `./build/pictura --headless --self-test` twenty times. The check resets the frame to the screen origin before measuring, so the initial position is not an input. | Every run prints `pictura self-test: PASS core float_screen_bounds` with the available right band present, the float clamped inside the screen on that axis, and the frame geometry restored; the process exits 0. | manual: the check itself is the automated self-test, whose coverage cannot be referenced by `check-test-plan.py`; the cross-platform starting position is not reproducible in one local run, so repeated runs are the flake evidence. |
| `vh_warning_free_build` | Configure and build from a clean build directory, capturing stderr, then search the capture for warning lines. | No CMake or compiler warning is caused by project code or configuration; only documented third-party warnings remain. | manual: warning output depends on the compiler and Qt version, so it is verified by capturing the logs rather than by a test assertion. |
| `vh_warning_free_gates` | Archive the change, then run `bash scripts/verify-fast.sh` and `bash scripts/verify-full.sh`, capturing stderr, and search the captures for warning lines. | Neither gate prints a warning caused by project code or configuration; only documented third-party warnings remain. | manual: the gates are shell scripts whose warning output depends on the toolchain; the captured output is the acceptance evidence. |

## Warning inventory to confirm in the captures

Expected absent after the change:

- Cargo: the `profile package spec` warning for `pictura-app`.
- CMake: the `Qt6GuiPrivate` private-module warning.
- C++ build: the deprecated filter-invalidation warnings.
- C++ build: the ignored `QFile::open()` result warning.
- C++ build: the ignored `QTest::qWaitForWindowActive()` result warning.
- C++ build: the `QApplication::setActiveWindow()` deprecation warning.
- C++ build: GCC 16's `-Wsfinae-incomplete` and `-Wmaybe-uninitialized` warnings
  from cxx-qt/Qt generated glue.

Expected unchanged and documented:

- Packaging: linuxdeploy, `dpkg-shlibdeps`, and bundled-Qt warnings. These are
  third-party and listed here and in the PR.
- Lint: `cargo deny` duplicate and unused license-allowance warnings. These are
  outside the three gates named by the issue and are left for a follow-up.
