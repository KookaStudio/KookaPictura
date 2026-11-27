# Tasks

## 1. Range editing

- [x] 1.1 Write the planned Qt and Rust regression cases, including numeric split boundaries, unsupported modes, opaque data, cache invalidation, history and persistence.
- [x] 1.2 Implement the preserving range bridge, supported-mode gating, explicit opaque-block replacement and split-handle editor.
- [x] 1.3 Render split ranges and invalidate cached composites when typed ranges change.
- [x] 1.4 Add an original badge asset with provenance, preserving layout and malformed-raw badge behavior.
- [x] 1.5 Cover the inherited layer-row scenarios listed in the plan with the existing suite or the planned row-contract case.

## 2. Integration

- [x] 2.1 Review tracker coverage and archive the change.
- [x] 2.2 Build the optimized app in Arch x86-64 Docker and run the focused Qt and Rust regressions, formatting, repository guards and the packaged headless self-test.

Verified at `dde68d0e2314165a3195700e5a1e8e31daff8180`. Five Qt suites and eleven Rust tests passed. The packaged self-test passed 466 checks and skipped one check without a GPU adapter.
