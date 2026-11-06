# Design

## The governor (`pictura-core::memory`)

```
memory/
  system.rs    pure parsers + the I/O layer over injectable roots
  governor.rs  Governor, Reservation, MemoryError, Pressure, try_plane
  tests.rs
```

**Reading the host.** `HostPaths { proc_root, cgroup_root }` (default `/proc`,
`/sys/fs/cgroup`). `read(&paths) -> HostReading` fills:

| Field | Source (Linux) | Elsewhere |
|---|---|---|
| `total`, `available` | `meminfo` `MemTotal` / `MemAvailable` | the OS calls that moved here from `pictura-codec` |
| `rss` | `self/status` `VmRSS` | none |
| `cgroup_max`, `cgroup_high` | the smallest `memory.max` / `memory.high` of the process's cgroup (`self/cgroup`, the `0::` line) and every ancestor below the root; `max` means none | none |
| `cgroup_current` | the leaf's `memory.current` | none |
| `psi_some_avg10` | `pressure/memory`, the `some` line | none |

Each field is a pure parser over the file's text plus a read; a missing file is
`None`, never an error.

**Ceiling.** `limit = min(usage % × total, cgroup_max, cgroup_high)` over the
values present; `ceiling = limit − max(limit / 20, 256 MiB)`. No value present
means no ceiling.

**Room** is what a new reservation may take:

- `ceiling − used − reserved`, where `used` is the cgroup's `memory.current`
  when a cgroup bound is present (what the kernel's OOM decision counts) and the
  process's RSS otherwise;
- also `available − max(available / 20, 256 MiB) − reserved`, so the app still
  backs off when other programs hold the machine's memory;
- the smaller of the two present, or unlimited when neither is readable (an
  unreadable host is not a refusal, as before).

**Pressure.** `some avg10` ≥ 20 % is Critical, ≥ 5 % Elevated, else Normal.
Without PSI, the free share (`available / total`) below 10 % is Critical,
below 20 % Elevated. Nothing acts on pressure in this step; `status` reports it
and the eviction of later steps (#1216, #1217) will read it.

**Caching.** The reading is cached and refreshed when older than 250 ms, so a
gate costs a mutex and a clock read. `reserved` is an atomic.

**Test hook.** `set_host_for_test(Option<HostReading>)` pins the reading for the
calling thread (a thread-local, as the gate's old fake was), so parallel tests
in one process do not see each other's host. `HostPaths` over a temp directory
tests the I/O layer.

**Refusals.** `Governor::reserve(bytes, purpose)` returns a `Reservation` (held
until dropped) or `MemoryError::Insufficient { purpose, bytes }`;
`Governor::admit(bytes, purpose)` is a reserve dropped at once, for gates whose
work allocates right after. Every refusal is recorded for the calling thread
(`last_refusal`, `clear_refusal`), which is how the app's existing
`edit_refused_for_memory()` learns about refusals raised deep in an engine call.

## Fallible allocation

`try_plane(len, value, purpose) -> Result<Vec<u8>, MemoryError>`: for 16 MiB
or more it first takes `admit` (so a buffer that would cross the ceiling is
refused before the allocator is asked), then `Vec::try_reserve_exact`; an
allocator error becomes `Insufficient` and is recorded like any refusal. A
thread-local failpoint (`fail_next_plane_for_test`) makes the next call fail,
to test the error path without exhausting memory.

ponytail: only New Document's planes and the Paint Bucket's match plane are
built through `try_plane` in this step; the other canvas-scale buffers rely on
the gate's estimate. Steps 3–5 of #1209
replace those buffers with tiles, which is where the remaining sites go.

## Gates

Each gate is one `memory_gate::admit(estimate)` at the entry the benchmark
drives, before anything allocates or changes. `P` is the canvas's pixels, `B`
the bytes per sample, `c` the colour channels, `doc` the bytes of the
document's planes (composite, channels, layers, masks; shared planes once).

| Path | Entry | Estimate |
|---|---|---|
| New Document (M1) | `PictureView::new_document`, `create_document` | `P × (B × (2c + 1) + 17)`: layer, composite, display and its pyramid |
| Add Layer (M2) | `add_layer` | `P × (B × (c + 1) + 2)` |
| Fill, bucket, gradient, stroke (M3–M5) | `paint_tools::fills` | #1207's, plus `6 × box` for the display refresh |
| Brush (M6) | `begin_paint` (and mask paint) | the target layer's planes + `3P` |
| Undo / Redo (M7) | `undo`, `redo`, history jump | the composite's planes + `12P` |
| Snapshot (M8) | `history_add_snapshot` | `doc + 4P` |
| Snapshot restore (M8) | `history_restore_snapshot` | `doc + 10P` |
| Move, Free Transform (M9, M10) | `impl_transform` | unchanged (#1207) |
| Save (M11) | `save_start_rust` | `doc + 8P`, held by the worker as a `Reservation` until it lands |
| Open (M12) | `decode_psd_with` | the decode's document limit is `min(default, room × 2 / 3)`, without the default's one-allocation floor |
| Image Size | `resize_budget_allows` | unchanged rule, bounded by room instead of ¾ available |

The factors are fitted to the peak resident size each scenario adds at 4250² in
the process benchmark (`profile-tools.py --scenarios memory --peak`), rounded
up: the process counts the shell's display buffers, which the engine's heap
peaks of `tests_profiles/memory.rs` do not (New Document adds 21.4 bytes a pixel
to the process against 12.3 to the heap). A first fit to the heap peaks let the
17000² New Document through under `MemoryMax=4G`, where it was OOM-killed.
`estimates_cover_the_measured_peaks` checks each estimate against its
scenario's heap peak, so an engine change that raises a peak past its estimate
fails a test; the process benchmark is the end-to-end check. These are approximations of our own engine,
not of Photoshop.

M13 (pan and zoom) has no gate: a view change cannot be refused. Its display
buffers are counted in New Document's and Open's estimates; #1216 removes them.

## History's share

`History::byte_cap` is half the governor's ceiling (`HISTORY_SHARE = 2`), read
each time it is enforced (the reading is cached). Without a ceiling, no cap, as
before.

## Memory Usage preference

`Governor::set_usage_percent` (5–100, default 60) and a `memoryUsage` spin box on
the Performance page. The frame applies the session's value at start and on
change; the value is saved in the session JSON (`memoryUsagePercent`). CS6 asks
for a restart; ours applies at once, since the ceiling is ours to compute.

## Reporting

The status bar shows "Could not <verb>: not enough memory. Close other
documents or free memory, then try again." for each new gate, through one
`memoryRefusalMessage(verb)` helper; the fill, Move and transform messages stay.
Over the control server a refused `document new|open|save_as`, `layer_op add`,
`edit undo|redo` or `history snapshot|restore` answers
`{"error": "refused", "message": "not enough memory"}`, which the harness
records as `refused`. `status` gains `memory`: `{ceiling, used, reserved, room,
pressure, usage_percent}` (bytes, `null` when unknown).

## Bridge placement

`cxxqt_object.rs` and `frame.h` are at their size ceilings and the bridge-file
count at its budget, so the new bridge functions are declared in the fills
bridge (beside `edit_refused_for_memory`) and implemented in
`helpers/memory_report.rs`; the preference's state is the governor's, read back
through the bridge rather than mirrored in a frame member.
