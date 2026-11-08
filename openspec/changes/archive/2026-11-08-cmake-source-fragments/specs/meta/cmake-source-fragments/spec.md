# Spec Delta

## Purpose

Keeps the repository's explicit C++ source lists maintainable by splitting them
into per-area fragments, so parallel branches adding sources edit different
files instead of colliding in one append-only block.

## ADDED Requirements

### Requirement: C++ source lists live in per-area fragments

The shell and executable source lists SHALL be declared in
`cmake/sources/*.cmake` fragments grouped along seams already present in the
tree, and the root `CMakeLists.txt` SHALL include those fragments instead of
hand-listing source paths. The root file SHALL keep the target definitions
(`add_library`, `add_executable`), compile and link settings, install rules,
and the release-please version marker. The fragments SHALL append to the
existing `PICTURA_SHELL_SOURCES` and `PICTURA_SELF_TEST_SOURCES` variables and
SHALL be included before the target calls that consume them, so
`pictura_depend_on_bridge_stamp` still receives the complete lists. The change
SHALL NOT add a target, move a source under `cpp/`, or introduce a glob.

#### Scenario: Fragments are included before the targets [csf_include_order]

- **WHEN** the root `CMakeLists.txt` is inspected after the change
- **THEN** each `cmake/sources/*.cmake` include appears before the target call that consumes its variable (`add_library(pictura_shell ...)` or `add_executable(pictura ...)`)

#### Scenario: The root file keeps the target contract [csf_root_contract]

- **WHEN** the root `CMakeLists.txt` is inspected after the change
- **THEN** it still defines both targets, keeps the `filter_commands.cpp` per-source property, the `x-release-please-version` marker, and contains no `file(GLOB` over C++ sources and no per-source path entries in the two source lists

#### Scenario: The bridge stamp receives every source [csf_bridge_stamp]

- **WHEN** the configure runs after the change
- **THEN** `pictura_depend_on_bridge_stamp` is called with the complete shell and self-test source lists

### Requirement: Every C++ translation unit appears exactly once

Each `.cpp` file under `crates/pictura-app/cpp/`, excluding
`crates/pictura-app/cpp/tests/` and the Rust-built
`crates/pictura-app/cpp/panels/actions_tool_recording_bridge.cpp`, SHALL appear
in exactly one fragment, and each fragment SHALL append to the variable for the
target that owns it: shell sources to `PICTURA_SHELL_SOURCES`, `main.cpp` and
`selftest*` sources to `PICTURA_SELF_TEST_SOURCES`. Headers MAY be listed but
are not required.

#### Scenario: The fragments cover the tree [csf_complete]

- **WHEN** the `.cpp` paths in `cmake/sources/*.cmake` are compared with the `.cpp` files under `crates/pictura-app/cpp/` excluding `cpp/tests/` and `cpp/panels/actions_tool_recording_bridge.cpp`
- **THEN** every translation unit is listed and none is missing

#### Scenario: No path is listed twice [csf_no_duplicates]

- **WHEN** the paths from all fragments are concatenated
- **THEN** no path appears more than once, the shell list holds 556 unique paths after the two pre-existing duplicate lines are removed, and the self-test list holds 76

#### Scenario: Each source belongs to its target [csf_target_owner]

- **WHEN** the self-test fragment is inspected
- **THEN** it contains only `main.cpp` and `selftest*` sources, and no such source appears in a shell fragment

### Requirement: The change is a pure move

The target graph, the set of compiled translation units, and the compile and
link settings SHALL be identical before and after the change. Configuring and
building the tree SHALL succeed, and the headless self-test SHALL pass
unchanged.

#### Scenario: The configured targets are identical [csf_targets_identical]

- **WHEN** the project is configured on the change and on the base revision
- **THEN** both configurations expose the same target names and dependencies

#### Scenario: The compiled source set is identical [csf_compile_set_identical]

- **WHEN** the sorted `"file"` values of `compile_commands.json` are compared between the change and the base revision
- **THEN** the two sets are identical

#### Scenario: The tree builds and self-tests [csf_build_selftest]

- **WHEN** `cmake -S . -B build -G Ninja && cmake --build build --parallel` runs and then `./build/pictura --headless --self-test`
- **THEN** the build succeeds and the self-test reports no failures

### Requirement: Adding a source edits one fragment

Adding a C++ source SHALL require editing exactly one area fragment and SHALL
NOT require editing the root `CMakeLists.txt`. The contributor documentation
SHALL name the fragment convention.

#### Scenario: A new source touches one fragment [csf_single_fragment]

- **WHEN** a contributor adds a source to an existing area and updates the build
- **THEN** only that area's fragment changes and the root `CMakeLists.txt` does not

#### Scenario: Parallel branches merge cleanly [csf_parallel_merge]

- **WHEN** two branches each add a source to a different fragment and are merged
- **THEN** the merge has no conflict in `CMakeLists.txt` or the fragments

#### Scenario: The contributor docs name the convention [csf_docs]

- **WHEN** `AGENTS.md` and `DEVELOPING.md` are inspected after the change
- **THEN** both direct a new shell or self-test source to the matching `cmake/sources/*.cmake` fragment rather than the root file, and a new Qt Test source to `crates/pictura-app/cpp/tests/CMakeLists.txt`

### Requirement: The source-list check is mechanical

A repository script SHALL verify the fragment invariants without a manual
inspection: no duplicate path, every listed path exists on disk, every
non-test non-Rust-built `.cpp` under `crates/pictura-app/cpp/` is listed, and no
`main.cpp` or `selftest*` source appears in a shell fragment. The script SHALL
support a self-test and SHALL run in `scripts/verify-fast.sh` and the guards
workflow.

#### Scenario: The fragment invariants are checked mechanically [csf_guard]

- **WHEN** `scripts/check-cmake-source-fragments.py` runs, directly or through `scripts/verify-fast.sh` or the guards workflow
- **THEN** it fails on a duplicate path, a listed path missing from disk, an unlisted non-test non-Rust-built `.cpp`, or a self-test source in a shell fragment, and passes on the unchanged tree
