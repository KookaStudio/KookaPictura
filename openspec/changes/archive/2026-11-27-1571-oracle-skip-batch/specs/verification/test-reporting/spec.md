# Spec Delta

## ADDED Requirements

### Requirement: Oracle tool requirements produce typed reported skips

Each Rust oracle test SHALL obtain its external tool through the shared typed requirement seam. When the required tool or capability is unavailable, the seam SHALL return a typed skip and emit exactly one `pictura oracle: SKIP <tool> <test> [reason]` token. The tool SHALL be `magick`, `psd-tools`, or `ag-psd`; the test SHALL combine the nextest binary identity with the Rust test name. The reason SHALL identify a missing capability when the tool family exists. A missing tool SHALL NOT be reported as a passing comparison, and a failure to append the token SHALL fail the test.

#### Scenario: A missing tool emits a named skip [oracle_skip_token]

- **WHEN** an oracle test requires a tool that is unavailable
- **THEN** the test returns through the typed skip path and emits one token naming the tool family, nextest binary, and Rust test

#### Scenario: Tool capabilities keep their family identity [oracle_capability_skip]

- **WHEN** ImageMagick lacks its RSVG coder, psd-tools lacks scikit-image, or psd-tools is too old for shallow knockout
- **THEN** the skip still names `magick` or `psd-tools` and the report preserves the capability reason

### Requirement: The unified report folds oracle skips into Rust results

The unified report SHALL parse oracle skip tokens, match each binary-qualified token to exactly one nextest JUnit `classname::name` case, and change that case from passed to skipped. The report SHALL show the tool, test name, and reason, include the skip in suite and TOTAL skipped counts, and SHALL NOT count it as passed. A token that is malformed, duplicated, matches no Rust case, matches more than one case, or contradicts a failed case SHALL make the report fail.

#### Scenario: An oracle pass becomes a named skip [oracle_skip_report]

- **WHEN** nextest reports an oracle test as passed and the skip stream names that test
- **THEN** the unified report lists it as skipped with its tool, reduces the passed count by one, and increases the skipped count by one

#### Scenario: An unmatched skip cannot hide [oracle_skip_unmatched]

- **WHEN** an oracle skip token is malformed, duplicated, unmatched, or ambiguous
- **THEN** the report names the token in an error and exits non-zero

#### Scenario: A failed oracle stays failed [oracle_skip_failure]

- **WHEN** a Rust test has both a failure and an oracle skip token
- **THEN** the failure remains visible and the report exits non-zero

### Requirement: Oracle transport batches work without changing comparisons

For each test binary and supported interpreter, the oracle transport SHALL reuse one worker process for all script requests in that binary. Requests and replies SHALL carry explicit boundaries and SHALL preserve argument order, exit status, standard output, and standard error. The Rust test SHALL own every comparison, and a wrong answer from a present tool SHALL still fail the test.

#### Scenario: One worker serves ordered requests [oracle_batch_order]

- **WHEN** two Python requests and two Node requests in one test binary receive different answers
- **THEN** each caller receives its own answer and argument order, and each interpreter's responses identify the same worker process

#### Scenario: Worker failures keep their details [oracle_batch_status]

- **WHEN** a Python or Node oracle request exits non-zero or writes to standard error
- **THEN** the caller receives the same status and output streams, and a following successful request still reaches the same worker

#### Scenario: A present wrong answer fails [oracle_teeth]

- **WHEN** the oracle tool runs successfully and returns bytes that exceed the comparison tolerance
- **THEN** the Rust comparison assertion fails with its mismatch message
