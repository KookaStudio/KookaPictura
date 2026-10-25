# verification/tool-profiler Specification

## Purpose
An end-to-end profiler that times tools in the real app through the control
server and compares builds before and after a change.

## Requirements

### Requirement: An end-to-end tool profiler

The repository SHALL provide `scripts/profile-tools.py`, a standard-library
Python script that launches the built app in headless control mode, opens an
image or creates a document of a given size, and drives each named tool
scenario (brushes at several sizes, Pencil, Eraser, the shape tools in Shape
and Pixels mode, Marquee, Gradient, Paint Bucket) through the control server as
a user's pointer would: a `down`, one request per `move`, and an `up`. It SHALL
time every request on the client and report, per scenario, the press, the
median, 95th-percentile and worst move, the release, the work deferred past
the release, the total, and an undo. It SHALL save the results as JSON and
print a before/after table against an earlier save, and SHALL keep the app's
`PICTURA_PAINT_TIMING` phase log when asked.

#### Scenario: Before and after a change [prof_compare]

- **WHEN** the profiler is run on one build with `--save`, and on a later build
  with `--compare` against that file
- **THEN** it prints each scenario's timings and a table of before → after
  times with the speed-up

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
