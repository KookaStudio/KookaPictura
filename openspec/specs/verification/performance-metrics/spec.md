# verification/performance-metrics Specification

## Purpose
Opt-in runtime metrics: where interactive time goes and how large the composite
canvases are. They let a performance fix be measured before and after, and they
never change what the application computes.

## Requirements

### Requirement: Metrics are opt-in and inert when off

The application SHALL report metrics only when the `PICTURA_METRICS`
environment variable is set to a non-zero integer at startup. The existing
`PICTURA_PAINT_TIMING` variable SHALL keep enabling the per-stroke report it
enables today. When metrics are off, the application MUST NOT print metric
lines, and it MUST NOT retain edit records or phase totals. Whether metrics are
on or off, every composite and every saved file MUST be byte-identical.

#### Scenario: Off by default

- **WHEN** the metrics level is off and an edit commits
- **THEN** no metric line is printed and no edit record or phase total is retained

#### Scenario: Metrics do not change output

- **WHEN** the same document receives the same edit sequence once with metrics on
  and once with metrics off
- **THEN** the two final composites are byte-identical

#### Scenario: The stroke report survives

- **WHEN** the per-stroke report is enabled and a stroke session records phases
- **THEN** the report lists those phases under their existing labels

### Requirement: Named phase timings

When metrics are on, the application SHALL accumulate, for each named phase, its
call count, total duration, and maximum duration. The instrumented phases SHALL
include at least the full composite, the history capture, the display-pyramid
reset, and the Layers panel refresh. The panel refresh is timed on the Qt side
and recorded through the same accumulator.

#### Scenario: A full recomposite is counted

- **WHEN** metrics are on and an edit triggers a full recomposite
- **THEN** the full-composite phase's count increases by one

#### Scenario: A Qt-side phase is recorded

- **WHEN** metrics are on and the Qt side records a phase through the bridge
- **THEN** the phase appears in the totals with its count and duration

### Requirement: Edit records

When metrics are on, each committed edit SHALL produce one record. The record
SHALL carry the edit's history label, whether the compositor ran, the wall time
of the edit from the start of its mutation to the end of the transaction, the
named phases that ran inside it, and the largest composite canvas allocated
during it, in bytes. The record SHALL be closed when
the transaction returns. Qt-side work queued after that point is reported only
in the phase totals.

#### Scenario: One record per committed edit

- **WHEN** metrics are on and New Layer commits
- **THEN** exactly one edit record labelled `New Layer` is produced

#### Scenario: An edit with no composite says so

- **WHEN** metrics are on and an edit commits without running the compositor
- **THEN** its record reports that no composite ran and lists no composite phase

#### Scenario: Canvas bytes for a full composite

- **WHEN** metrics are on and a full CPU composite of a W×H document small
  enough to composite as one band runs inside an edit
- **THEN** the edit record's canvas bytes are at least W×H×4

### Requirement: GPU counters

The render crate SHALL count resident-upload cache hits and misses, and SHALL
count every `GpuError` a GPU composite returns, by kind: eligibility declines
before the availability gate, `Unavailable` at the gate, and device errors
after it. Counting an eligibility decline SHALL NOT require a GPU device. When metrics are on, the device supports timestamp queries, and a
GPU composite runs, the composite SHALL report its device-side execution time.
Otherwise the device time SHALL be absent, never reported as zero. The
application SHALL request timestamp-query support from the device only when
metrics are on.

#### Scenario: A decline is attributed

- **WHEN** `composite_gpu` declines a document because a visible layer uses
  `Dissolve`
- **THEN** the decline count for the unsupported-mode reason increases

#### Scenario: No timestamps, no fake zero

- **WHEN** a GPU composite timing has no device timestamps
- **THEN** its report line carries no device-time field

### Requirement: Metrics reporting

When metrics are on, the application SHALL print each edit record and each
panel-refresh phase on stderr as single lines prefixed `pictura metrics:`, with
space-separated `key=value` fields. The control server SHALL provide a
`metrics` method, specified in `interop/agentic-control`, that returns the phase
totals, the GPU counters, and the most recent edit records.

#### Scenario: An edit record line

- **WHEN** metrics are on and an edit commits
- **THEN** one `pictura metrics: edit label=<label> ...` line is printed
