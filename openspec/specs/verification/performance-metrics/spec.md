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
after it. Counting an eligibility decline SHALL NOT require a GPU device. When
metrics are on, the device supports timestamp queries, and a GPU composite runs,
the composite SHALL report its device-side execution time. Otherwise the device
time SHALL be absent, never reported as zero. The application SHALL request
timestamp-query support from the device only when metrics are on.

#### Scenario: A decline is attributed

- **WHEN** the GPU path declines a document because a visible layer uses
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

### Requirement: Canvas frame timing measures through paint completion

When `PICTURA_METRICS` is on, each completed document canvas paint SHALL
produce one record with boundary `canvas_paint`. This includes paints caused
by panning, zooming, rotating, and the arrival of visible tiles. Explicit
control screenshot renders SHALL produce no record and SHALL NOT consume pending
input or GPU samples or advance the canvas cadence baseline. Each record
SHALL identify the canvas and report the count and summed duration of pending
viewport input operations handled for that canvas before the paint. Input
operations coalesced into one paint SHALL be counted once each. A paint with
no pending viewport input SHALL report zero input operations and zero input
time.

The record SHALL report elapsed paint time, time spent gathering and assembling
visible tiles during the paint, and the remaining Qt draw time. Tile gather
time SHALL include synchronous crop and cache work and exclude asynchronous
work outside the paint. Qt draw time SHALL equal paint time minus tile gather
time. Signed CPU budget slack SHALL equal 16.6 ms minus input time and paint
time. Paint-to-paint interval SHALL measure the time since the previous
recorded completion on the same canvas and SHALL be absent for the first paint
of a capture. Display presentation time SHALL be marked unavailable.

#### Scenario: Viewport input coalesces into one timed paint [fbi_coalesced_input]

- **WHEN** metrics are on and several pan, zoom, or rotate operations update
  the same document canvas before Qt handles one paint
- **THEN** that paint produces one record containing those operations, with CPU
  phase durations and signed budget slack consistent with the measured boundary

#### Scenario: Paint cadence belongs to the canvas [fbi_cadence]

- **WHEN** two canvases paint in alternation and one paints again with no new input
- **THEN** its next record measures the interval from its own prior paint,
  reports zero input, and marks display presentation time unavailable

#### Scenario: Late tiles produce a separate paint [fbi_late_tiles]

- **WHEN** a viewport update paints while needed tiles are building asynchronously,
  and the completed tiles trigger a later paint
- **THEN** the first record excludes the worker's time, and the later paint has
  its own record without counting the viewport input again

#### Scenario: Screenshot renders do not consume a frame [fbi_screenshot]

- **WHEN** a control screenshot grabs either the frame or canvas while input and
  a GPU sample are pending
- **THEN** its offscreen paint produces no frame and changes no cadence baseline,
  and the next eligible canvas paint consumes the pending timings

### Requirement: GPU composite duration follows the painted result

An optional frame device duration SHALL report timestamped GPU composite
work whose result was newly applied to pixels drawn by that paint. The interval
starts at the beginning of the first blend pass and ends at the end of the last
blend pass. It excludes later planar conversion, readback copy, and window
presentation. Attribution
SHALL intersect actual supplied source spans with the effective paint event
region and content clip after the relevant transforms. A GPU
duration sample SHALL be counted at most once, including when one composite
result supplies multiple drawn tiles. If several samples qualify, their
durations SHALL be summed. The GPU value SHALL be absent when no attributable
sample is available, including reused pixels, CPU fallback, unavailable
timestamps, or results rejected or superseded before drawing. Device duration
SHALL be reported independently of CPU phase times and budget slack.

#### Scenario: A visible GPU result carries its timing once [fbi_gpu_result]

- **WHEN** one timestamped GPU composite supplies multiple newly drawn tiles
  and the canvas subsequently repaints those tiles from cache
- **THEN** the first paint reports the device duration once and the cache-only
  paint omits it

#### Scenario: Partial damage leaves disjoint samples pending [fbi_gpu_partial]

- **WHEN** a paint event draws a region disjoint from a pending repaired source
  span in the same visible tile or crop, and a later paint draws that span
- **THEN** the first paint omits the sample and the later paint reports it once

#### Scenario: Reduced stand-in attribution [fbi_gpu_standin]

- **WHEN** a timestamped reduced-level stand-in is accepted and drawn, or is
  overwritten or never drawn
- **THEN** only the paint drawing accepted stand-in pixels reports its sample

#### Scenario: Unrelated and unavailable GPU timings are absent [fbi_gpu_absent]

- **WHEN** a canvas paints after CPU fallback, unavailable timestamps, or GPU
  work whose result was unrelated, rejected, or superseded
- **THEN** its frame record omits GPU composite duration

### Requirement: Canvas frame timing is inert when disabled

With `PICTURA_METRICS` off, the frame instrumentation SHALL read no clocks,
retain no timing state, and emit no frame records. The metrics setting SHALL
preserve canvas pixels, composited pixels, and saved bytes.

#### Scenario: Disabled canvas frame capture [fbi_disabled]

- **WHEN** equivalent viewport operations and an edit run with metrics off and
  on
- **THEN** the drawn pixels and saved bytes match across runs, and the disabled
  run retains no frame records
