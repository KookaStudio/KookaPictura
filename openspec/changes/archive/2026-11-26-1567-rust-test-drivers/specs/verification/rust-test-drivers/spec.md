# Spec Delta

## Purpose

Defines how Rust integration suites are grouped into build targets without losing tests, mixing process-wide state, or making manual profiling harder to run.

## ADDED Requirements

### Requirement: Process-safe integration suites share topic drivers

The codec and render crates SHALL group integration suites that do not mutate conflicting process-wide state into module-per-topic test drivers. Cargo auto-discovery SHALL produce one test binary per driver rather than one binary per moved source file.

#### Scenario: Codec suites use topic drivers [rtd_codec_drivers]

- **WHEN** the codec integration targets are inspected
- **THEN** process-safe suites are declared by the `camera_raw`, `metadata`, `psd`, and `save` drivers, and the existing `oracle` driver remains available

#### Scenario: Render suites use topic drivers [rtd_render_drivers]

- **WHEN** the render integration targets are inspected
- **THEN** process-safe document, oracle, and PDF suites are declared by the `document`, `oracle`, and `pdf` drivers

### Requirement: Process-global suites stay isolated

A suite that installs a global allocator or changes process-wide text gamma, backend selection, memory-host hooks, or child-process test state SHALL remain a separate test binary. Consolidation SHALL NOT rely on test ordering or a shared lock to make those suites safe.

#### Scenario: Allocator and heap guards are separate [rtd_allocator_isolation]

- **WHEN** the codec test targets are inspected
- **THEN** `hostile_inputs`, `streamed_save`, and `merged_preview_buffers` each remain an independent binary

#### Scenario: Render global state is separate [rtd_render_isolation]

- **WHEN** the render test targets are inspected
- **THEN** `text_gamma_off`, `backend_pin`, `backend_cpu`, `canvas_mask_domain`, `cold_engine_profile`, `gpu_parity`, and `gpu_filter_parity` each remain an independent binary

### Requirement: Consolidation preserves the test inventory

Moving a suite into a topic driver SHALL preserve every test function, every non-ignored test, and every exact ignore reason string. Module qualification MAY change test names, but no test may disappear from the source inventory.

#### Scenario: Non-ignored tests are retained [rtd_active_inventory]

- **WHEN** the pre-change and post-change source inventories are compared
- **THEN** every non-ignored test has exactly one destination under a topic driver or an isolated binary

#### Scenario: Ignore reasons are retained [rtd_ignore_reasons]

- **WHEN** an ignored test is moved into a driver or example
- **THEN** its ignore reason string is byte-for-byte unchanged

### Requirement: All-ignored profiling suites are runnable examples

The eight named all-ignored render profiling suites SHALL live under `crates/pictura-render/examples/`. Each SHALL provide a `main` entry point for `cargo run --example <name>` and SHALL retain its test functions and ignore reasons for `cargo test --example <name>`.

#### Scenario: Every profiling suite runs as an example [rtd_profiling_examples]

- **WHEN** a developer runs `cargo run --example <name>` for one of the eight suites with its documented environment and features
- **THEN** that suite's profiling functions execute and report through the example process

#### Scenario: Example tests remain inspectable [rtd_example_test_inventory]

- **WHEN** a developer builds one of the eight examples as a test target
- **THEN** the original test functions are present and still carry their exact ignore reasons

### Requirement: Run strings and references follow the targets

Documented commands and references to moved suites SHALL name the new driver, module, or example. Commands that quote a moved integration target SHALL remain runnable without guessing a replacement binary name.

#### Scenario: Archived profiling commands use examples [rtd_archived_run_strings]

- **WHEN** archived OpenSpec documents quote a command for one of the eight profiling suites
- **THEN** the command uses `cargo run --example <name>` and the referenced source path is under `examples/`

#### Scenario: Current driver commands resolve [rtd_current_run_strings]

- **WHEN** current guidance quotes a command for a consolidated codec or render suite
- **THEN** the command names the topic driver and, when selecting one former suite, its module-qualified test filter

### Requirement: Target reduction adds no dependency or size exception

The consolidation SHALL use Cargo's existing test and example auto-discovery. It SHALL add no dependency, no test-target list, and no file-size allowlist entry, and every changed source file SHALL remain within the existing size caps. The only permitted manifest addition is a `required-features` entry for an example whose source is gated on the crate's optional GPU feature, so that a build without that feature has no entry point to compile.

#### Scenario: Target layout is convention-only [rtd_no_manifest_entries]

- **WHEN** the crate manifests and file-size allowlist are inspected
- **THEN** no test target is declared explicitly, only the two GPU-gated examples carry a `required-features` entry, and the allowlist has no new entry

#### Scenario: Build and test target count falls [rtd_target_reduction]

- **WHEN** Cargo discovers the post-change targets
- **THEN** the codec and render integration binary count is lower than the 51 and 28 baseline binaries, and the eight profiling suites are examples rather than integration test binaries
