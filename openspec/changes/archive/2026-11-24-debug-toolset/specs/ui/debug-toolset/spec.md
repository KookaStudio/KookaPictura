# Spec Delta

## ADDED Requirements

### Requirement: Help exposes a Debug Toolset toggle

Help ▸ Developer SHALL contain a checkable `Debug Toolset` command, enabled with
no document open. The toolset SHALL be off at every launch, and its state and the
panel position SHALL NOT be persisted.

#### Scenario: The command is checkable and off at startup [dbg_menu_entry]

- **WHEN** the main window is constructed with no document
- **THEN** `help.developer.debugToolset` is enabled, checkable and unchecked, and
  no debug panel is visible

#### Scenario: State is not persisted [dbg_not_persisted]

- **WHEN** the toolset is enabled, the panel is moved, and a new main window is
  constructed
- **THEN** the new window has the toolset off and writes no setting for it

### Requirement: Disabled means no collection

With the toolset off the application SHALL run no refresh timer for it and read
no metric source on its behalf, and SHALL retain no earlier sample.

#### Scenario: Off runs nothing [dbg_off_no_collection]

- **WHEN** the toolset is off, and again after it was enabled then disabled
- **THEN** the refresh timer is inactive and the sampler holds no previous sample

### Requirement: The panel is small, floating and draggable

Enabling the toolset SHALL show one panel inside the main window, above the docks,
and disabling it SHALL hide it. The panel SHALL move when its header is dragged and
SHALL stay inside the main window.

#### Scenario: Toggle shows and hides the panel [dbg_toggle_panel]

- **WHEN** `help.developer.debugToolset` is triggered twice
- **THEN** the panel is visible after the first and hidden after the second, and
  the command's checked state follows

#### Scenario: The header drags the panel inside the window [dbg_drag]

- **WHEN** the header is dragged past the window edge
- **THEN** the panel moves with the pointer and its geometry stays inside the
  window

### Requirement: Metrics refresh every two seconds

While on, the panel SHALL refresh all values every 2000 ms. A value that needs two
samples SHALL read `…` until the second sample.

#### Scenario: The interval is two seconds [dbg_interval]

- **WHEN** the toolset is on
- **THEN** the refresh timer interval is 2000 ms, and a refresh updates every row

#### Scenario: Rate rows wait for a second sample [dbg_first_sample]

- **WHEN** the first refresh after enabling completes
- **THEN** CPU and swap I/O rows read `…`, and have numbers after the second refresh

### Requirement: Performance group describes this process

The panel SHALL show Pictura's RAM, swap, CPU, thread count, GPU, and CPU share per
thread group (GUI, rayon workers, open/save/filter/document jobs, other).

#### Scenario: Process rows parse from the kernel text [dbg_process_parse]

- **WHEN** `/proc/self/status` and `/proc/self/stat` fixtures are parsed
- **THEN** RAM, swap and thread count equal the fixture values and CPU equals the
  tick delta over the wall delta

#### Scenario: GPU reports what Pictura measures [dbg_gpu]

- **WHEN** a metrics snapshot with a GPU frame time is formatted, and when one
  without is formatted
- **THEN** the first shows that time in milliseconds and the second shows
  `not measured`

#### Scenario: Threads group by name and rank by CPU [dbg_thread_groups]

- **WHEN** task fixtures named `pictura-rayon-3`, `pictura-open`, the pid thread and `QXcbEvent`
  are grouped
- **THEN** they land in rayon, jobs, GUI and other, ordered by CPU delta

#### Scenario: Workers carry group names [dbg_thread_names]

- **WHEN** a rayon worker and an open job run
- **THEN** their thread names start with `pictura-rayon-` and `pictura-open`

### Requirement: Platform group describes the machine

On Linux the panel SHALL show OS and architecture, desktop environment and session
type, system memory total, used and cached, swap used, total, location and I/O,
per-CPU usage, and load average.

#### Scenario: Memory and swap parse [dbg_memory_swap_parse]

- **WHEN** `/proc/meminfo`, `/proc/swaps` and two `/proc/vmstat` fixtures are parsed
- **THEN** total, used (total − available), cached, swap used and total, the first
  swap path and the swap I/O rate match the fixture arithmetic

#### Scenario: CPU and load parse [dbg_cpu_load_parse]

- **WHEN** two `/proc/stat` fixtures and a `/proc/loadavg` fixture are parsed
- **THEN** each CPU's percentage equals its non-idle delta over its total delta and
  the three load averages match

#### Scenario: Desktop rows are present [dbg_platform_rows]

- **WHEN** the panel refreshes
- **THEN** the OS, architecture, desktop and session rows are non-empty

### Requirement: Version group shows a build-number placeholder

The panel SHALL show a Version row whose value is a placeholder until build numbers
exist.

#### Scenario: The placeholder shows [dbg_build_placeholder]

- **WHEN** the panel is shown
- **THEN** the Version row reads `n/a (placeholder)`
