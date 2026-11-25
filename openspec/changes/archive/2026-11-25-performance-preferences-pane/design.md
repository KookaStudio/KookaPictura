# Design

## History States

`history::depth` holds a process-wide limit in an `AtomicUsize`. The default
is 20 and `set_state_limit` clamps to 1–1000. `History::capture_live_with`
trims to the limit, replacing `MAX_DEPTH`. A test sets a per-history
override, in the same way as the byte cap, so parallel tests never share the
global.

`History::apply_state_limit` trims an existing history at once. It drops the
oldest state while the history is over `limit + 1` entries and the cursor is
not at the oldest. If the history is still over the limit, it truncates redo
states from the end, pinning a History Brush source that would be dropped,
as `capture_live_with` does. Snapshots are untouched. A new bridge function,
`apply_history_states(view)`, calls it and emits `changed` so the History
panel refreshes. The frame calls it for every open document when the
preference changes.

## Cache Levels

`pictura-render::tile_pyramid` holds `CACHE_LEVELS` (an `AtomicUsize`,
default and maximum 8, minimum 2). `TilePyramid::new` takes
`level_count(w, h).min(cache_levels())` and `fits` also compares the level
count. The next `fit_tiles` (any paint) therefore rebuilds a pyramid whose
depth no longer matches. `TilePyramid::with_levels` takes the cap as an
argument, so engine tests do not touch the global. Level selection already
clamps to `level_count`, so a capped pyramid presents its coarsest level and
the canvas scales it. A reduced-level stroke preview whose level lies past a
capped pyramid gets no snapshot (`level_snapshot` returns `None`), so that
stroke runs exact: correct, slower, and only when the user lowered the cap.

The Cache Tile Size menu shows "256 px" (256² RGBA = 256 KB) and is
disabled. `TILE` and `TILE_SIDE` are constants that the codecs, the history
and the renderer all depend on.

Presets: Tall and Thin sets Cache Levels to 4, while Default and Big and
Flat set it to 8. CS6 picks these values from RAM and CPU count. Ours are
fixed and approximated.

## Scratch Disks

The list edits the existing `scratchVolumes` preference through
`set_scratch_volumes` and holds at most four entries. Each row shows the
path, its free space (`QStorageInfo`), and "(tmpfs: uses RAM)" when the
filesystem is tmpfs. An empty list means the default
`$XDG_CACHE_HOME/kooka-pictura/scratch`, shown as a disabled placeholder
row.

## Memory Usage

A `QSlider` (5–100) and the existing spin box stay in step. "Let Pictura Use:
N MB" reads `ceiling` from `memory_status_json()` after every change, or shows
"—" when the governor has no ceiling.

## Graphics processor

The frame passes the first field of `gpu_adapter_info()` to the dialog, which
shows "Detected Graphics Processor: <name>" or "No graphics processor found".
Use GPU Compute keeps its existing enable rule.

## Status bar

The readout menu gains Scratch Sizes and Efficiency after Measurement
Scale. `status_readouts.{h,cpp}` builds the readout text, which moves the
size readout out of `frame.cpp` (at its line cap). The readouts work without
a document.

- Scratch Sizes follows the CS6 definition, "memory used by open images /
  memory available": the governor's `used` over its `ceiling`, formatted as
  CS6 does (`312.5M/2.04G`). Scratch-disk bytes appear per volume on the
  Performance page.
- Efficiency is `compute / (compute + io)` over the interval since the
  previous reading in which time was measured. It starts at 100 % and
  keeps the last value while nothing runs. A cumulative figure would let
  one early spill depress it forever, while CS6 describes recent
  operations.

## Persistence

`historyStates` (1–1000, default 20) and `cacheLevels` (2–8, default 8) join
`[performance]` in the schema and `preference_fields.inc`. The frame applies
them at start.

## Scratch store access from the GUI (review follow-up)

`scratch::set_volumes` queues the new list in `PENDING` and applies it on
the first `try_lock` that succeeds, or when the next holder of the store
lock takes it. `volumes()` and `sizes()` try the lock and fall back to the
last successful reading. They do this even while a change is pending, so a
2 s readout can be one sample stale but never waits behind a spill. Only
the first reading in a process blocks.
