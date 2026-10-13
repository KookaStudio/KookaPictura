# Proposal

## Why

Issue #311: after releasing a Rectangular or Elliptical Marquee the ants appear
only after a visible lag, and dragging a selection from inside is janky. On an
8000×6000 document (release build) the outline tracer took ~490 ms for a
rectangle and ran twice per commit (`changed` and `selectionCommitted` both
refresh the overlay); building the shape scanned every document pixel (rect
~96 ms, ellipse ~204 ms); an unfeathered shape was cloned by `feather(0)`
(~130 ms); and every drag move translated the mask pixel by pixel (~189 ms) and
then retraced it.

## What Changes

- **Run-based outline.** `pictura_select::contour` reduces each row to its
  inside runs (an identical row reuses the previous runs), derives directed
  boundary edges from run differences, and drops collinear vertices. The drawn
  edge set is unchanged; a rectangle is now four corners.
- **Bounded shapes.** `Selection::rect` fills only its clipped rows and columns;
  `Selection::ellipse` finds each row's single run in closed form and settles
  its ends with the unchanged per-pixel test. Output is byte-identical.
- **Row-copy translate.** `Selection::translate` copies row slices.
- **No clone for no feather.** Marquee, lasso, and path-to-selection shapes pass
  through unchanged at feather 0; a shape combined into no selection is used
  as-is for New/Add.
- **Offset ants during a move drag.** The canvas draws the committed contour at
  the drag offset instead of retracing the translated mask per move; release
  and cancel drop the offset. The mask itself still moves live.
- **BREAKING**: none.

## Capabilities

### New Capabilities

- `tools/selection-outline-performance`: the outline tracer's exactness and
  cost, the bounded shape fills, and the offset ants during a move drag.

### Modified Capabilities

None.

## Impact

- `crates/pictura-select`: `src/contour.rs`, `src/lib.rs` (`rect`, `ellipse`),
  `src/translate.rs`.
- `crates/pictura-app`: `src/cxxqt_object/{helpers,impl_selection,path_list,tests_profiles}.rs`;
  `cpp/image_view.{h,cpp}`, `cpp/tools.h`, `cpp/tools_selection_move.cpp`,
  `cpp/frame_build.cpp`; new Qt suite `cpp/tests/tst_selection_move.cpp`.
- No new dependencies.
