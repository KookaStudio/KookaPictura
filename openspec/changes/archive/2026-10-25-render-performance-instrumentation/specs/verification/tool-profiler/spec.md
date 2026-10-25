# Spec Delta

## ADDED Requirements

### Requirement: Layer-creation scenarios and phase breakdown

`scripts/profile-tools.py` SHALL provide `new-layer` and `new-group` scenarios.
Each scenario creates a layer or a group through the control server's
`layer_op` (`add` or `add_group`) on the profiled document and times the round
trip. With `--metrics`, the profiler SHALL launch the application with
`PICTURA_METRICS=1`, reset the totals before each scenario, read the `metrics`
snapshot after it, and print that scenario's per-phase totals next to its wall
time. Without `--metrics`, the profiler SHALL print wall time only and SHALL NOT
call `metrics`.

#### Scenario: New Layer is profiled with a breakdown

- **WHEN** the profiler runs `--scenarios new-layer --metrics`
- **THEN** it reports the round-trip time and the named phases that ran,
  including the Layers panel refresh

#### Scenario: Profiling without metrics still works

- **WHEN** the profiler runs a layer-creation scenario without `--metrics`
- **THEN** it reports wall time only and exits successfully
