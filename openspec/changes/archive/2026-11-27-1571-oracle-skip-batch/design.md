# Design

## Context

See `proposal.md` for the reporting hole and startup cost. The current tests duplicate `OnceLock<bool>` probes, print ad hoc skip messages, and return normally. Python commands run a fresh interpreter for each case. The reporter reads nextest JUnit but not oracle output, so a returned test is counted as passed.

The integration-test directories are not fully represented in the parent code graph, so the implementation inventory comes from exact source reads. The fetched branch starts at `b40039f7`, newer than the supplied graph generation.

## Goals / Non-Goals

**Goals:**

- One typed availability path for the three oracle tool families.
- Named skips that replace, rather than supplement, the corresponding nextest pass.
- One reusable Python or Node worker per test binary and interpreter.
- Byte-for-byte preservation of command results and all existing comparison assertions.

**Non-Goals:**

- Changing tolerances, fixture bytes, or oracle algorithms.
- Adding typed availability adapters for Ghostscript, ExifTool, PIL, rawpy, or unrelated optional tools. Their existing ad hoc skips remain, but Python script invocations can still use the batch transport.
- Parallel execution inside one worker. Requests serialize so replies cannot be mismatched.
- Adding dependencies or making skipped oracles fail CI by default.

## Decisions

### Typed requirement and caller-aware macro

`pictura-testkit::oracle::Tool` identifies ImageMagick, psd-tools, and ag-psd. Capability variants for RSVG, scikit-image, and psd-tools shallow knockout keep the existing skip conditions but report under their parent tool family. `require(tool)` caches the probe and returns `Result<(), OracleSkip>`. `OracleSkip` stores the family and reason.

`require_oracle!(tool)` calls `require`, reports the skip, and returns from the calling function. `OracleSkip::report` combines the libtest thread name with the current test binary's nextest identity, `package` for a library test or `package::target` for an integration test. Helper functions that already return early keep their control flow, so the test still ends without invoking a missing tool. Option-returning helpers retain their `None` path and emit the token before returning.

The skip goes to standard error and, when `PICTURA_ORACLE_SKIPS` is set, to that append-only file. The token carries an optional reason after the binary-qualified test name. The file removes any dependence on nextest choosing to print successful-test output. A failed token append panics at the skip site, so an I/O error cannot turn the skip into a normal pass. The reporter rejects malformed, duplicate, unmatched, ambiguous, and failure-contradicting tokens.

### Reporter merge by test identity

`scripts/test-report.sh` and the CI nextest job create an empty oracle-skip file, set `PICTURA_ORACLE_SKIPS` for the nextest invocation, and pass the file to `report_tests.py`. CI uploads the token stream with its JUnit artifact. The reporter parses only the exact token prefix. It matches `<binary-id>::<test-name>` against JUnit `classname::name`, requires one match, and changes a passed entry to skipped. It never downgrades a failure. Malformed, duplicate, unmatched, ambiguous, and failure-contradicting tokens are errors. An empty file means no oracle skipped; it is not a completion record.

The Rust suite counts are recomputed from the changed entries. This keeps suite subtotals and TOTAL consistent and makes the skipped names visible in the normal listing.

### Framed persistent workers

`oracle::Command` keeps the familiar `arg`, `args`, `env`, `current_dir`, `output`, and `status` builder shape. Its first argument still names `python3` or `node`. Instead of spawning per call, it sends a length-prefixed request to a process-global worker behind a mutex. A worker exit invalidates the cached child so the next request can restart it.

The Python worker executes either a cached script module's `main()` or a `-c` snippet. It captures text and binary stdout plus stderr, preserves `SystemExit`, and restores argv and cwd around each request. Python imports remain in `sys.modules`, so `psd_tools` is loaded once per worker.

The Node worker executes `-e` source in a fresh VM context with the worker's `require`, a per-request argv, and captured console output. Node's module cache keeps `ag-psd` loaded. Both protocols use an exit code and explicit stdout/stderr lengths. Rust rejects truncated frames and caps a single reply at 512 MiB.

### Comparison stays in Rust

Workers only produce oracle answers. Existing `pictura-testkit::compare` calls, field parsing, and assertions remain at their current sites. Python and Node transport tests compare deliberately different replies, check exit status and both output streams, and issue a successful request after a non-zero one to prove the worker survives. The teeth test requires the comparison-specific `oracle mismatch` panic, so a worker failure cannot satisfy it.

## Risks / Trade-offs

- A stateful Python script could leak state between requests. The worker restores argv and cwd, and current oracle entry points are treated as per-request operations. A script that cannot run repeatedly must use an isolated worker instead.
- Serializing requests can reduce parallelism within one test binary. This is acceptable because interpreter startup and imports dominate these suites; separate test binaries still run concurrently.
- Thread names are supplied by libtest. The report method derives the test executable's package/target identity, and the reporter matches the full JUnit identity. An unmatched name fails the report rather than silently passing.
- Appending tokens from many test processes can interleave on unusual filesystems. Each token is emitted as one small append write; duplicate or malformed tokens fail the report.
- Worker startup failure is a transport error, not a tool skip. Tool availability is decided before requests by `require`, so a missing tool cannot be disguised as a crashed worker.

## Migration Plan

Add the seam and workers first, then move each oracle file from local probes to `require_oracle!` and from `std::process::Command` to `oracle::Command`. Update the shell capture and reporter, add the transport and parser checks, and audit the source for remaining targeted probes. Rollback is a single revert because no persisted format changes.

## Verification status

No build, test, formatter, lint, typecheck, OpenSpec validation, or repository gate ran during this task. Review is source-only, so worker runtime behavior and test outcomes remain unverified. The fetched tree contains 136 typed report sites after migration; the issue's count of 138 did not match this HEAD, and no targeted ad hoc tool skip remains.
