# Proposal

## Why

Memory, CPU and GPU behaviour of large documents is diagnosed today by launching
with `PICTURA_METRICS`, reading stderr, or attaching gdb. Developers and testers
have no live, in-app view of what the process and the machine are doing. Issue
#1589 asks for a small panel that shows it.

## What Changes

- **Toggle.** `Help ▸ Developer ▸ Debug Toolset` is a checkable command. It is
  off at every launch, its state is not saved, and it is turned on by hand.
- **Panel.** When on, a small floating panel (draggable by its header, kept
  inside the main window) shows three groups, refreshed every 2 seconds:
  - *Performance* (this process): RAM, swap, CPU, thread count, GPU, and CPU
    time grouped by thread name.
  - *Platform* (Linux only for now): OS and architecture, desktop environment
    and session type, system memory (total, used, cached), swap (used, total,
    location, I/O), per-CPU usage, load average.
  - *Version*: a build-number placeholder until build numbers exist.
- **Off means off.** With the toolset off there is no timer and no `/proc`
  read for the panel; nothing is retained.
- **GPU.** Pictura has no portable GPU-utilisation query, so the row shows what
  Pictura itself measures: the last GPU frame time, available only when the
  process started with `PICTURA_METRICS` (GPU timestamps change device
  creation and cannot be switched on later). Otherwise it reads "not measured".
- **Attribution.** Engine crates do not own threads, so "which part uses the most"
  is answered by thread group: GUI, rayon workers (shared by every engine crate),
  open/save/filter/document jobs, other. The app's job threads and rayon workers
  get names so the groups exist. Per-crate attribution is out of scope.

## Capabilities

### New Capabilities

- `ui/debug-toolset`: the toggle, the floating panel, its metrics and their
  sources.

### Modified Capabilities

<!-- None. -->

## Impact

- New `crates/pictura-app/cpp/debug_toolset.{h,cpp}` (pure `/proc` parsers and
  the sampler) and `debug_toolset_panel.{h,cpp}` (the floating panel), listed in
  `cmake/sources/*.cmake`.
- `command_ids.h`, `frame.h`, `frame_commands.cpp`, and the `pictura-script`
  command catalogue plus its goldens: the new command.
- `crates/pictura-app/src`: names for job threads and the rayon global pool.
- New `crates/pictura-app/cpp/tests/tst_debug_toolset.cpp`.
- `ROADMAP.md`, `PARITY.md`: a post-CS6 developer tool, labelled non-parity.
- No new dependency; no engine-crate change.
