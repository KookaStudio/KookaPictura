# Tasks

## 1. Oracle tests

- [x] 1.1 Add Rust tests for the six pinned traces, empty/one/two-point traces, invalid canvases, non-finite input, holes in both input orders, exact-endpoint joining, near-miss separation, degenerate arity, duplicate closing vertices, border-collinear edges, open zero-area bands, and large-coordinate precision; verify preserved paths exactly and filled intersections topologically.
- [x] 1.2 Extend the six existing Qt clipping cases with a local copy of the complete old clipping function; use exact ordered equality for fast paths and intersection-split undirected boundary segments with a `1e-6` tolerance for filled intersections.
- [x] 1.3 Add a Qt Test teeth case for the `1e-6` vertex tolerance; verify `0.5e-6` passes and `2e-6` fails.
- [x] 1.4 Add a deterministic Qt differential corpus covering multiple loops, holes in both input orders, duplicate closing vertices, border-collinear edges, near touches, tangencies, and an open zero-area band.

## 2. Engine clipping

- [x] 2.1 Pin `i_overlay` 9.2.0 with default features disabled in `pictura-select` and update the lockfile; verify the direct dependency and four MIT support packages are recorded.
- [x] 2.2 Add `pictura-select::clip` with invalid-canvas and loop fast paths, even-odd i64 intersection, closed/nonzero-area empty-result fallback, open zero-area string clipping, arity filtering, pre-snap exact-endpoint splicing, exact border snapping, and canvas clamping; verify the Rust tests cover every branch.
- [x] 2.3 Export the engine API from `pictura-select`; verify the bridge can call it without Qt types.

## 3. C++ and Rust bridge

- [x] 3.1 Add `SelectionClipResult { valid, loop_lengths, points }`, the flat `(x, y)` bridge function, and its verb-manifest row; verify even point counts, checked count sums, exact consumption, remaining-point bounds, and malformed-payload rejection with tests.
- [x] 3.2 Add the Qt adapters for clipping, marquee geometry, and the close-radius test in `cpp/api/selection_api.{h,cpp}` through the existing include-only `selection_bridge.h`; preserve empty and one-point loops, copy both result vectors before destruction, and return no loops for `valid = false`; verify malformed-payload rejection in Rust and owned-result copying by source review.
- [x] 3.3 Replace the production boolean in `selection_geometry.cpp` with adapter delegation; include only `api/selection_api.h`, keep `pictura::clipSelectionShapeToCanvas` as the only shell entry point, and keep generated headers outside non-adapter files.

## 4. Source review and change bookkeeping

- [x] 4.1 Review the complete diff against `origin/master` for topology, tolerance, bridge lifetime, dependency licensing, CMake registration, file sizes, and test-plan accuracy; verify all legitimate findings are fixed.
- [x] 4.2 Mark implementation tasks complete and regenerate `THIRD-PARTY-LICENSES`; verify `git status` shows only intended uncommitted files.

## 5. Sequential verification

- [x] 5.1 Run `cargo fmt --all`; run `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura-select 'clip::tests'` with 16/16 passing, and `bash scripts/with-build-lock.sh -- cargo nextest run -p pictura_app 'selection_clip_shape_to_canvas'` with 2/2 passing.
- [x] 5.2 Run `bash scripts/with-build-lock.sh -- bash -c 'cmake -S . -B build -G Ninja && cmake --build build --parallel'`, fix the C++ `::rust` qualification error, rebuild, then run the final full build with `bash scripts/with-build-lock.sh -- cmake --build build --parallel`; run `bash scripts/with-build-lock.sh -- env QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_selection_preview$' --output-on-failure` with 33/33 Qt cases passing.
- [ ] 5.3 Strict OpenSpec validation passed with 527/527 items, and the change was archived to `openspec/changes/archive/2026-11-24-1252-clip-selection-shape-to-canvas/`. The final `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh` run passed fmt and Clippy, then reported 11,741 passed, 150 skipped, and one failure: `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`. A clean `origin/master` worktree reproduced the same single failure under `LC_ALL=C`; the host-locale run also reproduced the same unrelated numeric-locale failures on both trees. The aggregate gate remains baseline-red, not passed; proceed to commit and PR without changing unrelated tests.
