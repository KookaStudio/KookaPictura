# Tasks

## 1. The regression tests

- [x] 1.1 Add `scrollbarRangeFollowsThePanRange` to
  `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp`: a `CanvasScrollBars`
  around an `ImageView` with a 1000x800 document at zoom 1.0 (the initial view
  fits the document, so the zoom is set explicitly). The case reads the canvas
  size, the document size and the zoom at run time and compares the bar's
  range against the shared range helper; it asserts that the bar's minimum
  shows the start of the document and its maximum its end, that a hand pan
  moves the handle one pixel per pixel, that a pan into the reveal margin
  moves the handle with the canvas and reaches the end of the track only at
  the pannable limit, and that a document that fits keeps the bars visible with
  their pan range and the Free Pan collapse. Verify that the case builds and
  fails against the current code, its message naming the range it expected.
- [x] 1.2 Add `scrollbarHandleMovesAgainstTheContent` to the same file: raise
  the horizontal and the vertical bar's value on the overflowing document and
  read both offsets, asserting that each offset falls. Verify that the case
  builds and fails against the current code, which raises the offset instead of
  lowering it.
- [x] 1.3 Confirm `tst_canvas_pan` is registered once and is not sharded in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`, so the two new cases run in
  the existing entry. Verify with
  `grep -n tst_canvas_pan crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 2. The projection

- [x] 2.1 In `crates/pictura-app/cpp/canvas_scrollbars.cpp`, invert the four
  value↔offset expressions: a bar change sets the offset to the shared range's
  maximum minus the value, and a sync reads the value as the shared range's
  maximum minus the offset. The range stays the one `offsetRangeFor` derives,
  so the handle still reaches every pannable position. Verify with
  `ctest --test-dir build -R '^tst_canvas_pan' --output-on-failure`: the two
  new cases pass and `freePanCollapsesScrollbar` and
  `workspaceSnapHidesScrollbars` still pass.
- [x] 2.2 Update the `CanvasScrollBars` class comment in
  `crates/pictura-app/cpp/canvas_scrollbars.h`: it promises a projection of the
  canvas offset without saying in which direction. Verify with
  `grep -n 'projection of the canvas' crates/pictura-app/cpp/canvas_scrollbars.h`,
  which must print nothing.

## 3. The self-test

- [x] 3.1 Update the `canvas_scrollbar_roundtrip` check (exit code 309) in
  `crates/pictura-app/cpp/selftest_canvas_view.cpp`: its two assertions become
  the shared range's maximum minus the bar's value and the shared range's
  maximum minus the offset. Keep the exit code and do not add an `ST_BEGIN`
  site, so the self-test budget does not move. Verify with
  `./build/pictura --headless --self-test`: the run prints
  `pictura self-test: PASS core canvas_scrollbar_roundtrip` and exits 0.
- [x] 3.2 Confirm the neighbouring checks still hold: `canvas_pan_margin` (308)
  exercises the pan clamp, which this change does not touch, and
  `canvas_scrollbar_visible` (310) sets the bar to its minimum and requires the
  offset to change. Verify with the same headless self-test run.

## 4. The documentation

- [x] 4.1 Record the new rule in `docs/dev/STATE.md`: the workspace bars
  project the canvas's pan range and the handle travels against the content, so
  the bar's minimum shows the start of the document and its maximum its end.
  Verify with `bash scripts/guard.sh`, which requires the commit to carry
  `TASK-ALLOWS-DOCS`.

## 5. The integration checks

- [x] 5.1 `cargo fmt --all` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.
- [x] 5.2 A CMake/Ninja build succeeds and
  `ctest --test-dir build -R '^tst_canvas_pan' --output-on-failure` passes.
- [x] 5.3 `python3 scripts/check-test-plan.py`,
  `bash scripts/check-selftest-budget.sh` and
  `openspec validate --all --strict` pass.
- [x] 5.4 Archive the change with
  `openspec archive fix-workspace-scrollbar-direction -y`, so the pull request
  leaves no open change.
