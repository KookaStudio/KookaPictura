# Threading and Concurrency

- **Spec ID:** `ARCH-005` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` (technology-selection document)
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-006` gpu-rendering-pipeline, `ARCH-007` undo-history, `ARCH-010` document-model

> All module and crate names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not.

## CS6 behavior

The user experiences concurrency as responsiveness: pans and zooms stay smooth
while a filter runs, long filters show a progress indicator and can be
cancelled, and history records one clean step per operation rather than a flood
of intermediate states. The Mercury Graphics Engine is described as delivering
near-instant results for Liquify, Warp, Lighting Effects, and Oil Paint, with a
CPU fallback when no supported GPU is present. Those user-visible outcomes, not
the internal thread count, are the contract.

Where a filter is not GPU accelerated, for example Reduce Noise per the
Bare Feats tests, the CPU path must still keep the interface responsive and
report progress.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Progress indicator | Status bar / modal | `Esc` to cancel | Long filters and file I/O |
| Cancel button | Dialog | `Esc` | Must cancel without corrupting the document |
| History panel | Item view | `F7`? | One entry per committed operation, never per tile |
| Options bar | Tool bar | n/a | Live values during an in-flight preview |
| Canvas | GPU view | n/a | Shows preview while a job runs; final on commit |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Worker threads | int | `max(1, cores-1)` | 1 to cores | Reserve one core for UI |
| Tile size | int (px) | 256 | 64 to 512 | Shared with `ARCH-006` |
| Progress throttle | ms | 100 | 16 to 500 | Coalesce GUI updates |
| Cancellation check | tiles/rows | every tile | 1 to N | Fine-grained enough to cancel fast |
| History memory cap | MB or % RAM | TBD | e.g. 1 GB to 60% RAM | Owned by `ARCH-007` |
| Preview mode | enum | Transient | Transient / Snapshot | Transient is not undoable |
| Job timeout | s | none | none or N | For deadlock detection |

## Algorithms & pipeline

### Threads

1. **GUI/main thread (Qt).** Runs the Qt event loop and owns all `QObject`s,
   widgets, and `QAbstractItemModel`s. Applies committed commands, emits
   signals, and serves the canvas. `QAbstractItemModel` is not thread-safe and
   its API is called only from the thread it lives in.
2. **Render path.** With `QRhiWidget`, rendering is driven from the widget's
   paint cycle on the GUI thread; a QRhi instance and its resources must be used
   from one thread only, and resources are not shareable between QRhi instances.
   If QML is used later, `QQuickRhiItem` splits item and renderer and runs the
   renderer on the scene-graph thread, communicating only through
   `synchronize()`; that split is then mandatory.
3. **Worker pool (proposed: rayon).** CPU-bound, data-parallel work: tile-based
   filters, adjustments, resampling, analysis. Workers have **no Qt types** and
   no access to the document's shared mutable state; they operate on owned or
   `Arc`-shared immutable inputs and produce owned outputs.
4. **Async I/O (implemented for document open).** The open path's file read,
   decoding, first composite, and initial history capture run on a std thread;
   the worker owns no Qt types and posts its owned result back with a
   `CxxQtThread` queued signal. The GUI thread applies it, inserts the loading
   tab's document, and emits the completion. Cancelling a loading tab drops the
   result; the decode itself is not interrupted. Save, RAW, and
   PSD serialization remain on the GUI thread (Background Save is separate).
5. **Writer.** Document mutations are serialized through one logical writer.
   For the first implementation the writer **is the GUI thread**, which is the
   lazy correct choice: it removes the need for a second full consistency model
   and matches the Qt model rule. A dedicated document thread is a later
   optimization behind the same command interface.

### Why start single-writer on the GUI thread

`QAbstractItemModel` and `QObject` already constrain mutation to the GUI thread.
Introducing a second writer thread immediately requires either a second,
authoritative copy of the document (and a sync protocol) or moving the models
off the GUI thread. Neither is needed for parity. The command queue and the
immutable-snapshot design below make a later move to a dedicated writer a
mechanical change rather than a redesign.

### Command queue

A command is a small value describing a mutation and knows how to apply and
revert itself. Commands are dispatched on the writer thread in order.

