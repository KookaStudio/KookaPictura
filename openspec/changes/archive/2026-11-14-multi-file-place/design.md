# Design

## Context

See `proposal.md` for motivation and the two spec deltas for the contract.

- `File > Place…` is a single-path handler: `getOpenFileName()` plus one
  `place_smart_object`/`place_image` call (`crates/pictura-app/cpp/frame_menus.cpp:158`).
  `getOpenFileNames()` already exists (`cpp/dialogs.cpp:826`) and drives the
  shipped multi-file Open flow (`cpp/frame.cpp:790`; the handler starts at 788).
- `openFileFilters()` (`cpp/dialogs.cpp:262`) has no `Images (…)` entry; the
  Place dialog must add one beside the Photoshop entries while keeping the rest
  of the list.
- Both place bridges are per-file and synchronous
  (`crates/pictura-app/src/cxxqt_object/impl_layers_smart_object.rs:60,93`).
  Each success selects the new layer and records one `"Place"` state.
- `place_image` consumes `open_policy_override` only after the read and decode
  succeed (`impl_layers_smart_object.rs:99-113`), so a failed raster can leave
  its profile choice for the next file.
- Free Transform is interactive. `ImageView::transformCommitRequested` and
  `transformCancelRequested` connect to
  `ToolController::commitFreeTransform`/`cancelFreeTransform`
  (`cpp/tools.cpp:664-667`). The options bar's `optionsTransformCancel` control
  calls `cancelFreeTransform()` directly without a key event
  (`cpp/options_bar_transform.cpp:321-327`). There is no public "session ended"
  signal, and `PicturaMainWindow::tools_` is private.
- `PicturaMainWindow::dispatch` does not check an action's enabled state
  (`cpp/commands.cpp:541`), so an enabled provider alone cannot prevent a
  re-entrant batch.
- File-size caps: `frame.h` is exactly 1200 lines, `frame_menus.cpp` 1191, code
  cap 1200. A new public member seam would force unrelated churn.
- `FrameDropRouter` already places multiple dropped files and transforms only
  the last (`cpp/file_drop_router.cpp:121-151`); that path stays unchanged.

## Goals / Non-Goals

**Goals:**

- Multi-file Place with one Free Transform session per placed file, settled
  before the next file is placed.
- Escape or the transform's Cancel control cancels the current placement (layer
  and `"Place"` state removed) and offers whole-batch vs next-item before
  continuing; any other non-interruption session end keeps the placement and
  continues.
- One failure summary per batch, excluding profile-warning, transform, and batch
  cancellations.
- Isolate each file's profile policy.

**Non-Goals:**

- Asynchronous placement/progress, a batch undo group, or a generic command
  queue.
- Changing the canvas-drop flow, Open, Open As, or Replace Contents.
- A reusable modal-restore framework; the dialogs stay in the Place module.

## Decisions

### A new `frame_place` module, not a `PicturaMainWindow` member

`frame.h` has no headroom and `frame_menus.cpp` cannot absorb the loop plus the
dialogs. Add `cpp/frame_place.{h,cpp}` with a free function
`placeDocuments(PicturaMainWindow&, const QStringList&)` using only public frame
methods, and list it in `cmake/sources/frame.cmake`. Alternative: put the loop in
`frame_import.cpp` (poor fit with its PDF/EPS scope) or free a line in `frame.h`
for a member seam (unrelated churn; no test needs it).

### Place dialog filter

Keep `openFileFilters()` as the Place filter list and add an `Images (…)` entry
beside the Photoshop entries (`cpp/dialogs.cpp`). "Preserve the existing filters"
means the existing list plus that already-specified Images entry, not a new
filter scheme.

### Sequential nested event loop per file

After each successful place, call `beginFreeTransform(created)`, then wait in a
local `QEventLoop` until `PictureView::transform_session_active()` is false. A
10 ms `QTimer` polls the session; the loop also exits when the view changes or
is destroyed (`QPointer`), or the application is closing. This matches the
existing nested loops used by dialogs and `waitForOpen` and keeps the check in
one place for synchronous, deferred, and cancelled commits.

Alternative: an asynchronous state machine driven by a transform-ended signal.
That would need a new signal/flag on `ToolController` (not publicly reachable
from the Place module) and stored batch state on the frame (no header room).
Mark the nested loop as the deliberate ceiling so a future progress/cancel
feature replaces it with a queue.

### Escape and Cancel detection scoped to the batch's ImageView and Cancel control

Watch the transform wait through a small local `QObject` latch. It records
`ImageView::transformCancelRequested`, which the ImageView emits only while its
transform is active, and the options-bar `optionsTransformCancel` control's
`pressed` signal (which fires before its `clicked` slot ends the session),
guarded by `transform_session_active()`. Escape in the tool-switch prompt,
another window, or a progress dialog cannot latch the flag because those events
are not delivered to the scoped targets. The normal ImageView key path and the
Cancel control still end the session; Enter and the apply control leave the flag
false and are treated as commits.

Alternative: a `ToolController` "last transform cancelled" flag. It is more
precise for tool-switch and progress-window cancels but needs a public accessor
from the frame or a new frame member (header cap). The scoped latch covers the
user-facing Enter/apply/Escape/Cancel contract; any other non-interruption
session end is a non-Escape end and keeps the placement.

### Escape or Cancel removes the placement, guarded by history

