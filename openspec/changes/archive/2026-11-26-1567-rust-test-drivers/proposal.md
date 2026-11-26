# Proposal

## Why

The codec and render crates compile and link a separate Rust integration binary for nearly every test file. At the issue's base, `pictura-codec` has 51 top-level test binaries and `pictura-render` has 28 auto-discovered integration binaries, including eight render binaries whose tests are all ignored. After rebasing onto `origin/master` that includes #1612, render's baseline is 30 and codec's dev-dependency on render is already gone. This cost reaches both nextest and `cargo clippy --workspace --all-targets` without adding default-suite coverage.

## What Changes

- Consolidate process-safe codec suites into `camera_raw`, `metadata`, `psd`, and `save` drivers, keeping the existing `oracle` driver.
- Keep `hostile_inputs`, `streamed_save`, and `merged_preview_buffers` as separate codec processes because they install process-wide allocators.
- Consolidate process-safe render suites into `document`, `oracle`, and `pdf` drivers.
- Keep render suites that mutate process-wide text gamma, backend selection, memory-host hooks, or child-process state in separate binaries. Keep the existing GPU drivers separate as well.
- Move the eight named all-ignored render suites to `crates/pictura-render/examples/`. Each example runs its profiling functions from `main`, while `cfg_attr(test, ...)` preserves the tests and their exact ignore reasons for `cargo test --example`.
- Update documented commands and moved-file references. Any `docs/` edit is limited to those updates and must ship later under `TASK-ALLOWS-DOCS`.
- Add no dependencies and no file-size allowlist entries.

## Capabilities

### New Capabilities

- `verification/rust-test-drivers`: target layout, test inventory preservation, process isolation, and runnable profiling examples for Rust integration suites.

### Modified Capabilities

None.

## Impact

Integration test binary names and module-qualified test names change. Runtime application behavior and public APIs do not change. Cargo manifests, dependencies, CI job definitions, and nextest's GPU filter remain unchanged because the retained GPU and cold-process binary names stay stable. The rebase base already ships #1565, so codec keeps its post-#1565 dev-dependencies, which contain no `pictura-render`; this change does not touch them.
