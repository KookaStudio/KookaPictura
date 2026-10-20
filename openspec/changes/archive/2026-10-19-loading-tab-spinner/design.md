# Design

## Why a worker now

`PictureView::open` / `open_image` / `open_as_smart_object`
(`crates/pictura-app/src/cxxqt_object/impl_core.rs`) and `pdf_open`
(`pdf_export.rs`) read the file, decode, composite, and capture history in one
call on the GUI thread. The #667 banner could only paint before the call; the
event loop never ran during it. Issue #729 requires an animated spinner and live
hover/close/middle-click, so the work has to leave the GUI thread.

The existing filter commit path (`filter_tools/jobs.rs`, `filter_commit.cpp`)
already proves the pieces: a worker owns a cloned/independent document, returns
owned data, and the GUI thread applies it. This change follows that shape but
does **not** pump a nested loop in production: the open entry points return and
the result arrives through a signal.

## Worker and completion signal

`PictureView` gets `impl cxx_qt::Threading` and two signals:

```rust
#[qsignal] fn open_finished(self: Pin<&mut Self>, status: QString);
#[qsignal] fn open_detected(self: Pin<&mut Self>);   // PDF only
```

A new `open_jobs.rs` bridge (own module, like `filter_tools/jobs.rs`) exposes
free functions that take `Pin<&mut PictureView>`:

- `open_psd_start(path)`, `open_image_start(path)`,
  `open_smart_object_start(path)`
