# ui/debug-toolset Specification

## Purpose
The Help > Developer > Pictura Metrics toggle and the floating metrics panel it
shows: process and platform readings refreshed every 2 seconds, with nothing
collected while it is off.

## Requirements

### Requirement: Help exposes a Pictura Metrics toggle

Help ▸ Developer SHALL contain a checkable `Pictura Metrics` command, enabled with
no document open. The toolset SHALL be off at every launch, and its state and the
panel position SHALL NOT be persisted.

#### Scenario: The command is checkable and off at startup [dbg_menu_entry]

- **WHEN** the main window is constructed with no document
- **THEN** `help.developer.debugToolset` is enabled, checkable and unchecked, its menu
  text reads `Pictura Metrics`, and no debug panel is visible

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

The panel SHALL title this group `Pictura Resource Usage` and show Pictura's RAM,
swap, CPU, thread count, GPU, and CPU share per thread group (GUI, rayon workers,
open/save/filter/document jobs, other).

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

- **WHEN** task fixtures named `pictura-rayon-3`, `pictura-open`, the pid thread and
  `QXcbEvent` are grouped
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

### Requirement: The panel is light yellow with dark text

The panel SHALL paint a light-yellow background with dark text in every theme.

#### Scenario: Colours are light yellow and dark [dbg_appearance]

- **WHEN** the toolset is on
- **THEN** a rendered pixel of the panel background is light yellow and the row
  text colour is dark

### Requirement: Metrics are shown in titled tables

Each group SHALL be a bordered table with a title band and one row per metric, a
label cell beside a value cell, so every value sits in the row of its metric and in
the table of its group.

#### Scenario: Every row sits in its group's table [dbg_table]

- **WHEN** the panel is shown
- **THEN** the groups are titled `Pictura Resource Usage`, `Platform` and
  `Version`, and each metric's label and value are in their group's table

### Requirement: The panel has a close control

The panel SHALL have a close button at its top right. Activating it SHALL turn the
toolset off: the timer stops, no sample or chart history is retained, the panel
hides, and the Help menu item is unchecked.

#### Scenario: Close stops collection [dbg_close]

- **WHEN** the close button is clicked
- **THEN** the panel is hidden, the timer inactive, the sampler and chart histories
  empty, and `help.developer.debugToolset` unchecked

### Requirement: The panel shows a drag icon and a resize grip

The panel SHALL show a drag icon centred at the top and a size grip in the
bottom-right corner, and SHALL be resizable by that grip.

#### Scenario: Drag icon is at the top centre [dbg_drag_icon]

- **WHEN** the panel is shown
- **THEN** a drag icon is visible in the header, horizontally centred on the panel

#### Scenario: Resize grip is in the bottom-right corner [dbg_resize_grip]

- **WHEN** the panel is shown and then resized larger
- **THEN** a visible size grip sits in the bottom-right corner, and the panel
  takes the new size

### Requirement: RAM and CPU history charts

Under the Busiest row the panel SHALL show a RAM line chart and a CPU line chart of
Pictura's last 60 samples, newest at the right, retained only while the toolset is
on.

#### Scenario: Charts sit under Busiest and grow with samples [dbg_charts]

- **WHEN** the panel refreshes three times
- **THEN** both charts are visible below the Busiest row, the RAM series has three
  points and the CPU series two, and after 70 refreshes each holds at most 60

### Requirement: The panel is a floating, draggable window

Enabling the toolset SHALL show one panel as its own tool window above the main
window, and disabling it SHALL hide it. The panel SHALL move when its header is
dragged, to any place on screen including outside the main window, and SHALL
close with the main window.

#### Scenario: Toggle shows and hides the panel [dbg_toggle_panel]

- **WHEN** `help.developer.debugToolset` is triggered twice
- **THEN** the panel is visible after the first and hidden after the second, and
  the command's checked state follows

#### Scenario: The header drags the panel anywhere [dbg_drag]

- **WHEN** the header is dragged far beyond the main window's left edge
- **THEN** the panel follows the pointer and ends outside the main window

#### Scenario: The panel is a floating window [dbg_float_window]

- **WHEN** the toolset is on
- **THEN** the panel is a top-level tool window whose parent is the main window

### Requirement: Per-CPU usage is a heat matrix

The Platform group's `CPUs` row SHALL show one square per logical CPU, coloured from
cold to hot by that CPU's usage, with no numbers. The squares SHALL form one row for
8 or fewer CPUs and two rows for more. Before the second sample the row SHALL read
`…`, and it SHALL be emptied when the toolset turns off.

#### Scenario: Colour runs from cold to hot [dbg_cpu_heat_colours]

- **WHEN** a heat matrix is given loads of 0 %, 50 % and 100 %
- **THEN** the 0 % square is blue, the 100 % square red, and the 50 % square differs
  from both

#### Scenario: One or two rows by CPU count [dbg_cpu_heat_rows]

