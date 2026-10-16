# Test Plan — mask-paint-performance

One row per spec scenario in `specs/compositing/layer-masks/spec.md`.
Coverage names an automated test (`<crate>::<fn>` for Rust, `tst_<suite>::<case>`
for Qt Test) or `manual: <reason>`.

| Scenario | Steps | Coverage |
|---|---|---|
| A dab writes the coverage plane directly | Paint a mask dab and assert the edit document is a single coverage channel whose only change is the dab's coverage byte | `pictura-render::a_coverage_document_is_single_channel`, `pictura-render::coverage_and_rgba_documents_paint_the_same_coverage` |
| A large mask brush uses the accelerated path | Target a mask and stroke a brush above the raster budget; assert the reduced-level / GPU path runs in place of the exact per-dab CPU raster | `pictura-render::gpu_mask_stroke_matches_the_cpu_oracle` |
| The committed mask matches the exact raster | Commit a GPU mask stroke and compare its coverage against a full-resolution exact CPU raster of the same sample log (±1 unit) | `pictura-render::gpu_mask_stroke_matches_the_cpu_oracle` |
| The per-dab present stays region-bounded | Paint a mask dab and assert only the dab's region is recomposited and presented | `tst_mask_edit::maskStrokePresentsWhileDragging` |
| A live mask stroke renders while dragging | Begin a mask stroke, paint several dabs, and assert the canvas presents the accumulated region (a live mask stroke counts as painting) | `tst_mask_edit::maskStrokePresentsWhileDragging` |

## Manual rows

None. The profile helper `pictura-app::mask_dab_profile_4000` (`#[ignore]`) is a
measurement aid, not a scenario check.
