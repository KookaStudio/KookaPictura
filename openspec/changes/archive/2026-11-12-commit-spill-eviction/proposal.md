# Proposal

## Why

Issue #1290. On 2026-11-12 the 17000² acceptance runs went like this under
`systemd-run --user -p MemoryMax=2G`:
- A Free Transform at ×1.0 with 15° rotation, and one at ×1.5, were killed by
  the cgroup's OOM killer during the commit.
- The scratch disk stayed empty in every run, and the status readout showed no
  packed or spilled tiles, even with 138 MiB of room left.

Phases 3–4 made the commit write spillable tiles, but the evictor never evicted
them. There are two causes.

1. **Hot holds pin almost everything.** The view takes a hot hold when the
   presented view changes and keeps it open until the view changes again.
   While any hold is open, every tile pinned by any thread is stamped hot. The
   evictor skips hot tiles outright. So with the view still, every tile a
   commit reads or writes is unevictable. Closing the view hold is not enough
   on its own: at fit-to-screen on a large document the view's coarse tiles
   are composited from every layer tile, so a view hold covers the whole
   document.
2. **Under a hard limit, eviction starts too late.**
   - The background evictor reads the host every 100 ms. A resample can fill
     the 256 MiB headroom between the ceiling and `memory.max` faster than that.
   - The allocating thread evicts only above 125 % of the ceiling, which is
     past `memory.max`.

   So before an OOM kill, nothing evicts on the thread that allocates.

The runs also showed that the control server's `ControlScope` rejects a
cancellable job's progress window (`jobProgress`) when it shows during a
request. Rejecting the window cancels the job, so no control client can drive a
commit long enough to show progress. The `agentic-control` spec already says a
background job's progress window SHALL NOT be cancelled.

## What Changes

- **A hot hold can be closed.** A closed hold keeps hot only the tiles touched
  while it was open.
- **The view's hold is open only while its tiles build.** It is open on the
  worker that builds them (or on the GUI thread for a synchronous build), then
  closed and kept until the presented view changes. A commit's reads and
  writes no longer count as the view's.
- **Hot tiles are evicted last, not never.** A pass evicts in this order:
  display tiles, history-only tiles, cold live tiles, and only then hot tiles.
  Pinned (borrowed) tiles are never evicted.
- **Under a hard limit (`memory.max`), the allocating thread evicts above the
  ceiling instead of above 125 %.** It also keeps a running count of the bytes
  it allocates between host readings. Once that count crosses the threshold, it
  re-reads the host and evicts, so a burst between background passes is caught.
  Without a hard limit the threshold stays at 125 %.
- **`ControlScope` leaves `jobProgress` windows alone**, as it already does for
  `jobBusy`.

## Capabilities

- **MODIFIED** `document/tile-eviction`: "Eviction frees memory in priority
  order between thresholds" (hot tiles become the last class, closed holds, the
  hard-limit threshold, and the allocation estimate).
- **ADDED** `document/tile-eviction`: "The view keeps only the tiles it built
  hot" and "A large commit under a hard limit spills instead of being killed".
- **MODIFIED** `interop/agentic-control`: "A dialog reached during a request
  is refused" gains a scenario for a job's progress window.

## Impact

- `pictura-core`: `memory/eviction.rs` (holds, pass order, allocation
  estimate), `tiled_plane/body.rs` (hot stamp).
- `pictura-app`: `tiles.rs` (view hold on the tile builds).
- `cpp/control_scope.cpp`.
- No new dependencies.
