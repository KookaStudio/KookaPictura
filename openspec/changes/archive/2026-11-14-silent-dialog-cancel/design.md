# Design

## Context

`PicturaMainWindow::reportFilterRefusal` shows the status bar's banner. The
Image ▸ Adjustments dialog handler chains the readiness check and the dialog
result in one condition, so a rejected dialog falls into the `else` branch and
reports a refusal. HDR Toning has the same shape. Filter dialogs already
separate the two: `applyFilterCommand` reports only when the commit fails.

## Goals / Non-Goals

**Goals:**

- Make Cancel silent for adjustment and HDR Toning dialogs.
- Keep pre-dialog and commit refusals reported.
- Lock the behavior with Qt Test.

**Non-Goals:**

- Changing pixel/history semantics on cancel (already correct).
- Changing the banner or its styling.
- Reporting failures from an adjustment commit that accepts (unchanged).

## Decisions

### Separate readiness from the dialog result

The handler checks `filter_target_ready` first and reports only there. It then
runs the dialog and refreshes only when the result is true. A false result from
`runAdjustmentDialog` is the user's Cancel; `image_adjustment_default` is
non-empty for every catalogue dialog kind, so the empty-block path is not
user-reachable. This avoids a tri-state return type for one caller and keeps
the existing `bool` contract. HDR Toning is restructured the same way.

### Tests use the menu path

The existing adjustment tests construct dialogs directly, which never runs the
menu handler. The new tests trigger the registry action, reject the top-level
dialog from a zero-delay timer (as the filter suites do), and assert the status
bar's `currentMessage()` is empty. The filter test does the same through the
Filter menu, and the existing empty-blur test remains the counter-case that a
real refusal still reports.