- **WHEN** a heat matrix is given 4, 8, 9 and 16 loads
- **THEN** it has 1, 1, 2 and 2 rows, and every load has a square inside the widget

#### Scenario: The panel feeds the matrix from the sampler [dbg_cpu_heat_panel]

- **WHEN** the panel has taken one sample, then a second
- **THEN** the matrix is empty after the first and holds one load per CPU in
  `/proc/stat` after the second, and is empty again after the toolset is turned off

### Requirement: Kernel activity rows

The panel SHALL show, for this process, RAM split into anonymous and file-backed,
open file count, disk read and write rate, minor and major page-fault rate, and
voluntary and involuntary context-switch rate (summed over all threads).

#### Scenario: Activity values parse from the kernel text [dbg_activity_parse]

- **WHEN** status, stat, io, task-status fixtures and a file-descriptor count are
  sampled twice
- **THEN** anonymous and file RAM, the file count, and each rate equal the fixture
  arithmetic

#### Scenario: The panel shows the activity rows [dbg_activity_rows]

- **WHEN** the panel has taken two samples
- **THEN** the files, disk, faults and context-switch rows are non-empty and none
  reads `…`

### Requirement: Background jobs in flight

The panel SHALL list the open, save, filter and document jobs running now with how
long each has run.

#### Scenario: Jobs come from named threads [dbg_jobs]

- **WHEN** task fixtures named `pictura-open` and `pictura-save`, started 12 s and 3 s
  before the reading, and a GUI thread are sampled
- **THEN** the jobs are Open at 12 s and Save at 3 s, longest first, and an
  otherwise empty list reads `none`

### Requirement: Memory budget group

The panel SHALL show a `Pictura Memory Budget` table with the memory governor's used
and ceiling and its pressure, the scratch disk's use and free space, and the packed,
spilled and backed tile counts, read from the governor's status report.

#### Scenario: The status report parses [dbg_budget_parse]

- **WHEN** a status report with governor, scratch and tile fields is parsed, and when
  an empty one is
- **THEN** the first yields every value and the second yields none

#### Scenario: The budget rows show the report [dbg_budget_panel]

- **WHEN** the panel samples with a report whose used is 3 GiB of a 6 GiB ceiling,
  pressure `elevated`
- **THEN** the governor row shows `3.0 GB` of `6.0 GB` and `elevated`, and without a
  report the rows read `n/a`

### Requirement: GPU use of this process and of the system

The panel SHALL show this process's GPU engine busy percentage and GPU memory from
the DRM client information of its open render nodes, and the system GPU's busy
percentage and video memory from sysfs, and SHALL read `n/a` where the driver offers
neither.

#### Scenario: DRM client information parses [dbg_gpu_fdinfo_parse]

- **WHEN** two fdinfo fixtures with the same client id and one with another are
  sampled twice with 500 ms of engine time in 1 s
- **THEN** memory counts each client once and engine busy is 50 %

#### Scenario: System GPU sysfs parses [dbg_gpu_system_parse]

- **WHEN** busy-percent and VRAM used and total fixtures are sampled
- **THEN** the busy percentage and the used and total memory equal the fixtures, and
  empty fixtures give no system GPU

### Requirement: Edit and paint timing

The panel SHALL show the 50th and 95th percentile and the latest of the recent edit
wall times and canvas paint times when the process was started with
`PICTURA_METRICS`, and `not measured` otherwise.

#### Scenario: Percentiles come from the metrics snapshot [dbg_timing]

- **WHEN** a snapshot with edits of 10, 20, 30, 40 ms and frames is formatted, and
  when an empty one is
- **THEN** the edit text shows p50 25.0 ms, p95 38.5 ms and last 40.0 ms, and the
  empty one shows `not measured`

### Requirement: Thread-group history chart

Under the RAM and CPU charts the panel SHALL show a line chart of CPU per thread
group over the last 60 samples, one coloured line per group with a legend, retained
only while the toolset is on.

#### Scenario: Group lines grow with samples [dbg_group_chart]

- **WHEN** the panel has taken three samples, and again after the toolset is turned
  off
- **THEN** the GUI group holds two points after the third sample and nothing after
  the toolset is turned off

### Requirement: The panel is titled Pictura Metrics

The panel's header SHALL read `Pictura Metrics`.

#### Scenario: The header shows the product title [dbg_title]

- **WHEN** the toolset is on
- **THEN** the panel's title label reads `Pictura Metrics`

### Requirement: Tables are laid out in two columns

The panel SHALL place Pictura Resource Usage and Version in a left column, and Pictura
Memory Budget, Pictura Activity and Platform in a right column to its right, so the
panel is wider than the sum of its parts is tall.

#### Scenario: Budget, activity and platform sit right of usage [dbg_two_columns]

- **WHEN** the panel is shown
- **THEN** the Memory Budget, Activity and Platform tables start at or beyond the right
  edge of the Resource Usage table, the Version table is in the left column, and the
  panel is shorter than the stacked tables would be
