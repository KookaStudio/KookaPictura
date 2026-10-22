# Tasks

## 1. Tests first

- [x] 1.1 Author `clipToCanvasClosesAlongTheBorder`, `clipToCanvasLeavesInnerShapesAlone`, `clipToCanvasDropsOuterShapes`, and `clipToCanvasKeepsDegenerateLoops` in a new `crates/pictura-app/cpp/tests/tst_selection_preview.cpp`, and register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the file and the CMake list carry every name and that `test-plan.md` names the same cases
- [x] 1.2 Author `marqueePastTheEdgeClosesAlongTheBorder`, `ellipticalMarqueePastTheEdgeClosesAlongTheBorder`, `lassoPastTheEdgeClosesAlongTheBorder`, `magneticLassoPastTheEdgeClosesAlongTheBorder`, `regionTracePastTheEdgeClosesAlongTheBorder`, `liveOutlineMatchesTheCommittedOne`, `polygonalBandStaysOpenAtTheBorder`, `combineModeShowsTheNewShape`, and `zoomMarqueeStaysUnclipped` in the same suite, each driving the real tool handler through the canvas; verify every name in `test-plan.md` exists in the file
- [x] 1.3 Write the shared edge-set comparison the cases use: the clipped loop and the committed contour are compared as unit-lattice edge sets, not as vertex lists, because the clipped loop may repeat its first point; verify the comparison helper is used by `liveOutlineMatchesTheCommittedOne`

## 2. The outline clip

- [x] 2.1 Add `clipSelectionShapeToCanvas(loops, canvas)` to `crates/pictura-app/cpp/selection_geometry.{h,cpp}`: intersect each loop with the canvas rectangle as a `QPainterPath` and return the result's subpaths; verify the declaration and definition agree and that no Qt widget type enters the header
- [x] 2.2 Return a loop with fewer than three points unchanged, and return a loop whose bounding rectangle lies inside the canvas unchanged; write the containment test as an explicit comparison, not `QRectF::contains`, which is false for a null rectangle; verify by inspection that a one-point preview survives
- [x] 2.3 Drop a loop that lies wholly outside the canvas, and keep the result's subpaths separate so a hole keeps its own outline; verify `clipToCanvasDropsOuterShapes` and `clipToCanvasClosesAlongTheBorder` pass

## 3. The canvas applies the clip

- [x] 3.1 Add `ImageView::setSelectionShapePreview(loops)` to `crates/pictura-app/cpp/image_view.{h,cpp}`: a closed, dashed band clipped to the document rectangle, and an accessor for the preview loops the tests read; verify the setter stores a closed, non-solid band and calls `updateAntsTimer()` so the live ants keep animating
- [x] 3.2 Re-word the `ponytail:` marker in `crates/pictura-app/cpp/image_view.cpp` so it names the move-selection drag's offset outline, the case it describes, and the requirement that specifies it; verify the marker no longer reads as a note about the marquee

## 4. The tools ask for the clip

- [x] 4.1 Route the rectangular and elliptical marquee previews in `crates/pictura-app/cpp/tool_selection.cpp` through `setSelectionShapePreview`; verify the two call sites are the only marquee previews left on `setSelectionPreview`
- [x] 4.2 Route the freehand lasso preview in `crates/pictura-app/cpp/tool_selection.cpp` through `setSelectionShapePreview` and leave the polygonal lasso's open band on `setSelectionPreview`; verify the polygonal band still passes `false` for closed and `true` for solid
- [x] 4.3 Route the magnetic lasso preview in `crates/pictura-app/cpp/tool_magneticlasso.cpp` through `setSelectionShapePreview`, keeping `setSelectionPreviewOrigin` for the origin marker; verify the origin marker is unaffected
- [x] 4.4 Route both region-trace previews in `crates/pictura-app/cpp/tool_region_drag.cpp` through `setSelectionShapePreview` and leave the moved outline on `setSelectionPreview`; verify the moved outline keeps its current geometry
- [x] 4.5 Leave the zoom marquee in `crates/pictura-app/cpp/tool_hand_zoom.cpp` and the red-eye ring in `crates/pictura-app/cpp/tool_redeye.cpp` on `setSelectionPreview`; verify no preview call site outside the selection tools asks for a clip

## 5. Close-out

- [x] 5.1 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify every scenario row is covered, every named test exists, and the change validates
- [x] 5.2 Archive the change with `openspec archive 2026-10-21-marching-ants-canvas-edge` and re-run `openspec validate --all --strict`; verify no non-archived change remains, because `scripts/verify-fast.sh` runs `scripts/check-openspec-archived.sh` and fails on an open change
- [x] 5.3 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `bash scripts/verify-fast.sh`; verify they pass, that `scripts/check-file-size.sh` accepts the touched files, and that the app self-test cases `tsc_ellipse_preview` and the magnetic-lasso preview case still pass: the build is 836/836, the Qt layer 141/141, the self-test 472 passed, and `verify-fast.sh` reports 5266 passed, 29 skipped, 0 failed

## 6. Review fixes

- [x] 6.1 Fix the test suite's `QPolygonF(QPointF)` construction, which Qt 6 does not have, so the suite compiles; verify the construction against Qt 6.11.1
- [x] 6.2 Keep the trace when the clipped region is empty, so a stroke that doubles back on itself keeps its band instead of vanishing; verify the new case
- [x] 6.3 Test the "wholly outside" case with an explicit comparison, because `QRectF::intersects` is false for a flat bounding box and a horizontal stroke was dropped as if it lay outside; verify the flat-stroke case
- [x] 6.4 Correct the design's D4 and the risk register, which claimed the raw trace came back from the intersection, and add the self-overlapping shape to the requirement; verify the archived delta and the merged spec agree
- [x] 6.5 Drop the polygonal test's waits: the close rule is the distance to the last press, and the clicks are 50 px apart; verify the comment states the rule