```
enum JobState { Queued, Running, Cancelled, Failed, Completed, Committed }
```

- Interactive edits that must feel live (brush strokes, slider drags) use a
  **transient preview**: recompute the affected tiles into a preview layer that
  is composited but never enters history. On commit (mouse-up, Enter, or dialog
  OK), a single command captures the net change as one undo record.
- Long, non-interactive operations are modeled as jobs that produce results,
  followed by a commit command that atomically applies all results. The job and
  the commit are separate so cancellation is clean.
- Ordering: previews may be superseded; only the last preview for a given target
  survives. Commits are never superseded.

### Cancellation and progress

- A cancellation token (`Arc<AtomicBool>`, or a runtime token) is shared with
  the job. Workers poll it at tile or row granularity and return early.
- Progress is a monotonic counter of completed work units. Rather than emitting
  per tile, a reporter thread coalesces updates to at most one GUI signal per
  throttle interval.
- On cancel, the job's partial outputs are dropped. If the operation captured a
  region snapshot at start for revert purposes, the snapshot is also dropped.
- Cancelling never leaves a half-applied commit: results are applied only by the
  commit command, and only if the job completed.

### Undo interaction

- History records **committed** states only. In-flight jobs have no undo entry.
- A destructive operation that needs its before-state (for example, a filter
  applied in place) captures the affected region at job start as an immutable
  snapshot referenced by the commit command. Memory accounting is shared with
  the history budget.
- Undo while a job is running: the command layer refuses to undo across an
  in-flight target, or cancels the job first and then undoes. The chosen policy
  is one line in the spec and must be fixed (see `## Open questions`).
- Undo/redo invalidates tile caches and any live preview generation; a
  generation counter on caches prevents stale tiles from being displayed.

### Memory safety across the FFI boundary

- Worker inputs and outputs are Rust-owned (`Arc<[T]>` or `Arc<Mutex<...>>` with
  a narrow lock). Qt types never cross into a worker.
- Handles crossing the bridge are owned with an explicit contract: either Qt
  copies, or Rust retains ownership and hands out a borrowed view valid only
  for the call.
- No `&mut` alias may exist on both sides of the boundary. Shared mutable state
  is protected by a lock or channel, never by two live references.
- A generation/stamp check rejects results computed against a document state
  that has since been undone or replaced.
- GPU resources have a single owning thread. Cross-thread GPU handoff uses a
  queue and a fence, or CPU readback, per `ARCH-006`.

## Rust module mapping

- `pictura_core::command` — `Command` trait (`apply`, `revert`), commit records.
- `pictura_core::job` — job lifecycle, cancellation token, progress counters.
- `pictura_core::pool` — tile-work partitioning over the worker pool.
- `pictura_qt::threading` — bridges progress/cancel to Qt signals via queued
  connections (CXX-Qt `CxxQtThread`/`Threading`).
- `pictura_core::io` — async file I/O and decoders posting completions.
- `pictura_core::history` — snapshots and undo stack (see `ARCH-007`).

Data crossing: `u64` job ids, progress counts (`u64`), cancellation flags,
and owned buffer handles. No Qt types in `pictura_core`.

## Qt6 component mapping

- `QThread`/`QThreadPool` are **not** used for CPU work; Rust controls its own
  pool. Qt threads are used only where Qt requires them (scene-graph render
  thread if QML is adopted).
- Queued connections carry cross-thread notifications to `QObject`s on the GUI
  thread.
- `QProgressDialog` or a status-bar progress widget consumes coalesced updates;
  its Cancel maps to the cancellation token.
- `QAbstractItemModel` updates are applied on the GUI thread after commit.

## Data-model impact

- Jobs add transient state only: id, target node, progress, token. Never
  serialized.
- Commands carry the reversible delta; the document model exposes an immutable
  view for workers.
- History snapshots and memory caps are owned by `ARCH-007`; this document
  defines when snapshots are taken (at commit or at job start for in-place ops).
- No PSD/XMP fields are affected.

## Edge cases

- **Document closed mid-job.** Cancel the token, drop results, clear the job.
- **Undo/redo mid-job.** Refuse or cancel-then-undo, per the fixed policy.
- **Nested parallelism.** A worker must not spawn another full parallel region;
  the pool is entered once per top-level job. Over-subscription is a known
  cause of slowdowns.
