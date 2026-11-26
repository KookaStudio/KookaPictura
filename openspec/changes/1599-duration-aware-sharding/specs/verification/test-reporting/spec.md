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
per-function seconds, and per-entry wall time. The store SHALL be written by
`scripts/test-report.sh` after the Qt Test layer and by the Qt-aware CI job. The
Qt-aware CI job SHALL upload the raw JUnit reports, CTest's `ctest-junit.xml`, and the
timing store as one artifact on every run, and SHALL print the longest entries
against a quarter of the layer's measured total, marking each entry as within or
over that budget.

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
