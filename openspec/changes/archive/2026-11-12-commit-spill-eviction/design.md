# Design

## D1. Open and closed hot holds

`HotHold` gains `close(&mut self)`. The registry tracks two things:
- the open holds' start epochs;
- the closed holds' `[start, end)` epoch intervals.

When a body is pinned, it is stamped with the current epoch, but only while
some hold is **open**. Closing a hold advances the epoch, so later stamps fall
outside the closed interval. A body counts as hot when either:
- its stamp is at or after the oldest open hold's start, or
- its stamp falls inside a live closed interval.

A stamp of 0 is never hot.

`Evictable::hot_since(oldest)` becomes `hot_stamp() -> u64`. `run_pass` takes
one snapshot of the holds (`HotSet`) and checks each body against it. Closed
intervals are checked with a binary search over a sorted `Vec`.

## D2. Pass order: hot tiles last

`run_pass` keeps its classes (history-only, then live), now with hot tiles
after both:
- Rounds 1–2: history-only, then cold live tiles, skipping hot ones.
- Round 3, only if the pass still needs memory: hot tiles, history-only first.

Pinned bodies are skipped in every round. The clock bit is consumed only in
rounds 1–2, so in round 3 a hot tile is evicted even if it was read since the
last pass. Round 3 packs before it spills, like the other rounds.

## D3. The view hold

`DisplayTiles::view_hold: Option<HotHold>` becomes
`view_holds: Vec<HotHold>`, which holds closed holds only:
- **Worker build.** `TileJob` takes `hot()` on the worker before `build()`,
  closes it after, and hands it to `land_tiles`.
- **Synchronous build.** `build_now` does the same on the GUI thread.
- **Keeping it.** `land_tiles` keeps a hold only when the job's view key still
  matches the presented one.
- **Releasing.** `pin_presented_view` clears the holds when the view key
  changes, where it used to take a new open hold.

**Ceiling (`ponytail:`):** the list keeps the 8 newest holds. A tile rebuilt
after an edit is stamped again by the newer hold, so the oldest intervals only
cover tiles the view has already re-read.

## D4. Allocation-time eviction

`step()` records three things after each host reading:
- `BASE_USED`: the used bytes it read;
- `URGENT_AT`: the threshold;
- `SINCE`: the bytes allocated since the reading, reset to 0.

The threshold is the ceiling when the reading has `cgroup_max`, else 125 % of
the ceiling. `on_allocate(bytes)` adds to `SINCE`. When
`BASE_USED + SINCE > URGENT_AT` and at least 4 MiB has been allocated since the
last check, the thread re-reads the host:
- If it is over the threshold, it reclaims down to 90 % of the ceiling.
- Either way, it resets `BASE_USED` and `SINCE` from the fresh reading.

`refresh_urgency()` (tests) records the same three values.

The 4 MiB step bounds the re-reads to one per 4 MiB allocated, even when
nothing can be evicted. Reading the cgroup means reading two small files.

## D5. ControlScope

The `eventFilter` exemption is widened from `jobBusy` to `jobProgress` too. A
person can still cancel the window. A control request that started the job
blocks in the job's nested loop until the job lands, as it already does for
`jobBusy`.
