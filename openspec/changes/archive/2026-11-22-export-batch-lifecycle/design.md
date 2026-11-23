# Design

## Context

See `proposal.md - Why`. The current shape and the constraints that shape the
approach:

- `runCancellableJob` (`crates/pictura-app/cpp/busy_job.{h,cpp}`) owns only the
  progress window: it calls `start()` once, calls `running()` once before its
  loop, then polls `running()` every 16 ms and `progress()` while the modal loop
  runs. Both exporters put their step loop inside the `running` callback, so the
  first write is synchronous and later writes are one per poll; `cancel` flips a
  flag that the next `running` call observes. That is the protocol being
  duplicated.
- The two exporters already share the file writer in Rust
  (`export_tools/imp.rs`, `write_document_file`); this change touches only the
  C++ lifecycle.
- Target lists arrive as `"<key>\t<name>\n"` text from
  `export_layer_targets()` and `layer_comps_api::exportTargets()`; names are
  already sanitized and collision-suffixed there.
- `exportLayersSuffix` and `layerCompsSuffix` are identical but stay with their
  exporters; format resolution is deliberately not part of the seam.
- C++ sources are listed explicitly in `cmake/sources/documents.cmake`; Qt Test
  suites are registered in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- No new dependency. Files target well under the 800 LOC guideline.

## Goals / Non-Goals

**Goals:**

- One module owns validation, target parsing, path composition, preflight,
  iteration, completed-item progress, cancellation, view/parent lifetime,
  stop-on-failure, and failure reporting.
- Both exporters reduce to their existing target text, writer, suffix, and
  wording (destination, empty-list, and per-target).
- The lifecycle is directly testable: cancel between files, view destruction,
  empty targets, and progress monotonicity are checked without sleeps or timer
  races.

**Non-Goals:**

- No change to target selection, rendering, naming, formats, suffixes, dialog
  controls, or document/history/dirty behaviour.
- Zoomify, Data Sets as Files (`variables/imp/export.rs`), and Layer Comps to
  PDF keep their own batch logic; they are not adapters here.
- `runCancellableJob` is not generalised or rewritten. The Rust test hook
  `export_tools_set_delay` and the timing-based layers cancellation tests stay
  as they are.
- No parallel writing, retry, resume, or new UI.

## Decisions

### New module `export_batch.{h,cpp}`

The lifecycle moves to a new pair rather than growing `busy_job`, because
`busy_job` is generic progress mechanics used by many commands (save, filters,
transform, import) and must not learn export policy.

### Free function with a spec and a live run handle

```cpp
namespace pictura {

struct ExportTarget {
    QString key;    // layer path or comp id, adapter-interpreted
    QString path;   // destination/name+suffix
};

struct ExportBatchSpec {
    QWidget* parent = nullptr;
    PictureView* view = nullptr;
    QString title;                  // progress window and warning title
    QString destination;
    QString suffix;                 // exporter-resolved
    QString targetsText;            // "<key>\t<name>\n" lines
    QString destinationFailureText; // shown when the destination is refused
    QString emptyInfoText;          // empty-list message; empty means silent
    std::function<bool(PictureView&, const ExportTarget&)> write;
    std::function<QString(const ExportTarget&)> failureText;
};

struct ExportBatchRun {
    int total = 0;
    int written = 0;
    bool cancelled = false;
    bool failed = false;
    void cancel() { cancelled = true; }
    int progressPermille() const;   // 1000 when total == 0
};

bool exportBatch(const ExportBatchSpec& spec);                       // owns a run
bool exportBatch(const ExportBatchSpec& spec, ExportBatchRun& run);  // caller/test handle

}
```

- `write` receives `PictureView&` only while the module's `QPointer` is alive, so
  an adapter cannot write through a destroyed view.
- `failureText` is called on a target failure (a failed write, a directory
  target refused in preflight, or a destroyed view); destination and empty-list
  wording come from `destinationFailureText` and `emptyInfoText`, so all wording
  is adapter-supplied.
