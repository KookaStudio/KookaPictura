# Design

## Sampling

`DebugSampler` (C++, `debug_toolset.cpp`) turns `/proc` text into a `DebugSnapshot`.
Every parser is a pure function of a string so tests feed fixtures:

| Row | Source |
|---|---|
| Process RAM, swap, threads | `/proc/self/status` (`VmRSS`, `VmSwap`, `Threads`) |
| Process CPU % | `/proc/self/stat` utime+stime delta over wall delta, of one core |
| System memory | `/proc/meminfo` (`MemTotal`, `MemAvailable`, `Cached`, `Buffers`); used = total − available |
| Swap used / total | `/proc/meminfo` (`SwapTotal`, `SwapFree`) |
| Swap location | `/proc/swaps` (first column) |
| Swap I/O | `/proc/vmstat` `pswpin`/`pswpout` delta × page size ÷ seconds |
| Per-CPU % | `/proc/stat` `cpuN` lines, delta of non-idle over total |
| Load average | `/proc/loadavg` |
| Thread groups | `/proc/self/task/<tid>/{comm,stat}` CPU delta, grouped by comm prefix |
| OS, arch, desktop, session | `collectSystemInfo({})`, read once when the toolset turns on |

Rate rows need two samples; the first refresh after enabling shows `…` for them.

## Off means off

The panel owns a 2000 ms `QTimer`, started on enable and stopped on disable. The
sampler keeps its previous-sample state in the panel; disabling drops it. Nothing
runs while off, and there is no global flag for engine code to check: the engine
is never asked to measure for the panel.

## GPU

`PictureView::metrics_snapshot` already serves the last frame's `gpu_ms` when
`PICTURA_METRICS` is set. The panel reads it only when the toolset is on. Without
`PICTURA_METRICS` the row reads "not measured (start with PICTURA_METRICS=1)".

## Thread groups

`/proc/<tid>/comm` truncates to 15 characters, so names are short prefixes:
`pictura-rayon-N` (rayon global pool via `thread_name`), `pictura-open`, `pictura-save`,
`pictura-filter`, `pictura-doc`. The GUI thread is the one whose tid equals the pid. Anything
else (Qt, GPU driver, glibc) is `other`. The panel lists groups by CPU share,
highest first. ponytail: groups are by role, not by crate, because rayon workers
serve every crate.

## Panel

A frameless `QFrame` child of the main window (not a top-level), raised above the
docks, initial position top-right over the panel column as in the issue's
screenshot. Dragging the header moves it and clamps it inside the window. Position
is not saved. Styled by the global stylesheet; no new colours.

## Command

`help.developer.debugToolset`, checkable, label "Debug Toolset", under
Help ▸ Developer. Handler toggles the panel; the checked provider reports its
visibility. Listed in the `pictura-script` catalogue so scripts and the control
server see it.

## Build number

A single constant string `"n/a (placeholder)"` shown on the Version row, replaced
when build numbers exist.