- `open_pdf_start(path, password)` (detects editing data, then decodes)
- `open_pdf_page_start(path, password, page, area, target)`
- `open_pdf_image_start(path, index, target)`
- `open_eps_start(path, target)`
- `open_cancel()` (sets the view's cancellation flag)

Each start captures only `Send` data from the view — `String`s, `bool`s, and a
Rust-owned copy of `ImportTarget` — then calls `view.qt_thread()` and spawns
`std::thread`. The worker never touches the view. When the work finishes it
queues a closure with `CxxQtThread::queue`; the closure runs on the GUI thread,
checks the cancellation flag, applies the result, and emits `open_finished` with
one of:

- `"ok"`
- `"none"` / `"password"` / `"wrongpassword"` (PDF probe)
- `"error:<message>"`

`queue` returns `Err` when the view was destroyed (the tab was cancelled), which
drops the result. That is the cancellation mechanism; the worker itself keeps
running (non-goal: interrupting the decode).

The PDF worker queues `open_detected` as soon as `pdf::editing_psd` reports
embedded Pictura data, before the PSD decode. The frame creates the loading tab
on that signal, so the tab exists for the decode but not for a generic PDF's
probe. `open_finished` follows on the same queued path, so the two arrive in
order.

## Applying a result

The decode half returns an owned value:

```rust
struct OpenedDocument {
    doc: Document,
    history: History,
    active_layer: Option<String>,
    path: Option<String>,
    source_format: String,
    pdf_password: Option<String>,
}
```

`apply_opened(rust, opened)` runs on the GUI thread:

1. `rust.doc = Some(opened.doc)`
2. `rust.reset_edit_state()` (clears selection/history/transients, rebuilds the
   pyramid from the already-composited document)
3. restore `history`, `active_layer`, `path`, `source_format`, `pdf_password`
4. `dirty = false`

The synchronous methods (`open`, `open_image`, `open_as_smart_object`,
`pdf_open`, `pdf_import_open_page`, `pdf_import_open_image`, `eps_import_open`)
are re-expressed as `decode/render` + `apply_opened` on the calling thread, so
Revert, Edit Contents, the self-test, and the existing tests keep their
behaviour and share one implementation.

## Loading tab lifecycle

`DocEntry` gains:

```cpp
bool loading = false;
quint64 openToken = 0;
QWidget* page = nullptr;      // the tab page; == canvasHost for ready docs
```

A loading entry has a `view` with no document, a placeholder `page` (a plain
widget with the canvas colour), and no `canvas` / `canvasHost`. `docs_` stays in
lockstep with the tab order: the loading entry is appended before the tab, and
`tabMoved` keeps moving it.

`beginLoadingDocument(view, path, displayName, token)`:

- applies the persisted GPU/colour preferences to the view
- creates the placeholder page and appends the `DocEntry`
- adds the tab titled `documentName(index)` (file name; import display name)
- installs a `TabSpinner` as the tab's right-side button
- connects `open_detected` / `open_finished`
- calls `refresh()` so the tab bar and empty workspace appear
- does **not** change the current tab when another document is active

`finishLoadingDocument(index)` extracts the canvas creation and signal wiring
from `addDocument` into shared helpers, parents the new `CanvasScrollBars` in
the placeholder page's layout, restores the default close button, remembers the
recent path, and activates the tab unless the user selected another tab during
the load.

`removeDocument` is made null-safe for a loading entry (no canvas) and deletes
`entry.page` once, which owns the canvas host when it exists.

## Spinner widget

`TabSpinner` (`tab_spinner.{h,cpp}`) is a small `QWidget` painted with an arc
on the theme foreground, rotated by a `QTimer` (80 ms). `enterEvent` /
`leaveEvent` toggle a hover state that paints the existing `panel.close` icon
instead. A left click emits `cancelRequested()`. The widget exposes
`angleForTest()` and `hoveringForTest()` so Qt Test can assert animation and the
hover swap without pixel comparisons. A tooltip names the file being opened.

The spinner is installed with `QTabBar::setTabButton(index, RightSide, spinner)`.
On finish it is replaced with `nullptr`; the test asserts the default QSS close
button is back (if Qt does not restore it, `finish` installs a close button
widget connected to `tabCloseRequested`).

## Middle-click

`PicturaMainWindow` installs an event filter on `tabs_->tabBar()`. A middle
button press on a tab whose entry is loading calls `cancelOpen(index)` and
consumes the event. Any other tab is passed through untouched, so normal
documents keep today's behaviour (none).

## Cancellation

`cancelOpen(index)`:

1. calls `view->open_cancel()` (the queued result is dropped even if the view
   is still alive)
2. takes the token
3. `removeDocument(index)` (the tab disappears immediately)
4. `endOpenCursor()`
5. emits `documentOpenFinished(token, Cancelled, {})`

No error is shown. If the worker has already queued its result, the flag drops
it; if the view is already destroyed, `queue` fails.

## Busy cursor

`openCount_` is incremented by each start and decremented by each finish /
cancel / failure. The first increment calls
`QApplication::setOverrideCursor(Qt::BusyCursor)`, the last decrement calls
`restoreOverrideCursor()`. Modal PDF/EPS dialogs temporarily release the
override so the dialog does not show a busy cursor; the counter is resumed
after the dialog.

`Qt::BusyCursor` is the platform busy cursor. On X11/Wayland that is a watch,
not an animated spinner; the per-tab spinner is the animation. A custom
animated cursor is not built (ceiling).

## PDF flow

`startOpenPdfPath(path, interactive)` creates a temporary `PictureView` (not in
a tab), allocates a token, and starts `open_pdf_start`.

- `open_detected` -> `beginLoadingDocument(view, path, fileName, token)`. The
  decode continues on the worker and lands in that tab.
- `"ok"` -> normal success.
- `"none"` -> delete the temporary view; run the existing probe / password /
  Import PDF dialog synchronously (with a wait cursor, no tab); build a
  `PdfImportPlan` that copies the dialog's name, mode/depth/ppi/per-cm,
  anti-alias, page list, per-page pixel size and area, and suppress-warnings
  flag; start the first page render.
- `"password"` / `"wrongpassword"` -> delete the temporary view, prompt (unless
  non-interactive), then retry with a fresh temporary view or finish
  `Cancelled` / `Failed`.

Page renders run one at a time in dialog order. Each gets its own loading tab
and token. A success fills it; a failure removes it, reports the existing error,
and stops the flow (finishing `Opened` if an earlier page opened, matching the
old return value). Cancelling one page cancels the rest of the flow; documents
already opened stay open. The warnings box and `rememberRecent` run when the
flow ends.

## EPS flow

The bounding-box probe and Rasterize Generic EPS Format dialog stay
synchronous. After OK the frame creates a loading tab and starts
`open_eps_start`; completion follows the normal success/failure path.

## Tab activation

Each loading entry records `userMovedAway`. The `currentChanged` handler sets it
when the current tab becomes a different tab while an entry is loading, unless
the change is programmatic (`suppressTabTracking_` is set around the frame's own
`setCurrentIndex` calls). On success the frame activates the tab only when
`userMovedAway` is false.

## Reentrancy guards

- Concurrent opens are allowed: every entry has its own view, worker, token, and
  tab. The cursor counter handles the shared cursor.
- `closeDocument` on a loading entry cancels it instead of asking to save.
- `closeEvent` and `aboutToQuit` call `cancelAllOpens()`, which cancels every
  loading tab and every PDF probe flow before the window goes away.
- Commands that need a document stay disabled for a loading tab through the
  existing `activeView()->has_document()` providers.
- The agentic control server uses the synchronous wrapper, so its `open` op
  still returns a result; a cancel maps to a `cancelled` error instead of
  `io_error`.

## Test seam

`PicturaMainWindow::setOpenDelayForTest(ms)` stores a delay that every new view
passes to the worker. The worker sleeps before queueing its result, so the
loading tab, spinner animation, hover, click, and middle-click are observable
while the GUI thread is free. Tests use `startOpen...` (returns immediately)
and `QTRY_*`; the synchronous `openDocumentAtPath` wrapper stays for the
existing suites and the self-test.

## Thread-safety notes

- No Qt types cross into a worker: `QString` paths, passwords, names, modes, and
  the `ImportTarget` bridge struct are converted to `String` / a Rust-owned
  struct before `spawn`.
- `Document`, `History`, `PixelBuffer`, and the raster result are `Send` (the
  filter worker already moves `Document` + `History`).
- `decode_image_rgba` (Qt's image decoder) runs on the worker for raster
  imports. Qt image I/O is reentrant; this is the one new Qt call off the GUI
  thread and is covered by the raster entry-point test.
- `current_buffer(doc, gpu)` runs on the worker exactly as a filter commit does.
  GPU resources stay owned by the worker for the composite; the GUI applies the
  finished document.

## File-size split

`frame.cpp` is at the hard cap, so the loading lifecycle and async entry points
live in `frame_open.cpp`; the PDF probe/flow lives in `frame_pdf.cpp`; the PDF
import flow stays in `frame_import.cpp`. `TabSpinner` is its own pair. New
`.cpp`/`.h` files are added to `CMakeLists.txt` explicitly.

## Ceilings (`ponytail:`)

- No decode cancellation token; cancel discards the result and lets the worker
  finish.
- No progress percentage; the spinner is indeterminate.
- `Qt::BusyCursor` is the platform watch, not a custom animated cursor.
- Cancelling one page of a multi-page PDF stops the remaining pages; it does
  not undo pages already opened.
- Revert and Edit Contents stay synchronous.
- The spinner is a theme-coloured arc, not a CS6 asset (CS6 has no tab spinner).
