## ADDED Requirements

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
