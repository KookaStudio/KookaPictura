# Design

## Context

See `proposal.md` for the duplicate work being removed. `main.cpp` currently
opens a supplied PSD before `runSelfTest`, and `codecLoaded` selects either the
`fresh_white` block or the 36-check document block. `SelfTestRun` owns one report
guard and one final SUMMARY. `scripts/report_tests.py` validates each capture and
merges unique `(suite, name)` keys, but its single-capture SUMMARY override
currently lives in `build_layers`.

The legacy runs share 466 checks. Bare mode adds `fresh_white`, for 467 checks.
Document mode adds 36 checks, for 502 checks. Their unique union is 503 checks.
The shrink-only budget is also 503.

## Goals / Non-Goals

**Goals:**

- Keep one process, one report guard, one SUMMARY, and the same 503 unique checks.
- Run `fresh_white` on the scratch document, then open `two_layers.psd`
  in-process and run the 36 document checks.
- Execute the 466 shared checks once.
- Preserve direct bare-mode behavior and all existing failure identities.
- Keep the reporter's legacy multi-capture API and FAIL-wins merge behavior.

**Non-Goals:**

- Running two full self-test passes in one process.
- Moving, deleting, or weakening any check or assertion.
- Changing ST macros, exit codes, check registries, or the 503-check budget.
- Changing nextest, self-test, or CTest overlap in this phase.
- Updating `docs/`, `DEVELOPING.md`, or Serena memories.

## Decisions

### Use one combined capture

`scripts/test-report.sh` and the Linux headless CI job will pass
`two_layers.psd` to one `--headless --self-test` process. For a self-test,
`main.cpp` will create the scratch document first and defer opening the PSD to
`runSelfTest`. Direct `./build/pictura --headless --self-test` remains the
bare-only entry point.

Inside `runSelfTest`, one mutable `documentMode` flag starts false. The existing
`fresh_white` block runs against the scratch image. The function then opens
`psdPath` through the existing frame path, closes the scratch document after a
successful open so the document block starts from the same one-document state as
the old document run, updates the active view and image, and sets `documentMode`
from the open result. The existing 36-check document block runs only after a
successful open. The shared checks that follow run once.

This produces 503 unique keys, 466 shared plus `fresh_white` plus 36 document
checks. The old two-process report merges the same 503 keys from 969 check
executions. The new run executes 503 checks.

Alternatives considered:

- Keep two processes. This repeats the 466 shared checks.
- Run only the old document mode. This drops `fresh_white` and emits 502 keys.
- Run two full passes in one process. This still repeats the shared checks and
  needs a larger fixture lifecycle change.

### Keep the mode flag and image handoff small

`main.cpp` will call the loaded-document boolean `documentMode` and skip its
pre-run open when `selfTest` is true. `selftest.h` and `selftest.cpp` will use the
same mutable flag. `runSelfTest` will take its `QImage` by value so it can replace
the scratch image after the in-process open without changing any check body.
A failed open leaves the flag false and the scratch document active, as the old
document invocation does when its open fails. The new handoff stays compact
enough to keep `selftest.cpp` at its existing 6439-line ceiling. No ST macro,
failure code, or return path changes.

### Move the single-capture SUMMARY rule into the merge function

`merge_selftest_streams` will retain the parsed SUMMARY when it receives one
complete stream and use it as the returned total. `build_layers` will stop
applying the same override. Multi-stream calls will still recompute totals from
unique keys and preserve FAIL-wins behavior.

This keeps one implementation of the rule and gives the reporter self-check a
direct way to prove the one-stream path. Removing the multi-stream API would
break existing callers and tests without helping this change.

The self-check will compare a synthetic combined stream containing two shared
checks, `fresh_white`, and 36 document checks against the legacy merge of a bare
stream and a document stream. It will assert identical unique entries and a
39-check total for the fixture. The production arithmetic remains 466 + 1 + 36
= 503.

### Keep the current overlap boundary

Nextest remains before the self-test and CTest. The one self-test process still
runs concurrently with CTest. This phase does not add nextest to that group.

### Remove the duplicate Linux CI invocation

The Linux headless job will keep only the combined self-test step. Other
platform packaging smoke tests already run one bare invocation and do not
contribute the duplicate report capture.

## Risks / Trade-offs

- A missing or unreadable fixture can make the in-process open leave the run in
  bare mode and reduce the capture to 467 checks. This fallback already exists.
  The reporter self-check proves the set arithmetic, while later run
  verification must prove the committed fixture opens and the real SUMMARY
  reports 503 checks.
- Direct `--self-test <file.psd>` now emits the combined 503-check stream instead
  of the old 502-check document stream. This is the intended unique-set
  preservation.
- One standard process no longer exercises two concurrent self-test endpoint
  names. The control endpoint remains process-unique, but concurrent invocation
  coverage needs a separate check if the project wants to keep it in CI.
- `docs/dev/testing-conventions.md`, `DEVELOPING.md`, and Serena memories still
  describe two invocations. They are outside this task's allowed scope and need
  a docs-authorized follow-up.
- No build or runtime evidence exists in this phase. Compilation, token output,
  failure identity, and the real 503-check SUMMARY remain unverified.

## Measurement plan for a later sequential phase

Do not run these measurements while implementing this change. After the code is
review-clean, use one built tree and the same 80% CPU affinity slice for every
run. Fix `CTEST_PARALLEL_LEVEL`, record the allowed CPU count, and keep other
verification work off the machine.

1. Warm each layer once, then run nextest alone, the combined self-test alone,
   and CTest alone five times each. Capture wall time, user CPU, system CPU, and
   peak RSS with `/usr/bin/time -v`.
2. Measure the current schedule, nextest then concurrent self-test plus CTest,
   five times. Measure the candidate schedule with nextest overlapping both,
   five times. Interleave the schedules to reduce thermal and cache drift.
3. For the CPU-halving claim, compare the pre-change bare-plus-document runs
   with the new combined run from equivalent builds under the same budget.
   Record process CPU separately from wall time.
4. Keep separate logs and JUnit outputs for every layer. A schedule with a
   failure, missing report, or changed check set is invalid evidence.
5. Compare medians and spread. Commit nextest overlap only if the candidate
   improves median wall time beyond the observed run-to-run noise, stays inside
   the CPU budget, and does not add flakes. Record the result either way.

No overlap change belongs in this implementation.

## Migration plan

Bare-mode callers need no migration. A direct self-test invocation with a PSD
fixture now includes `fresh_white` before the document block and reports the combined
503-check stream. The standard report and Linux CI keep the same 503 unique
checks with one process and no new dependency or data format. Rollback restores
the pre-run open and second invocation while leaving the reporter's single-stream
support intact.
