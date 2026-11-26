# Proposal

## Why

`scripts/test-report.sh` and the Linux headless CI job run the self-test twice. The bare run executes 467 checks, the document run executes 502, and the reporter keeps 503 unique checks after deduplication. One combined run produces that same unique set without repeating the 466 shared checks or paying for the second process.

## What Changes

- Run one self-test process with `two_layers.psd` in `scripts/test-report.sh` and the Linux headless CI job. The process starts on the scratch document for `fresh_white`, opens the PSD in-process, and then runs the document-only block.
- Use one mutable `documentMode` boolean for the two mode-specific blocks. `fresh_white` runs before the in-process open, and the 36 document checks run after a successful open.
- Make a single self-test stream's SUMMARY authoritative in the reporter's merge path. Keep the existing multi-stream merge behavior and FAIL-wins rule for callers that still provide multiple captures.
- Preserve the token grammar, check names, failure codes, process status mapping, and the 503-check shrink-only budget.
- Leave nextest before the self-test and CTest layers. The single self-test still overlaps CTest. No nextest overlap will ship without the measurement described in the design and a later sequential measurement phase.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/test-reporting`: the unified report runs one combined self-test capture and reports its SUMMARY without losing the existing 503-check unique set.
- `verification/verification-harness`: Linux headless CI runs the combined self-test once instead of running bare and document modes separately.

## Impact

- C++ startup and self-test mode selection in `crates/pictura-app/cpp/main.cpp`, `selftest.h`, and `selftest.cpp`.
- Local capture orchestration in `scripts/test-report.sh` and single-stream handling in `scripts/report_tests.py`.
- Linux headless CI invocation in `.github/workflows/ci.yml`.
- OpenSpec planning and test-plan artifacts for issue #1570.
- No dependency, ST macro, exit-code, budget, check-body, or assertion removal changes.