- **Thread-pool starvation.** A worker must never block on the GUI thread or on
  another job. Blocking calls (modal dialogs, GPU fences) are forbidden in the
  pool.
- **Progress flood.** Coalescing is mandatory; unthrottled signals can saturate
  the event loop.
- **GPU device lost / driver reset.** The render path fails; in-flight
  GPU jobs are cancelled and the CPU fallback is offered (`ARCH-006`).
- **Panic in a worker.** A panicking pool task must not poison the pool or
  unwind across FFI; catch at the task boundary.
- **Very large (PSB) documents.** Tiles and job counts grow; progress arithmetic
  must use 64-bit counters.
- **CPU oversubscription.** Reserve a core for the UI; clamp the pool.
- **Blocking-queued deadlock.** Blocking queued connections between two threads
  can deadlock if both block; prefer non-blocking queued or channels.
- **Runtime dependency risk.** A proposed async runtime adds weight; evaluate
  std threads plus channels if requirements are simple (see `## Open questions`).

## Parity acceptance criteria

- Given a long filter is running, the canvas continues to pan and zoom at
  interactive frame rate and the progress indicator advances at least 10 times
  per second.
- Given Cancel is pressed during a long filter, the job stops within one tile (or
  one row) of work and the document is byte-identical to before the operation.
- Given a non-interactive filter completes, exactly one history entry is added,
  regardless of tile count.
- Given Undo is invoked while a job targets the same node, the defined policy is
  followed and the document never enters a half-applied state.
- Given a worker computes a result against a state that is undone before commit,
  the result is rejected by the generation check and never displayed or
  committed.
- Given a Panic occurs in a worker while another job runs, the application
  reports the error and the remaining jobs complete normally.
- Given the pool is saturated by N jobs, the GUI thread never blocks on a worker
  and remains responsive.

## Sources

Fetched for this document:

- `https://doc.qt.io/qt-6/qabstractitemmodel.html` — model is not thread-safe;
  API only on the model's thread; background updates must be queued to the main
  thread; `begin/end` and `dataChanged` contract.
- `https://doc.qt.io/qt-6/qrhi.html` — a QRhi instance and its resources are
  bound to one thread; resources are not shareable between QRhi instances;
  command buffers submit at `endFrame`; threaded-render-loop comparison.
- `https://doc.qt.io/qt-6/qrhiwidget.html` — rendering driven from the widget
  paint cycle; `initialize`/`render` contract; single API per window.
- `https://doc.qt.io/qt-6/qquickrhiitem.html` — item/renderer split, render on
  the scene-graph thread, `synchronize()`, no cross-thread shared variables.
- `https://docs.rs/cxx-qt/latest/cxx_qt/` — `CxxQtThread`, `Threading`,
  `ConnectionType`, `QMetaObjectConnection` available for cross-thread work.
- `https://barefeats.com/pscs6.html` — Reduce Noise is primarily a CPU function;
  GPU-disabled runs, confirming a CPU fallback exists.

## Open questions

- **Undo-during-job policy.** Cancel-then-undo versus refuse-to-undo is not
  chosen. Resolve with a design note here; either is acceptable if it is fixed
  and tested.
- **Async runtime choice (resolved for open).** The document open uses std
  threads plus a `CxxQtThread` queued completion; no runtime is warranted for
  one owned result per open. Save remains synchronous, so a runtime decision for
  Background Save is still open.
- **Dedicated document writer.** When to move the writer off the GUI thread and
  what consistency model (double buffer versus immutable snapshots) it uses.
  Resolve with a profile once the single-writer version is complete.
- **rayon and tokio API specifics.** Verification against current crate
  documentation is outstanding; only the design roles are fixed here.
- **History snapshot timing.** Commit-time versus job-start snapshots, and the
  memory cost of each, must be reconciled with `ARCH-007`.
- **Progress semantics for GPU jobs.** Whether GPU readback permits meaningful
  incremental progress, or whether the indicator is indeterminate for that path.
  Resolve in `ARCH-006`.
- **Worker panic reporting.** The user-facing error surface for a failed job is
  unspecified; resolve with `11-cross-cutting/error-handling.md`.
