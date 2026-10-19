# Proposal

## Why

Opening a document still reads, decodes, composites, and captures history on the
GUI thread. The #667 banner (#701) paints `Opening <file>…` before the blocking
call, but the window is frozen for the whole load: the banner cannot animate, a
large file has no tab and no per-file state, and hover/close/middle-click cannot
work. Issue #729 replaces the banner with a real loading tab and takes the
threading change #667 deferred.

## What Changes

- **Open is asynchronous.** Every open entry point starts the read/decode/first
  composite/history capture on a worker thread and returns immediately. The
  result is applied on the GUI thread and reported through a completion signal.
  A synchronous wrapper remains for the self-test, the agentic control server,
  and tests that need a result before continuing.
- **A loading tab appears immediately.** The tab is titled with the file name,
  shows an empty workspace (no canvas, no document image), and does not steal
  the active tab. If no document is active it is the active tab.
- **The tab's close button becomes a spinner.** It animates while the worker
  runs; hovering swaps it for the normal close glyph; clicking cancels.
  Middle-clicking the loading tab also cancels. Normal tabs keep today's close
  button and middle-click behaviour.
- **Completion is explicit.** Success fills the tab, restores the close button,
  activates the tab unless the user selected another tab during the load, and
  keeps the existing status notice. Failure removes the tab and keeps the
  existing error path. Cancel removes the tab immediately, discards the worker
  result, restores the previous tab, and shows no error.
- **The busy cursor is reference-counted** across concurrent opens and restored
  when the last one finishes or is cancelled.
- **The lower-left banner is removed**, with its stylesheet rule, test hooks,
  and `documentOpenStarted` signal.
- **PDF and EPS keep their dialogs.** A generic PDF's probe and Import PDF
  dialog are not loading phases; a loading tab appears only when a chosen page
  or image render starts. A Pictura PDF's `pdf_open` decode gets a tab as soon
  as the worker finds its editing data. EPS renders get a tab; the bounding-box
  probe and dialog do not.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `document/document-lifecycle`: remove "Open shows a loading banner"; add the
  loading-tab, spinner/cancel, and completion requirements.

## Impact

- Rust (`crates/pictura-app`): new `open_jobs` worker module; `PictureView`
  gains `cxx_qt::Threading`, `open_finished` / `open_detected` signals, and a
  cancellation flag; the existing synchronous `open` / `open_image` /
  `open_as_smart_object` / `pdf_open` / `pdf_import_*` / `eps_import_*` split
  into a pure decode/render half and a GUI apply half.
- App C++: `frame_open.cpp` (loading tab lifecycle, async entry points, sync
  wrappers), `frame_pdf.cpp` / `frame_import.cpp` (PDF/EPS async flows),
  `frame.cpp` (`DocEntry`, start/finish split, guards), `frame.h`,
  `frame_columns.cpp` (banner call removed), `theme.cpp` (banner rule removed),
  new `tab_spinner.{h,cpp}`, `CMakeLists.txt`.
- Tests: new `tst_loading_tab.cpp`; `tst_design_system.cpp` loses the banner
  style case; `tst_open_loading_banner.cpp` is deleted; new Rust unit tests for
  the decode/apply split and cancellation.
- Docs: `docs/01-architecture/threading-and-concurrency.md` resolves its
  "Async runtime choice" open question and describes the open worker;
  `docs/10-workflow-io/document-lifecycle.md` records the loading tab. The docs
  commit carries `TASK-ALLOWS-DOCS`.
- No new dependencies. No engine-crate behavior change: the codec/render entry
  points are unchanged, only where they run.

## Non-goals

- Interrupting the decode itself mid-call: cancel discards the result; a codec
  cancel token is separate.
- Progress percentage: the spinner is indeterminate.
- Changing middle-click behaviour for normal documents.
- Moving Revert or Edit Contents onto the worker; they reload an already-open
  document and are not Open commands.
