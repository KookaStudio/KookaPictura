# Proposal

## Why

Issue #279 asks for central failure-code allocation, declarative checks, and one
shared fixture lifecycle. The current tree does not meet that contract. It has
729 direct failure calls, 37 delegated failure propagators, and 503 `ST_BEGIN`
sites. Comparing each failure code with its declarative owner finds 186 failure
sites across 30 files that would name an earlier check. The issue's historical
822-site count is not reproducible from the authoritative tree, so this change
records the current population instead of preserving a stale total.

The existing uncommitted work fixed summary visibility and reporter truncation,
but it left codes as magic integers, kept `suite_` hardcoded, and reused a
paint-only fixture. Those gaps keep the original failure modes alive.

## What Changes

- Add `selftest_codes.h` as the append-only allocation registry. Every failure
  site uses a named constant. The registry also records retired codes, and code
  555 is allocated to the new profile-open startup failure.
- Add `selftest_checks.h` as the declarative check registry. Each check declares
  its name and suite. Existing checks stay in `core` to preserve token bytes;
  the reporter supports real suite transitions for checks that need them.
- Move every failure site under its owning `ST_BEGIN`. Preserve the ordered
  `ST_BEGIN`, `ST_PASS`, `ST_SKIP`, failure-code, and failure-message sequences.
- Add `SelfTestRun` as the single run-scope lifecycle. It owns the report guard,
  non-interactive setup, document cleanup, and tool restoration. Reuse its
  document RAII primitive from the paint fixture.
- Emit exactly one final `SUMMARY` on normal completion, delegated failure, and
  startup failure. Keep raw codes in `FAIL` tokens and normalize only the process
  status at the `main.cpp` boundary.
- Make the unified reporter reject missing, partial, multiple, trailing, or
  count-mismatched summaries. A summary-reported failure fails the run even when
  no parseable `FAIL` token exists.
- Extend `scripts/check-selftest-codes.py` to validate both registries, code and
  check ownership, startup failures, and skip truncation. Run it in both fast
  gate paths and the guards workflow.
- Add Qt Test coverage for suite transitions, startup failure, registry
  consistency, a real early return, and the shared run fixture.

Continue-on-failure, code renumbering, and full retirement into Qt Test remain
out of scope. The self-test budget stays at 503 and the `selftest.cpp` allowlist
ceiling drops to 6439.

## Capabilities

### Modified Capabilities

- `verification/test-reporting`: the structured output contract gains declarative
  registries, real suite selection, own-check failure naming, one shared run
  lifecycle, strict summary validation, and append-only code allocation.
- `verification/verification-harness`: the docs-only guard requirement includes
  the self-test registry and ownership guard.

## Impact

- `crates/pictura-app/cpp/selftest_codes.h`, `selftest_checks.h`: new registries.
- `crates/pictura-app/cpp/selftest_report.{h,cpp}`: dynamic suites, startup failure
  helper, idempotent summary, and settable sink.
- `crates/pictura-app/cpp/selftest_run.{h,cpp}`, `selftest_fixture.h`: shared
  lifecycle and document cleanup.
- `crates/pictura-app/cpp/main.cpp`: startup failures use the harness and process
  statuses normalize at the boundary.
- All `selftest*.cpp` failure sites use registry constants. Misowned sites move
  their `ST_BEGIN` above fixture setup.
- `scripts/report_tests.py`, `scripts/check-selftest-codes.py`: strict reporting
  and registry enforcement.
- `crates/pictura-app/cpp/tests/tst_selftest_report.cpp`: Qt Test coverage.
- `scripts/verify-fast.sh`, `.github/workflows/guards.yml`,
  `cmake/sources/selftest.cmake`, `crates/pictura-app/cpp/tests/CMakeLists.txt`:
  registration and gate wiring.
- `docs/dev/testing-conventions.md`: testing-contract update. This file needs a
  later `TASK-ALLOWS-DOCS` commit.

The green token stream keeps the same suite, check names, pass details, and
order. Failure tokens keep their raw codes and messages. The intended differences
are the corrected check name at 186 misowned sites and the added final summary on
failure paths.
