# Proposal

## Why

The loading tab shows an animated spinner, but a large open can spend a long
time in one opaque worker call with no sense of how far it has got. The tab's
empty workspace has room for a small progress card, and the worker already knows
the major phases (read, decode, composite, prepare). This is a phase-only
follow-up to #729, folded into the same issue and pull request.

## What Changes

- The loading tab's empty workspace shows a centered card with the file name, the
  current phase, a determinate progress bar, the percentage, and a Cancel
  button.
- The worker reports phase boundaries through a new `openProgress` signal:
  `Reading file`, `Decoding`, `Compositing`, `Preparing layers`, and, for PDF/EPS
  imports, `Rendering page` / `Rendering image` / `Rendering EPS`.
- The percentages are **phase weights**, not measured work. There is no
  sub-progress inside the codec, Qt's raster decoder, or Ghostscript; the bar
  moves at phase boundaries and the tab spinner shows liveness in between.
- The card's Cancel cancels exactly like the tab spinner and middle-click. The
  card disappears when the tab is filled or removed.
- A generic PDF probe and the Import PDF dialog show no card; the card appears
  when a chosen page/image render starts.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `document/document-lifecycle`: add "The loading tab shows phase progress"
  beside the existing loading-tab requirements.

## Impact

- Rust: `open_jobs` decode/import helpers take a phase callback; the worker
  queues a throttled `openProgress` signal; `PictureView` gains the signal.
- App C++: new `LoadingCard` widget, the loading page becomes a stacked
  placeholder/canvas page, `frame_open.cpp` wires progress and the card's
  Cancel, `theme.cpp` styles the card, `CMakeLists.txt` registers it.
- Tests: `tst_loading_tab` gains the card/phase/cancel cases; a Rust test
  asserts the phase sequence.
- Docs: the document-lifecycle workflow records the card and that its
  percentages are phase weights. The docs commit carries `TASK-ALLOWS-DOCS`.
- No new dependencies, no engine behavior change: only phase callbacks around
  the existing calls.

## Non-goals

- Measured sub-progress inside the PSD codec, Qt raster decoding, or
  Ghostscript. Those stay phase-only.
- A modal task dialog: the card lives in the loading tab, matching #729's
  tab-only design.
- Progress for Revert, Edit Contents, or synchronous wrappers: they have no
  loading tab.
