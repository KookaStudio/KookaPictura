# CI Build Tree Cache

## Purpose

Defines how CI reuses the Qt headless build tree without accepting stale outputs, hiding source edits, or losing the existing test and build policy guarantees.

## Requirements

### Requirement: The Qt headless build tree is cacheable as one incremental unit

The `qt-headless` job SHALL restore the complete CMake build tree as one cache unit. The unit SHALL include Corrosion's Cargo build and target output, generated cxx-qt headers, moc and autogen output, FetchContent sources, Ninja state, and the application and Qt Test executables. Cache identity SHALL cover the runner image and architecture, Rust and C++ toolchains, Qt installation, CMake, Ninja, Cargo lockfile and manifests, build-graph inputs, Cargo profile, and configure flags.

#### Scenario: a compatible warm tree is restored [cbt_cache_contents]

- **WHEN** a run has the same cache identity as a saved default-branch tree
- **THEN** CI restores the CMake, Corrosion, generated, moc, autogen, FetchContent, Ninja, application, and Qt Test outputs together

#### Scenario: ordinary source content does not invalidate the tree [cbt_source_same_key]

- **WHEN** a pull request changes Rust, C++, bridge, or Qt Test source content without changing the build graph or cache identity
- **THEN** the run uses the same restore prefix and Ninja decides which outputs to rebuild

### Requirement: Only the default branch saves and one writer owns the tree

CI SHALL submit a build-tree cache save only for runs on `master`. Pull requests and manual non-default runs SHALL restore without saving. A quota refusal SHALL be reported and SHALL NOT fail an otherwise successful build. The `qt-headless` job SHALL have one build writer, with configure, build, test, manifest creation, and cache save running in order. Cache keys SHALL be unique per successful default-branch run while retaining a stable restore prefix.

#### Scenario: a pull request restores but does not save [cbt_pr_no_save]

- **WHEN** CI runs `qt-headless` for a pull request
- **THEN** it can restore a compatible default-branch entry and does not save a new entry

#### Scenario: a successful default-branch run submits one save [cbt_master_save]

- **WHEN** all `qt-headless` build and test steps succeed on `master`
- **THEN** CI submits one uniquely keyed save under the stable restore prefix, records whether quota accepted it, and keeps the build result independent of a quota refusal

#### Scenario: no concurrent writer mutates the tree [cbt_single_writer]

- **WHEN** a `qt-headless` run configures, builds, tests, and saves its tree
- **THEN** no second build or cache writer in that job mutates the same tree

### Requirement: A restored tree must prove it is complete and compatible

Before configure, CI SHALL validate the restored manifest and SHALL reject the tree if the identity differs, the source directory differs, a required build file is missing, a generated output is missing or differs from its saved digest, or a registered Qt Test executable is missing. A rejected tree SHALL be removed before a cold configure. CI SHALL normalize unchanged tracked source and directory timestamps to an epoch older than restored outputs, while changed and new sources keep newer timestamps.

#### Scenario: a missing generated output rejects the restore [cbt_stale_generated]

- **WHEN** a restored tree is missing a generated cxx-qt header, moc file, autogen output, or required Corrosion output
- **THEN** CI reports stale rejection, removes the tree, and configures from cold inputs

#### Scenario: a changed generated digest rejects the restore [cbt_stale_digest]

- **WHEN** a generated output exists but its content differs from the manifest digest
- **THEN** CI rejects the restore instead of building against that output

#### Scenario: a different toolchain rejects the restore [cbt_stale_identity]

- **WHEN** the restored manifest fingerprint does not match the current toolchain, Qt, profile, or configure identity
- **THEN** CI rejects the restore and performs a cold configure

#### Scenario: unchanged sources do not force a rebuild [cbt_mtime_normalization]

- **WHEN** checkout gives unchanged tracked files fresh timestamps after a compatible restore
- **THEN** CI makes those files older than restored outputs and leaves changed or new files newer

### Requirement: Warm builds rebuild only affected outputs

After a valid restore, the existing Ninja build SHALL decide the work. An unchanged warm tree SHALL compile and link no outputs. A source edit SHALL rebuild its affected outputs without invalidating the cache identity. A bridge module edit SHALL retain the existing same-build C++ regeneration guarantee. A forced-cold run SHALL configure, build every target, and run the normal checks without a restored tree.

#### Scenario: an unchanged warm build does no compile work [cbt_warm_no_work]

- **WHEN** a valid restored tree matches the current source content
- **THEN** the build records zero Ninja edges and does not recompile or relink outputs

#### Scenario: a changed target rebuilds incrementally [cbt_changed_target]

- **WHEN** a Rust, shell-header, bridge-file, or Qt Test source changes after a valid restore
- **THEN** the build records the affected Ninja edges and does not rebuild unrelated outputs

#### Scenario: a forced-cold run remains complete [cbt_forced_cold]

- **WHEN** a manual run requests a forced-cold cache bypass
- **THEN** CI configures from empty inputs, builds every target, and runs the normal self-tests and CTest suites

### Requirement: Cache identity changes only for incompatible build inputs

A change to the toolchain, Qt, compiler, CMake, Ninja, lockfile, Cargo manifests, build-graph source lists, bridge-source list, build script, cache schema, profile, or configure flags SHALL change the restore prefix. An ordinary source content edit SHALL not change it.

#### Scenario: a build-input change invalidates prior entries [cbt_key_invalidation]

- **WHEN** one of the incompatible build inputs changes
- **THEN** prior entries do not match the new restore prefix

### Requirement: Existing build and test policy is preserved

The cached path SHALL keep precompiled headers disabled and SHALL keep the application's GPU feature enabled. Every Qt Test executable registered by CMake SHALL build, and every CTest entry SHALL retain its own JUnit output path. A missing per-entry report SHALL fail reporting rather than being silently omitted.

#### Scenario: no PCH and GPU stay enabled [cbt_no_pch_gpu]

- **WHEN** CI configures the cached build
- **THEN** precompiled headers remain disabled and the application still enables the GPU stack

#### Scenario: every Qt Test entry reports [cbt_qt_junit]

- **WHEN** CTest finishes
- **THEN** every registered entry has emitted its own JUnit report and a missing report fails the job

### Requirement: CI records honest cache and build measurements

Every `qt-headless` run SHALL publish a JSON measurement artifact and a job-summary table. The record SHALL include cache state and matched key, change classes, restored uncompressed bytes, saved uncompressed bytes, saved archive bytes when available, restore, validation, and upload seconds, sccache hits and hit rate, Ninja edge count, and Build duration. Forced-cold and stale-rejection runs SHALL be labeled distinctly. The record SHALL not claim a ten-minute job target.

#### Scenario: measurements are emitted for warm and cold runs [cbt_measurements]

- **WHEN** a warm, forced-cold, or stale-rejection run finishes
- **THEN** the summary and JSON artifact name its cache state and record the available size, time, hit-rate, edge-count, and Build-duration measurements
