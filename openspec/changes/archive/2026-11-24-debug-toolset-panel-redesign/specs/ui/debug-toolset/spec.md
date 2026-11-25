# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: The panel is small, floating and draggable

**Reason**: The panel was clamped inside the main window. It is now its own
floating window that can leave it, so the requirement is replaced by
`The panel is a floating, draggable window`.

**Migration**: `dbg_toggle_panel` carries over unchanged; `dbg_drag` now means
dragging anywhere on screen.