Record `placeState = view->history_index()` and confirm
`view->history_label(placeState) == "Place"` after a successful place. When the
wait ends with the Escape/Cancel flag set, undo only if the index still points at
that `"Place"` state and the label still matches: `view->undo()` removes the
layer and its state. If the user already undid it (`history_index < placeState`),
do not undo a second time; treat the item as cancelled. If `view->undo()` returns
false, surface the existing memory refusal, do not claim the placement was
cancelled, and stop the batch without the cancel/continue prompt. If the user
committed an identity transform, the index is still at the Place state but the
flag is false, so the layer is kept and the batch continues.

Alternative: keep the layer on Escape (current contract) — rejected by the user;
the spec deltas carve the menu Place flow out of the drop rule.

### Non-Escape, non-interruption session ends keep the placement

A tool switch choosing Don't Apply, or any other non-interruption session end
that leaves the `"Place"` state, keeps the placed layer and its state and
continues with the next file. The only removals are Escape and the transform's
Cancel control. History navigation that already removed the `"Place"` state is
treated as cancelled without a second undo.

### Cancel/continue prompt, then one summary

The Escape/Cancel prompt is a `QMessageBox` with `Cancel the whole Place` and
`Continue with the next item` (object names for tests). Closing it cancels the
whole Place. It is shown only when paths remain; Escape or Cancel on the last
item ends silently. File failures are collected across the batch and shown once
after the last session settles. Profile-warning cancels, transform cancels,
Escape/Cancel cancellations, an interrupted batch, and an abandoned batch are
never collected.

### Per-file profile policy isolation

Move the `open_policy_override.take().unwrap_or(color_policy)` block to the top
of `place_image`, before `read_capped`/`decode_import`. A failed file then
consumes its own decision and cannot leak it to the next file. No API change.

### Narrow enablement plus handler, entry, and re-entry guards

Give `FilePlace` its own provider:
`activeView() && activeView()->has_document() && !activeView()->transform_session_active()`.
The command handler checks the same condition before showing the file dialog, so
a dispatch during an active transform never opens the Place dialog. Because
dispatch ignores enabled state, `placeDocuments` rechecks at entry that the
active view exists, has a document, and has no active transform session, then
keeps a per-window "batch running" set and returns immediately on re-entry. The
provider is refreshed by the existing transform lock, so the menu item disables
during each session.

### Tests extend the existing multi-file suite

Add Place cases to `cpp/tests/tst_multi_file_open.cpp`; reuse its offscreen file
dialog probe and add a timer-driven helper that handles the profile warning, each
transform (commit, apply, Escape, and the Cancel control), the tool-switch
prompt, interruptions, the Escape prompt, and the summary. Tests include
`multiSelectPlacesOneLayerPerFile`, `eachPlaceWaitsForThePreviousTransform`,
`mixedFormatsPlacePerFile`, `applyControlCommitsAndAdvances`,
`failedFileDoesNotStopTheRestAndIsSummarized`, `allFilesFailShowsOneSummary`,
`cancelledProfileWarningSkipsWithoutSummary`, `placeUnavailableWithoutDocument`,
`placeUnavailableDuringTransform`, `failedPlaceDoesNotReuseTheProfilePolicy`,
`emptySelectionStartsNoBatch`, `dispatchDuringBatchIsIgnored`,
`placeDialogOffersImagesFilter`, `escRemovesThePlacementAndHistory`,
`escContinuePlacesTheNextFile`, `escCancelWholeStopsTheBatch`,
`escOnLastItemEndsWithoutDialog`, `cancelControlRemovesThePlacement`,
`escapeInToolSwitchPromptDoesNotCancelThePlace`,
`toolSwitchDontApplyKeepsThePlacement`,
`identityCommitAdvancesWithoutHistory`, `documentSwitchAbortsTheBatch`, and
`closingDocumentAbortsTheBatch`. No new test target and no production seam.
Alternative: a new suite plus a public `placeDocuments` seam — more files,
CMake churn, and a header cap fight for no coverage gain, since the real dialog
is already proven testable offscreen.

## Risks / Trade-offs

- **Nested-loop reentrancy** (user opens another document, triggers Place again,
  or closes the document or window) → entry and per-window guards, abort on
  active-view change or view destruction, and no summary after an interruption.
- **Escape/Cancel detection is scoped, not global** → the latch watches the
  batch's active ImageView and the Cancel control; Escape in the tool-switch
  prompt, another window, or a progress dialog cannot latch the flag. A
  progress-window cancel ends the session as a non-Escape cancel
  (`ToolController::commitFreeTransform` cancels it after the job reports no
  commit), so the wait returns and the batch keeps the placement and advances.
- **Undo/Redo are not transform-locked** → if the user undoes the Place during
  the transform, the missing history state is treated as a cancelled item and no
  second undo runs. If the Escape/Cancel rollback fails, the batch stops with the
  memory refusal and does not claim a cancellation. A redo may be treated as a
  completed item.
- **Single-file Place behaviour changes** → Escape or the Cancel control now
  removes the layer instead of keeping it. This is intentional (spec delta);
  every other non-interruption session end keeps it, and the canvas-drop rule
  stays keep-on-cancel.
- **File-size ceilings** → the loop lives in a new file; keep the new file small
  and do not grow `frame_menus.cpp` beyond its current handler removal.
- **Synchronous per-file decode** → a large file can block briefly before its
  transform appears. Accepted: the user already gets an interactive session per
  file, and async placement is out of scope.

## Migration Plan

No stored data or schema changes. The only compatibility changes are the menu
Place Escape/Cancel removal and the non-Escape keep-and-continue rule, documented
in the spec and long-form docs. Rollback is a plain revert of the change; no
migration step is required.

## Open Questions

None. The interaction, cancellation, summary, and test decisions are settled in
the proposal and spec deltas.
