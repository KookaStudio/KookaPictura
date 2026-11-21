# Spec Delta

## MODIFIED Requirements

### Requirement: Metrics readback

The server SHALL provide a `metrics` method. When the application runs with
metrics on (`verification/performance-metrics`), the method SHALL return
`{enabled: true, phases, counters, edits, frames, frame_records_evicted}`:

- `phases` maps each phase label to `{count, total_ms, max_ms}`.
- `counters` holds the GPU resident hits and misses and the decline counts by
  reason.
- `edits` lists the most recent edit records, newest last.
- `frames` lists up to the 1,024 most recent document canvas frame records
  across the application, in capture order. Each record SHALL contain
  `canvas_id`, `boundary` (`canvas_paint`), `input_count`, `input_ms`,
  `tile_gather_ms`, `qt_draw_ms`, `paint_ms`, `cpu_budget_slack_ms`, and
  `gpu_present_ms` (`null`). `interval_ms` and `gpu_composite_ms` SHALL be
  included when those measurements are available. `gpu_composite_ms` is the
  sum of qualifying device intervals. Each runs from the beginning of the first
  blend pass through the end of the last blend pass and excludes later planar
  conversion, readback copy, and window presentation. `canvas_id` SHALL be
  stable for the lifetime of the canvas.
- `frame_records_evicted` counts records displaced by the frame history limit
  since the last reset.

With `reset: true`, the method SHALL take the snapshot before clearing the
phase totals, edit records, frame records, and frame eviction count. The next
frame for each canvas SHALL start a fresh interval baseline and SHALL exclude
input and GPU timing samples collected before the reset. When metrics are off,
the method SHALL return the error code `refused`.

#### Scenario: Snapshot after an edit

- **WHEN** metrics are on and a client calls `metrics` after one New Layer
- **THEN** the response contains a `New Layer` edit record and non-empty phases

#### Scenario: Reset clears the totals

- **WHEN** a client calls `metrics` with `reset: true` and then calls it again
  without editing
- **THEN** the second response has no phases and no edit records

#### Scenario: Disabled metrics are refused

- **WHEN** metrics are off and a client calls `metrics`
- **THEN** the response is an error with code `refused`

#### Scenario: Frame history is bounded and resettable [fbi_frame_readback]

- **WHEN** more than 1,024 frame records are captured and the client reads
  `metrics` with `reset: true`
- **THEN** it reads the newest 1,024 records in capture order and the eviction
  count, the subsequent snapshot starts with an empty history and zero evictions,
  and the first new paint on each canvas carries no prior interval or timings
