# Spec Delta

## ADDED Requirements

### Requirement: Metrics readback

The server SHALL provide a `metrics` method. When the application runs with
metrics on (`verification/performance-metrics`), the method SHALL return
`{enabled: true, phases, counters, edits}`:

- `phases`: maps each phase label to `{count, total_ms, max_ms}`;
- `counters`: holds the GPU resident hits and misses and the decline counts by
  reason;
- `edits`: lists the most recent edit records, newest last.

With `reset: true`, the method SHALL clear the phase totals and the edit records
after reading them. When metrics are off, the method SHALL return the error code
`refused`.

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

### Requirement: Group creation over layer_op

`layer_op` SHALL accept op `add_group`, which creates an empty group above
`index` through the same application method as the Layers panel's New Group
button. It SHALL return `{ok, layers}`. An out-of-range `index` SHALL return
`invalid_param`.

#### Scenario: A group is added

- **WHEN** a client invokes `layer_op` with op `add_group` and an in-range index
- **THEN** the result reports one more top-level node, and that node is a group
