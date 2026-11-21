# Design

## Context

`main.cpp` installs a stderr Qt handler after constructing QApplication and
resolves the state root later. The document session captures labels for
committed edits, including canvas gestures. The command registry supplies
parameters for recordable commands and can report them after an edit commits.
Native save workers already snapshot document identity and metadata revision.
JPEG and TIFF already write XMP. Raster open currently imports Photoshop
resource blocks, while their standard XMP packets have their own container
fields.

## Goals / Non-Goals

**Goals**

- Put diagnostic ownership at the executable lifecycle and reuse the existing
  stderr and state-home conventions.
- Keep History Log capture independent of the undo stack and merge it at the
  same save boundary as other persisted metadata.

**Non-Goals**

- Telemetry, diagnostic uploads, crash reports, and per-frame tracing belong to
  separate work.
- The format target is interoperable XMP with readable Kooka Pictura text.
  Exact CS6 text formatting still needs a CS6 fixture. Ordinary PNG Save As
  would need its own metadata plumbing. This change covers PNG via Save for Web.

## Decisions

### Own the diagnostic subscriber in the app

Add Rust logging to `pictura-app`, with a declared-only cxx-qt bridge and a
C++ lifetime owner in the startup path. Resolve launch mode and the effective
state root before opening the file layer. Install the Qt handler before
QApplication where the chosen startup path supports it, restore the old
handler on shutdown, and keep the diagnostic writer alive through Qt teardown.
Keep the early child modes on stderr and preserve their stdout protocols.
Honor Qt category rules and expose the Rust target filter through `RUST_LOG`.
Start with discrete open, save, and GPU outcome events. Existing test tokens
and explicit protocol output keep their current writers.

