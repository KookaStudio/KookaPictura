## ADDED Requirements

### Requirement: Bridge files hold declarations only

Each Rust source that holds a `#[cxx_qt::bridge]` module SHALL hold no
implementation outside that module. Besides the module and the attributes and
doc comments above it, such a file MAY contain only module declarations without
a body (including test modules declared with a `#[cfg(test)]` attribute and
kept in their own files), `use` and `pub use` items, and trait implementations
with an empty body that the bridge requires. It SHALL NOT contain a function
with a body outside the bridge module, nor a module with an inline body. The
bodies of the functions a bridge declares, their helpers and their tests SHALL
live in ordinary modules that the cxx-qt build script does not track, and the
list of bridge files the build script hands to cxx-qt SHALL stay the same.
Moving a body out SHALL be a pure move with no behaviour change. An edit to
such a body SHALL NOT rerun the cxx-qt build script.

#### Scenario: an implementation edit does not rerun the build script [bp_bridge_impl_no_rerun]

- **WHEN** the body of a function that a bridge declares is edited and the project is rebuilt
- **THEN** the cxx-qt build script does not rerun and no generated C++ is recompiled

#### Scenario: bridge files carry no implementation [bp_bridge_decl_only]

- **WHEN** the bridge Rust sources are inspected after the change
- **THEN** none contains a function body or an inline module body outside its bridge module

### Requirement: A clean C++ rebuild regenerates the cxx-qt outputs

After the build tree's outputs are removed with the generator's clean command,
the next build SHALL regenerate every cxx-qt output that the C++ link consumes,
and SHALL succeed without a manual step such as touching the build script or
reconfiguring.

#### Scenario: clean then build succeeds [bp_clean_rebuild]

- **WHEN** a built tree is cleaned with the generator's clean command and built again
- **THEN** the build succeeds and links the shell and every Qt Test executable

### Requirement: CI reuses compiled Rust targets between runs

The CI jobs that compile the Rust workspace for linting and testing SHALL
restore compiled targets from a cache keyed on the toolchain and the lockfile,
so an unchanged dependency graph is not recompiled on every run. Each of those
jobs SHALL use its own cache key. The caches SHALL be saved only by runs on the
default branch, so pull requests restore them without adding entries. Because
only a default-branch run can save the first cache, the jobs' wall time and
cache size after that run SHALL be recorded in a follow-up issue rather than
in the change that adds the caching.

#### Scenario: a pull request restores but does not save [bp_ci_target_cache]

- **WHEN** CI runs for a pull request
- **THEN** the lint and test jobs each restore compiled targets under their own key when a default-branch cache exists, and neither job saves a cache

### Requirement: Engine-crate edit cost is measured

The cost of a one-line edit to an engine crate — at least `pictura-core` and
`pictura-render` — SHALL be measured under the development and release Cargo
profiles, broken down by crate, and recorded in the change's design notes. A
per-package profile override SHALL be added only when the measurement shows the
dependents' rebuild dominates, and only when the Qt Test layer's wall time does
not grow by more than the edit time it saves.

#### Scenario: the engine edit cost is recorded [bp_engine_edit_measured]

- **WHEN** the design notes are inspected after the change
- **THEN** they list the per-crate rebuild time for an edit to `pictura-core` and to `pictura-render` under both profiles, and the decision on profile overrides

## MODIFIED Requirements

### Requirement: Development compile settings are cheaper for known-heavy units

In an optimized development configuration (`BUILD_TESTING` on with Release or
RelWithDebInfo), `filter_commands.cpp` SHALL compile at `-O1` with GCC, Clang,
or AppleClang, and each Qt Test translation unit SHALL compile at `-g1` with
those compilers. Release packaging and a Debug configure SHALL keep the normal
configuration's optimization and debug settings for the same sources.

#### Scenario: heavy command sources compile at O1 [bp_cheap_tu]

- **WHEN** an optimized development build compiles `filter_commands.cpp` with GCC, Clang, or AppleClang
- **THEN** its compile command carries `-O1`

#### Scenario: Qt Test units compile with reduced debug info [bp_test_debug]

- **WHEN** a development build compiles a Qt Test translation unit with GCC, Clang, or AppleClang
- **THEN** its compile command carries `-g1` rather than full `-g`

### Requirement: The heaviest C++ translation units are measured and cheapened

The build system SHALL measure the cost of the heaviest C++ translation units
— the shell's menu construction and the Qt Test translation units — and of the
C++ compile pipeline, and SHALL cheapen what dominates by splitting a heavy
source along a concern seam, by reducing debug information, by reducing the
per-suite moc cost, by a precompiled header, or by different development
optimisation flags, whichever the measurement shows pays. A cheapening SHALL be
kept only when, on the same machine, it does not slow a cold C++ build (no
compiler-cache hits) and does not slow a warm rebuild served from the compiler
cache, and it speeds up at least one of them. A cheapening that trades one for
the other SHALL be recorded in the change's design notes with both numbers and
the reason for keeping it. A measured change that pays in neither SHALL be
recorded as skipped rather than shipped, and every record SHALL name the
numbers it was decided on. A kept cheapening that changes compile settings
SHALL apply only to the optimized development configuration (`BUILD_TESTING`
on with Release or RelWithDebInfo); a Debug configure and release packaging
SHALL keep their settings, and the per-source settings that other requirements
pin (such as `-O1` for the heavy command sources and `-g1` for the Qt Test
units) SHALL still take effect. The sources SHALL compile without a
precompiled header, so a configure with precompiled headers disabled builds.

#### Scenario: a heavy source is split or the split is declined [bp_heavy_tu_cheapened]

- **WHEN** the menu-construction source is measured as the heaviest translation unit
- **THEN** it is split along a concern seam as a pure move, or the measurement is recorded as not worth the split

#### Scenario: the test-unit cost is reduced or declined [bp_test_tu_cost]

- **WHEN** the Qt Test translation units are measured as the dominant C++ compile cost
- **THEN** the per-suite moc cost is reduced, or the measurement is recorded as not worth changing

#### Scenario: a compile change is measured both ways [bp_compile_change_measured]

- **WHEN** the design notes are inspected after the change
- **THEN** each compile-pipeline change lists its cold-build and warm-rebuild cost before and after, and its kept or skipped verdict

#### Scenario: packaging and Debug keep their settings [bp_compile_change_release]

- **WHEN** the project is configured with `BUILD_TESTING` off, or as a Debug build
- **THEN** no precompiled header and no development-only optimisation or debug flag appears in the compile commands

#### Scenario: pinned per-source settings still win [bp_compile_change_per_source]

- **WHEN** an optimized development build compiles a heavy command source or a Qt Test unit after a kept flag change
- **THEN** the heavy command source is still compiled at `-O1` and the Qt Test unit at `-g1`

#### Scenario: the sources build without a precompiled header [bp_pch_off_builds]

- **WHEN** the project is configured with precompiled headers disabled and built
- **THEN** every C++ source compiles and the shell and every Qt Test executable link
