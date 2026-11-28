# Tasks

## 1. Protractor

- [x] 1.1 Write `tst_ruler_tool::protractorCreate`, `tst_ruler_tool::protractorDouble`, and `tst_ruler_tool::protractorClear` covering the planned gestures and readouts.
- [x] 1.2 Extend the ruler view state, bridge, tool gesture, overlay and readouts; verify the protractor cases.

## 2. Straighten

- [x] 2.1 Write `tst_ruler_tool::straighten` and `tst_ruler_tool::straightenNoop` covering rotation, undo and refusal.
- [x] 2.2 Wire Straighten to the guarded document rotation path; verify both cases.

## 3. Origin

- [x] 3.1 Write `tst_rulers_guides::originDrag`, `tst_rulers_guides::originUnsnapped`, and `tst_rulers_guides::originViews` covering the planned origin behavior.
- [x] 3.2 Implement intersection drag, origin-relative ruler and Info coordinate readouts, with unsnapped origin placement; verify origin cases and existing ruler cases.