Use `tracing` for event fields, `tracing-subscriber` for filtering and
formatting, and an app-owned bounded, lossy writer thread for disk I/O.
Bridge the existing `log` facade through the subscriber for dependency event
metadata. Format file records as newline-delimited JSON. Persist a schema of
reviewed fields. Qt records use a severity, category, and fixed event identity.
Rust first-party records use controlled identifiers and structured outcome
fields. A file-layer visitor drops free-form message and error strings from
dependencies and Qt. Preserve Qt's existing diagnostic text on stderr and
write fatal output synchronously there. The bounded queue keeps disk waits
away from Qt and render callers. [Qt logging](https://doc.qt.io/qt-6/qtlogging.html#qInstallMessageHandler)

Use per-launch file names with a UTC date prefix. Retention prunes only files
owned by this sink outside the seven-day window. Separate files avoid
cross-process writes into the same record, with the date, timestamp, and
launch identity available for correlation. Detect both initialization and
post-start writer failures at the sink. Report once through a direct stderr
path, then disable the failed file writer while the stderr layer continues.
At orderly shutdown, after Qt teardown, stop accepting file records and ask
the worker to drain and flush for at most two seconds. If it completes, every
accepted record is flushed. On error or deadline, report the incomplete drain
directly to stderr, detach the worker, and allow process exit without writing
to stdout. Do not use `tracing-appender`'s `WorkerGuard` drop for this contract.
Exercise both timely and stalled shutdown and abrupt fatal exit in child processes.

### Capture History Log at successful edit and lifecycle boundaries

Store the History Log options in the typed preferences store under General.
Let the existing document and application lifecycle report successful opens,
closes, starts, and quits.
Record edits at the committed document-history capture boundary, including
deferred gesture completion. Sample the active settings per event. Copy the
canonical edit label and, when available, a stable ID and supported action
parameters. Carry command detail with the particular execution and target
document through completion. Use the existing parameter values from command
execution where possible. Emit a single edit row even when an action also
reports completion. Canvas edits and action replay use this same capture
boundary. Parameters without an explicit provenance representation keep the
edit label. Store document names and escape control characters so text rows
stay readable and unambiguous.

Keep record storage outside snapshots. This preserves the recorded chronology
when the user moves the undo cursor or purges the History panel. Order text
appends through a bounded nonblocking writer. Keep the resolved destination
and a sequence ID with each record so path changes cannot redirect queued rows.
On queue saturation, reject the individual new row and report its sequence ID
and destination. Editing and independent metadata capture continue. On a write
failure, report the affected row and every queued row that cannot be written,
including their sequence IDs and destinations. Batch notices by destination and
sequence ranges to avoid an unbounded UI notice queue while keeping every
affected row identifiable. On orderly quit, enqueue document closes before the quit
row, stop accepting rows, and drain and flush for at most two seconds. Report
any rejected, failed, or undrained rows to the user while the UI is available,
or directly to stderr once it is gone. Detach a stalled worker at the deadline
so exit remains bounded.

### Patch XMP before encoding and acknowledge by snapshot

Use the existing namespace-aware XMP patcher against the snapshot's current
packet, preserving sibling properties. Treat prior history as opaque text and
append complete UTC records. The existing Photoshop File Info tab is the
display surface. Preserve newlines when passing the text through the File
Info bridge.

Each save stages the eligible record prefix with the document identity and
metadata revision. Keep the saved history baseline in the session across
undo and redo. On a successful save of the source document, fold the staged
prefix into that baseline and advance the cursor for those record IDs only.
When File Info changed only unrelated properties during the save, rebase the
successfully saved prefix onto the newest raw packet before acknowledging it.
If rebasing fails, keep those records eligible and report the omission.
For each accepted explicit raw File Info edit, compare the namespace-resolved
History value before and after the edit, distinguishing absent from present.
Increment a History change generation when that value changes, including when
it later changes back. If the generation changed during the save, the effective
replacement or removal wins: adopt the current value as the new baseline,
acknowledge the saved prefix without reintroducing it, and retain later captured
records for future appends. A rewrite that leaves History equal is unrelated
for this purpose and retains the saved rows.
A document replacement must win over completion of an older job.
Save a Copy and Save for Web stage their own view of the entries and leave
the source cursor available for its next save. Failed saves keep pending
records available for retry.

Native save and TIFF already stage a detached document. JPEG can stage just
its `RasterMetadata` packet. On raster open, read JPEG APP1 and TIFF tag 700
within existing metadata bounds and merge the packet into resource 1060,
preserving other imported resources. Prefer a valid standard container packet
when an embedded Photoshop resource block also supplies XMP. A malformed
packet leaves image open available. Save for Web should stage the packet
before its existing metadata filtering so copyright-only selections retain
their existing exclusions. For ordinary and sliced full-XMP JPEG exports,
preflight the staged packet against APP1 capacity. If only the append overflows,
export the existing encodable packet, warn the user, and keep the omitted rows
eligible for the source save. Apply the same rule to each JPEG slice. [Structured XMP patching](../../specs/codec/xmp-structured-properties/spec.md) · [Save for Web metadata policy](../../specs/interop/save-for-web/spec.md)

## Risks / Trade-offs

- **Sensitive event data.** Test the file layer with sentinel strings in
  filenames, document content, Qt text, and dependency events. The opt-in
  History Log has its own destinations and stores the limited provenance
  fields defined for that feature.
- **Sustained diagnostic traffic or fatal exit.** The lossy queue can discard
  buffered events during a flood or process abort. Synchronous fatal stderr
  provides the final diagnostic in the abort case.
- **Oversized or unsupported XMP.** Preflight the staged append against the
  existing XMP bound and JPEG's APP1 capacity. When only the append fails,
  retain the original encodable packet, surface a notice, and leave staged
  entries available for another destination. Bound pending metadata growth
  per document. Report saturation and retain the already captured prefix.
- **Editing continues during save.** Keep save staging and completion scoped
  to the saved document and record sequence. Exercise success, failure, copy,
  and a later save with a fixture whose worker is held in flight.

## Migration Plan

Absent History Log settings resolve to Off, Metadata, and Sessions Only.
Existing files keep their original XMP until the user saves an enabled
metadata destination with eligible entries.
