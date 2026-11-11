# Design

## Context

- `runCancellableJob` (`busy_job.cpp`) already drives Auto-Align and Auto-Blend:
  a `QProgressDialog` named `jobProgress` that appears after 250 ms, a 16 ms poll
  of `running()` and `progress()`, and Cancel wired to `cancel()`. It does not
  set the wait cursor, but `runBusyJob` does.
- A Free Transform commit of 4 MP or more calls `defer_transform`, and
  `build_transform` spawns a detached worker that queues its landing onto the
  GUI thread. This path has no window and no cancel. Any reader that calls
  `settle_pending_composite` before the landing runs the whole resample again,
  synchronously, on the GUI thread.
- A lifted (selection) transform always commits synchronously.
- The resamplers that matter are `transform::resample_plane` (similarity,
  quad, and crossed quad, all through `transform_layer_with_map`),
  `transform_native` (deep planes, which reuses `resample_plane`), and
  `warp_store` for warps.

## Decisions

### D1. `pictura_core::job::JobProgress` with a scoped thread-local

```rust
pub struct JobProgress { cancel: AtomicBool, done: AtomicU64, total: AtomicU64, shown: AtomicU32 }
pub struct Cancelled;
pub fn run<T>(job: &Arc<JobProgress>, f: impl FnOnce() -> T) -> Result<T, Cancelled>;
pub fn current() -> Option<Arc<JobProgress>>;
```

`run` installs the token in a thread-local for the duration of `f`, and returns
`Err(Cancelled)` when the flag is set at the end. The app splits the work into
equal stages with `set_stages(n)`, one per transformed member. A layer-level
engine call (`transform_layer_with_map`, `transform_layer_warp`) opens a stage
with `begin_stage(rows)`, counting every plane's output rows. Its resample loops
read `current()` once, on the calling thread, before they fan out with rayon,
call `advance` per row or band, and skip the remaining rows once `cancelled()`
is true. The displayed value is stored with `fetch_max` and capped at 999 until
`run` finishes, so it never decreases, even when a stage's row estimate is off.
A cancelled layer call refuses before `write_layer`, so its document is
unchanged, as for any other refusal.

Why not a parameter? The transform entry points (`transform_layer_with`,
`transform_layer_quad_with`, `transform_layer_warp`) are called from more than
30 places: actions, Auto-Align, Puppet Warp, the agentic API, and tests. PR 2's
`resize`, `rotate_in` and conversion paths would need the same change. The
scoped context costs nothing when no job is installed, because `current()` is
`None` and the loop is unchanged.

**Ceiling (`ponytail:`):** an engine that calls `resample_plane` from inside its
own rayon task does not see the caller's thread-local. Such an engine must
capture `current()` before it fans out and re-install the token with
`job::scope`. The transform paths in this change do not nest that way.

### D2. A cancelled engine leaves garbage on a clone only

The job's worker always works on a `Document` clone, and planes are shared tiles,
so the clone is cheap. A cancelled resample may return a partly zeroed plane.
`job::run` reports `Cancelled`, and the worker drops the clone, so the live
document is never touched. The engine functions keep their `bool` contract.

### D3. `TransformCommitJob` replaces `DeferredTransform` and `TransformJob`

`commit_transform` keeps its synchronous path for transforms under 4 MP. At or
above 4 MP, for both group and lifted transforms, it:

1. clones the live `TransformSession` as `resume`, and the document;
2. spawns the worker: `job::run(&progress, || (spec.apply)(&mut doc))`, plus
   `merge_lifted` for a lifted session;
3. parks `TransformCommitJob { worker, progress, resume, label, generation }`
   on the view, keeping the session and its overlay live;
4. returns `true`.

The four bridge functions `transform_commit_running`, `_progress`, `_cancel`
and `_finish` follow the Auto-Align shape. `finish` joins the worker and then
does one of two things:

- **landed** — installs the transformed layers (or, for a lifted transform, the
  whole document), drops the session, and records the edit through the existing
  `commit_transform_edit` path (one `"Free Transform"` state, a deferred region
  composite at 16 MP or more);
- **cancelled or refused** — restores the cloned session and leaves the document
  and history untouched.

`settle_pending_composite` keeps its role for API callers that never run the
window. It joins the job synchronously and lands it.

### D4. One C++ helper for every commit site

`runTransformCommit(QWidget* parent, PictureView& view) -> bool` calls
`commit_transform` as the job's `start`, then `runCancellableJob` with the four
bridge functions. When `commit_transform` committed synchronously,
`transform_commit_running` is false at once, so no window appears. The four
call sites (`tool_transform.cpp` commit, the controls-box release in `tools.cpp`,
the Edit > Transform menu in `frame_menus_transform.cpp`, and the tool-switch
prompt) call it. The self-test calls stay on the raw API, which settles.

### D5. Wait cursor in `runCancellableJob`

The same `setOverrideCursor(Qt::WaitCursor)` / `restoreOverrideCursor` pair as
`runBusyJob`, set right after `start()` succeeds and while `running()` is true.
Auto-Align and Auto-Blend gain it too, which the issue asks for. `QProgressDialog`
already maps Esc to `canceled`, and `lop_esc_cancels` pins that behaviour.

## Risks

- **Re-entrancy.** The modal loop excludes no events (it is a `WindowModal`
  dialog, the same as Auto-Align). The window-modal dialog blocks input to the
  frame, and timers still run. The Qt test checks that a second commit request
  during the job is refused (`transform_commit_running` guard).
- **Session clone cost.** `TransformSession` holds a lifted layer and the
  `before` document as `Arc`-shared tiles, so a clone copies no pixels.
