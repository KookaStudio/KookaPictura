# Proposal

## Why

Missing oracle tools currently look like passing tests. The hand-written early returns hide the difference between a real comparison and no comparison, and per-case Python startup repeats expensive imports such as `psd-tools`.

## What Changes

- Add `pictura-testkit::oracle::require(tool)` with typed skips for ImageMagick, psd-tools, and ag-psd, including the existing RSVG, scikit-image, and shallow-knockout capability checks.
- Emit `pictura oracle: SKIP <tool> <test> [reason]` tokens with binary-qualified test identity and fold them into the Rust layer of the unified report as named skipped tests.
- Run Python and Node oracle requests through one cached worker per test binary, with framed requests and replies. Keep every pixel and metadata comparison in Rust through `pictura-testkit::compare` or the existing field assertions.
- Add checks for token parsing, request/reply correlation, and a present oracle returning a wrong answer.
- Replace the magick, psd-tools, and ag-psd availability helpers and their early returns without removing assertions. Other optional tools keep their existing availability policy, while their Python script calls can still use the batch transport.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/test-reporting`: require missing oracle tools to appear as named skipped tests, reject unmatched skip tokens, and require batched oracle transport to preserve request identity and comparison failures.

## Impact

`pictura-testkit` gains the oracle seam and batch transport. Oracle tests across the codec, render, color, adjust, filters, ops, and select crates use it. `scripts/report_tests.py`, `scripts/test-report.sh`, and the CI nextest job consume the skip stream. Python and Node worker scripts are added under `scripts/`. No external dependency or fixture format changes; select gains the existing internal testkit dev-dependency.
