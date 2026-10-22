# Tasks

Groups 1–4 are written and group 5's commands have run: `cargo clippy`
(with a private `CARGO_TARGET_DIR` — the shared `../.target` tree was
poisoned by another worktree's newer sources), `cargo nextest` (3589
passed), the CMake build, `ctest` (141 entries passed), the headless
self-test (472 passed), and the guard scripts.

## 1. Regression tests

- [x] 1.1 Extend `block_erases_a_hard_square_at_full_strength` in
  `crates/pictura-paint/src/eraser.rs`: dab at a half-pixel position
  (`20.5, 10.5`) and require exactly 16 document pixels a side — the pixels
  just outside the square untouched, which the inclusive boundary would have
  erased. Verify with `cargo nextest run -p pictura-paint block_erases`.
- [x] 1.2 Add `the_block_square_scales_with_the_zoom` in the same file: 400 %
  zoom covers 4 document pixels, 25 % covers 64, 3200 % covers exactly 1.
  Verify with `cargo nextest run -p pictura-paint the_block_square`.
- [x] 1.3 Add the `tst_eraser_tools` suite with
  `blockErasesASquareThatScalesWithTheZoom`, which drives the real canvas at
  400 % and counts the erased pixels, and
  `theBlockCursorIsASquareOfTheBlockSize`, which renders the canvas and
  measures the drawn outline at two zooms, allowing the cosmetic pen's
  one-pixel overhang. Register the suite in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`. Verify that the suite builds
  and that both cases fail against the current code.

## 2. The engine

- [x] 2.1 In `crates/pictura-paint/src/eraser.rs`, replace `BLOCK_SIZE` with
  `BLOCK_SCREEN_PX` (16 screen pixels) and add `block_side_doc_px(zoom)`, which
  treats a non-finite or non-positive zoom as 1.0 and clamps the result to the
  `StrokeConfig` diameter range. Verify with a unit test on the conversion
  itself.
- [x] 2.2 `begin_erase` takes the zoom and sets the Block's diameter from it.
  Verify with `cargo nextest run -p pictura-paint eraser`.
- [x] 2.3 In `crates/pictura-paint/src/tip.rs`, make the square tip's boundary
  half-open, and update the module doc, the `StrokeConfig::square` doc and the
  `ponytail:` note that record the old rule. Verify that
  `cargo nextest run -p pictura-paint` passes, including
  `square_tip_fills_its_corners` and the two stroke parity tests.
- [x] 2.4 The Block's arm overrides the brush preset's Scattering, dab count,
  and Size Jitter too, so a dab cannot cover a different number of pixels
  than the spec states. Verify with the shape-dynamics assertions in
  `the_block_square_scales_with_the_zoom`.

## 3. The bridge

- [x] 3.1 `begin_eraser` in `paint_tools.rs` takes the zoom and passes it to
  `begin_erase`; expose `block_side_doc_px` to C++ so the cursor and the erase
  share one rule. Verify that the crate builds and the generated bridge header
  carries the new parameter.
- [x] 3.2 `tool_eraser.cpp` passes `ctx.canvas()->zoom()`. Verify that the app
  builds.
- [x] 3.3 `tst_mask_edit.cpp`'s two `begin_eraser` call sites pass a zoom.
  Verify with `ctest --test-dir build -R '^tst_mask_edit' --output-on-failure`.

## 4. The Block cursor

- [x] 4.1 `ImageView` gains a square brush outline beside the circle, sharing
  the circle's state so only one is ever drawn, with a one-screen-pixel halo.
  Verify that `clearBrushOutline` still clears the square, and that the
  existing ring's rendering is unchanged.
- [x] 4.2 `ToolController` draws the block square in Block mode, gains the
  `refreshBrushOutline()` helper, and calls it when the Eraser's Mode changes,
  when the canvas zooms, and when the brush size changes. Verify with
  `tst_eraser_tools::theBlockCursorIsASquareOfTheBlockSize`.
- [x] 4.3 The cursor keeps the brush-size circle on a layer mask, where the
  Block dab paints the coverage with the brush tip: `eraserBlockMode()` asks
  the view whether a mask edit target is active. Verify that the circle is
  drawn there and the square only on a layer.
- [x] 4.4 Update the Eraser's Block ceiling in `docs/dev/STATE.md` (the two
  sentences that call the block a fixed document-pixel square and its ring the
  brush size). The commit carries `TASK-ALLOWS-DOCS`; verify with
  `bash scripts/guard.sh`.

## 5. Integration

- [x] 5.1 `cargo fmt --all` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.
- [x] 5.2 `cargo nextest run --workspace` passes.
- [x] 5.3 A CMake/Ninja build succeeds and
  `ctest --test-dir build -R '^tst_eraser_tools' --output-on-failure` passes.
- [x] 5.4 `bash scripts/check-file-size.sh`,
  `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`
  pass.
- [ ] 5.5 Archive the change with
  `openspec archive 2026-10-22-block-eraser-zoom -y`, so the pull request
  leaves no open change.
