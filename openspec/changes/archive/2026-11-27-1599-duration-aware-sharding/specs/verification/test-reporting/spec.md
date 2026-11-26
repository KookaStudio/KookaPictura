# Spec Delta

## ADDED Requirements

### Requirement: Qt Test duration evidence is retained and published

The reporter SHALL keep the duration it already parses from each Qt Test JUnit
report instead of discarding it: the Qt layer SHALL carry each suite's `time`
attribute, and the rendered report SHALL print that duration beside the suite's
counts when it is non-zero. A missing or malformed `time` SHALL read as zero and
SHALL NOT fail the report.

`scripts/qt_test_timings.py` SHALL collect every per-entry JUnit report and,
when present, CTest's per-entry `--output-junit` wall time into one
timing store at `build/qt-test-timings.json`, recording per-suite seconds,
per-function seconds, and per-entry wall time. The shared `scripts/test-report.sh`
executor SHALL write the store and print the longest entries after the Qt Test
layer in both local and CI runs. The Qt-aware CI job SHALL upload the raw JUnit reports, CTest's `ctest-junit.xml`, and the
timing store as one artifact on every run. The longest-entry report SHALL mark
each entry as within or over a quarter of the layer's measured total.

#### Scenario: The report keeps suite durations [tr_qt_durations]

- **WHEN** the reporter renders a Qt Test layer whose JUnit reports carry `time` attributes
- **THEN** each suite's seconds appear beside its counts, and a report without any `time` attribute still renders with zero durations and exits 0

#### Scenario: The timing store is written after the layer runs [tr_qt_timing_store]

- **WHEN** `scripts/test-report.sh` finishes the Qt Test layer with at least one JUnit report
- **THEN** `build/qt-test-timings.json` holds each suite's seconds and functions and each CTest entry's execution time

#### Scenario: CI publishes the raw timing artifacts [tr_qt_timing_artifact]

- **WHEN** the Qt-aware CI job finishes, successfully or not
- **THEN** it uploads the JUnit reports, `build/ctest-junit.xml`, and `build/qt-test-timings.json` as the `qt-test-timings` artifact

#### Scenario: The longest entry is reported against the quarter budget [tr_qt_longest_entry]

- **WHEN** the timing store holds per-entry execution times
- **THEN** the reporter prints each entry's seconds and marks it within or over a quarter of the layer's measured total
