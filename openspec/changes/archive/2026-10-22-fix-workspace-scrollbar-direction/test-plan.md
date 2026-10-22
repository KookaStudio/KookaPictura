# Test Plan

The scenarios are the workspace scrollbars' projection: the direction the
handle moves the canvas, the document's ends at the bar's ends, and the reveal
margin the bar has to cover.

The self-test's `canvas_scrollbar_roundtrip` check (exit code 309) is updated
by this change and covers the same round trip as
`tst_canvas_pan::scrollbarRangeFollowsThePanRange`;
`check-test-plan.py` cannot name a self-test check, so the Qt Test case is
the row's coverage and the self-test is the second line of defence.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lcs_direction` | Raise the horizontal bar's value, then the vertical bar's value, and read both offsets | The offset falls on each axis, so the content travels against the handle | `tst_canvas_pan::scrollbarHandleMovesAgainstTheContent` |
| `lcs_ends` | Set the horizontal bar to its minimum and to its maximum, reading the offset each time, and compare it with the shared range helper | At the bar's minimum the offset is the range's maximum, so the start of the document is visible; at its maximum the range's minimum, so the end is visible | `tst_canvas_pan::scrollbarRangeFollowsThePanRange` |
| `lcs_margin` | Pan the canvas past the document into the reveal margin and read the bar, then set the bar's value back | The bar's value moves with the canvas into the margin and reaches the end of its track only at the pannable limit, and setting the value brings the canvas back with it | `tst_canvas_pan::scrollbarRangeFollowsThePanRange` |
