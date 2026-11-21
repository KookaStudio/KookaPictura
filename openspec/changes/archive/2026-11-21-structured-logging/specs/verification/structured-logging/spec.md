# Structured application logging

## Purpose

Persist local application diagnostics from Qt and Rust for use in troubleshooting.

## ADDED Requirements

### Requirement: Rust and Qt share a diagnostic stream

The application SHALL emit structured diagnostic records with a UTC timestamp,
severity, source, and event identity. Rust SHALL emit lifecycle outcomes for
document open, save, and GPU initialization or fallback. The Qt bridge SHALL
record the category and map Debug, Info, Warning, Critical, and Fatal to Debug,
Info, Warn, Error, and Error respectively, retaining a Fatal indicator.
Qt category rules SHALL continue to filter Qt messages. Application filters
SHALL default to Info and support per-target overrides including Debug.
Only reviewed diagnostic fields SHALL reach persistent output. Paths, filenames,
document titles, layer names, user text, and raw Qt or dependency message text
SHALL be excluded from the file sink. History Log records SHALL be excluded
from application diagnostics.

#### Scenario: Both languages reach the same stream [log_shared_stream]

- **WHEN** a document operation emits a Rust diagnostic and Qt emits nonfatal messages at known severities and categories
- **THEN** both sources appear in the file with UTC timestamps, event identity, and the expected severity and category fields

#### Scenario: Application and Qt category filters apply [log_filtering]

- **WHEN** a target filter enables Debug for one target and a Qt category rule disables one category
- **THEN** the selected Debug events appear and events filtered by either source are absent

#### Scenario: Private content stays out of diagnostics [log_private_fields]

- **WHEN** a document operation, a dependency message, a Qt warning, and History Log entries contain identifiable fixture data
- **THEN** the file retains the diagnostic event identities and omits the fixture paths, names, content, and History Log entries

### Requirement: File logging is bounded and follows application state

The editor SHALL write under `$XDG_STATE_HOME/kooka-pictura/logs`, honoring
`--state-home` and the normal state-home fallback. Self-tests SHALL use their
temporary state root. MCP stdio, theme dump, and GPU probe child modes SHALL
reserve stdout for their existing protocol output and use stderr for diagnostics.
The file sink SHALL buffer writes off the caller thread, drain and flush accepted
records on orderly exit within two seconds when the writer is responsive,
rotate daily in UTC, and retain files from at most the seven most recent UTC
dates. Simultaneous launches SHALL produce intact, attributable records.
When the file sink cannot open or write, the editor SHALL continue with stderr
and report the sink failure once per launch. Queue saturation SHALL drop
diagnostic events without blocking editor operations. If shutdown drain fails
or exceeds its deadline, the application SHALL report the incomplete drain
directly to stderr and exit without waiting for the stalled worker or writing
the failure notice to stdout. A Qt fatal event SHALL
reach stderr synchronously and Qt SHALL retain its normal process exit behavior.

#### Scenario: Logs rotate and concurrent launches remain readable [log_rotation]

- **WHEN** concurrent launches emit diagnostics across a simulated UTC rollover with older files present
- **THEN** each persisted record is intact and attributable, and the sink retains the latest seven UTC dates

#### Scenario: Startup modes preserve isolation and protocols [log_startup_modes]

- **WHEN** the editor uses an explicit state home, runs a self-test, and starts in MCP stdio, theme dump, and GPU probe child modes
- **THEN** each launch respects its designated state root and protocol output remains parseable

#### Scenario: Sink failures and floods keep the editor responsive [log_sink_failure]

- **WHEN** log directory creation or a later file write fails, or the diagnostic queue fills while its writer is stalled
- **THEN** the editor remains responsive, reports each launch's sink failure once on stderr, and continues to emit diagnostics there when the file sink has failed

#### Scenario: Shutdown flushes and Qt fatal remains visible [log_shutdown]

- **WHEN** one process exits normally after an accepted log write, a second exits with its writer stalled past the deadline, and a third emits a Qt fatal event
- **THEN** the first flushes its record, the second exits within the bounded drain and reports the incomplete drain only on stderr, and the third emits an error with the fatal marker on stderr and terminates abnormally