- `ExportBatchRun` is the test surface: a test's `write` lambda captures it and
  calls `run.cancel()` after a chosen file, and can sample
  `progressPermille()`. `run.failed` records a write failure or a destroyed view
  and is never set by a cancel. `progressPermille()` reaches 1000 once the last
  target is written, and the progress window closes at that point; its final
  paint is not part of the contract. Production callers use the one-argument
  overload.
- Alternatives considered: a class with setters (more lines, same behaviour) and
  pre-parsed `QVector<ExportTarget>` (moves the existing text seam into callers
  and duplicates parsing). The spec-plus-lambdas shape matches the existing
  `runCancellableJob` call style.

### Exporter spec builders

Each exporter exposes one builder used by both production and tests:

```cpp
ExportBatchSpec layersExportBatchSpec(PictureView& view, QWidget* parent,
    const QString& folder, const QString& prefix, bool visibleOnly,
    const QString& format, bool includeIcc);
ExportBatchSpec layerCompsExportBatchSpec(PictureView& view, QWidget* parent,
    const QString& folder, const QString& prefix, const QString& format,
    bool includeIcc);
```

Each builds the whole `ExportBatchSpec` - title, destination, suffix, target
text, `write`, `failureText`, `destinationFailureText`, and `emptyInfoText` - so
a test that replaces only `write` (to cancel or fail deterministically) still
exercises the production target text, suffix, and failure wording.
`exportLayers`/`exportLayerComps` reduce to a document guard plus
`exportBatch(<builder>(...))`.

### Parsing and path composition stay in the module

The module splits `targetsText` on `\n`/`\t` and composes
`destination + '/' + name + suffix`, exactly as both callers do today. Malformed
lines are skipped, matching current behaviour.

### The step loop moves behind `runCancellableJob`

The module supplies `start = true`, a `running` callback that performs at most one
write per poll, a completed-item `progress`, a `cancel` that sets
`run.cancelled`, and a `finish` that reports the failing target and returns
`!cancelled && !failed`. `run.failed` is set on a write failure and on view
destruction, never on cancel. `runCancellableJob` calls `running()` once before
its loop, so the first write is synchronous and a cancel raised inside it is
observed immediately, skipping the loop; later writes are one per 16 ms poll.
Cadence, 250 ms window delay, Esc behaviour, and window title are unchanged.

### Preflight before the progress window

Destination validation, parsing, path composition, and the directory-at-target
check run before `runCancellableJob`, so a refusal never flashes the window. The
module validates the destination first, then parses, so a bad destination still
wins over an empty target list, as today. A refused destination shows
`destinationFailureText`; an empty target text shows `emptyInfoText` when it is
non-empty (Layer Comps: "This document has no layer comps."; Export Layers:
empty, so silent), while a non-empty but entirely malformed target text returns
silently; a directory target or a failed write shows
`failureText(target)`. Both a refused destination and an empty list return
`false` before the progress window and are not write failures. Warning and
information boxes appear only when `parent` is non-null. This is what turns the
comps view-destroyed message from `Could not write .` into the specified
`Could not write <path>.`

## Risks / Trade-offs

- **Moving the step closure changes who sees the cancel flag** -> `runCancellableJob`
  calls `running()` once before its loop, so the first write is synchronous and a
  cancel raised inside it returns false immediately, skipping the loop. Later
  writes are one per 16 ms poll, and `running` returns false as soon as
  `cancelled`/`failed` is set.
- **`ExportBatchRun` mutation from inside a writer** -> all calls stay on the UI
  thread; the run is a plain stack object, not shared across threads.
- **View destroyed inside a write callback** -> the next step sees a null
  `QPointer`, sets `run.failed` and the failing target to the next one, and
  reports; this matches the existing layers behaviour and fixes the comps
  message.
- **Behavioural drift while deduplicating** -> the acceptance suites
  (`tst_export_layers`, `tst_layer_comps_export`) already pin formats, naming,
  rendering, and non-mutation; the two comp skips become `tst_export_batch`
  cases over the production comps spec builder, which leaves only `write`
  replaced.

## Migration Plan

Pure in-tree refactor: add the module and the two spec builders, rewire both
exporters, split the tests, then delete the two `BatchState` structs. Rollback
is a revert; there is no data or format migration.
