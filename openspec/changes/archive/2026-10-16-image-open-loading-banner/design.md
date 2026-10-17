# Design

## Where the time goes

`PictureView::open` (PSD/PSB) and `PictureView::open_image` (raster) run
`std::fs::read`, the decode, `Document::from_rgba`, the first composite, and the
initial history capture synchronously on the GUI thread. For a large image that
is long enough that the event loop never returns to paint, so a blank or stale
window reads as a hang. There is no progress to report from inside the engine
call, so the frame reports *that an open is running* before it starts.

## The banner

A single `QLabel` child of the window, built lazily on the first open:

- `objectName` `documentOpenBanner`, `WA_TransparentForMouseEvents` so it never
  eats a click, word-wrapped with a 480 px cap so a long file name does not run
  off the window.
- Styled by the `QLabel#documentOpenBanner` rule in `theme.cpp`, reusing the
  existing notice tokens (`surface-notice` yellow, `content-notice` dark) with a
  6 px `border-radius` and padding. No new colour token: the notice pair is
  already the app's "dark text on yellow" info style.

Two private helpers on `PicturaMainWindow`, defined in the new `frame_open.cpp`:

- `beginOpenIndicator(path)`:
  - Build the label if needed, set its text to `Opening <file name>…`, size and
    position it, `show()` and `raise()` it.
  - `QApplication::setOverrideCursor(Qt::WaitCursor)`.
  - `repaint()` the banner then
    `QApplication::processEvents(QEventLoop::ExcludeUserInputEvents)` so it is on
    screen before the blocking call. User input is excluded so the pumped events
    cannot start a second open.
  - `emit documentOpenStarted(path)` — the test seam, emitted after painting and
    immediately before the load.
- `endOpenIndicator()`: `hide()` the banner, then
  `QApplication::restoreOverrideCursor()`.
- `positionOpenBanner()`: pin the banner 12 px in from the window's left, just
  above the status bar, so it reads as a lower-left toast. Called on show and
  resize (`frame_columns.cpp`) so it stays put while the window changes.

Each open entry point brackets its `document_api::…` call:

```
beginOpenIndicator(path);
const bool loaded = document_api::open(*view, path);   // blocks
endOpenIndicator();
```

`openPath` then shows the open notice (if any) in the status bar after
`endOpenIndicator`; `openAsSmartObjectPath` keeps its existing refusal message,
also shown after.

## Test seam

`documentOpenStarted(const QString& path)` is emitted synchronously while the
banner is showing. A Qt Test connects a lambda that records
`openBannerTextForTest()` and `openBannerVisibleForTest()` at that instant — the
only moment the transient banner state is observable, since the load that
follows blocks and the banner is hidden before the call returns. The styling is
guarded separately by `tst_design_system::openBannerUsesTheNoticeStyle`, which
reads the `#documentOpenBanner` rule from the built stylesheet.

## Ceilings (`ponytail:`)

- Synchronous: the banner is static and the window is not interactive during the
  load. A worker-thread open (docs `ARCH-005`) would animate it and keep the UI
  live; that is a separate, larger change.
- Revert (`revertActive`) and Edit Contents (`editSmartObjectContents`) reload
  through `document_api` directly and are not covered; they are reloads of an
  already-open document, not the Open commands the issue names.
- The banner is a static label, not a progress bar: the engine exposes no open
  progress to drive one.

## File-size note

`frame.cpp` sat at 1197 LOC against the 1200 hard cap, so the open methods moved
verbatim into `frame_open.cpp` (a pure move) and the banner helpers live there
too.
