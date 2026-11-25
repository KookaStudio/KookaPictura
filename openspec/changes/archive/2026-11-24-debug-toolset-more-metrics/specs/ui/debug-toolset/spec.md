# Spec Delta

## ADDED Requirements

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
