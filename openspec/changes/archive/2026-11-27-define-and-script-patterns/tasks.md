# Tasks

## 1. Define and consume patterns

- [x] 1.1 Write Qt cases `tst_pattern_authoring::defineAndReuse`, `invalidAndCancel`, `consumerReuse`, `defineFailure`, `fillPreset`, `fillReplay`, `emptyList`, `shapePatternAtomicity`, `captureMemoryAndStaleEdit` and `builtinSnapshot` plus Rust pattern alpha and coordinate tests as specified in the test plan.
- [x] 1.2 Implement hard rectangular selection capture and persistent named preset creation; cover acceptance, cancellation, failure and no document history.
- [x] 1.3 Wire current pattern entries and snapshots into Edit Fill and Paint Bucket, and use content-derived IDs for existing layer and shape embedding. Record portable tile snapshots and script choices; preserve imports and legacy index-only recorded actions. Add the planned Qt case sources for consumers and replay.

## 2. Scripted fills

- [x] 2.1 Write `pictura_app::scripted_pattern_geometry_and_determinism`, `pictura_app::scripted_pattern_bounds` and Qt cases `tst_pattern_authoring::scriptEdit`, `scriptCancel` and `scriptBounds`.
- [x] 2.2 Implement five bounded deterministic approximate layouts through the existing fill transaction, with UI selection, portable action replay and refusal without partial edits on exceeded bounds; cover all planned cases.

## 3. Review and delivery

- [x] 3.1 Review implementation, update ROADMAP.md and PARITY.md for actual shipped scope, archive the change and retain the test plan.
- [x] 3.2 Build the Arch x86-64 release in local Docker. The packaged headless self-test, focused Paint Bucket, command catalogue and scripted geometry Rust tests, Pattern Authoring, Fill Tools and Tool Options Persistence Qt suites, Rust formatting, affected-package Clippy and strict OpenSpec validation passed.
