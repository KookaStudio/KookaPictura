# Spec Delta

## Purpose

Defines how a developer obtains a release-mode build whose Rust and C++ frames
carry source-line attribution under an external sampler, and what the
documentation must say about it — so a profile of the shipped configuration can
be taken without changing `Cargo.toml`, the CMake defaults, or the release
artifacts.

## ADDED Requirements

### Requirement: A release-mode build carries line tables on request

The repository SHALL support compiling the application for profiling as a local
CMake configure with `BUILD_TESTING` off and build type `RelWithDebInfo`, with
Rust debug information enabled by the `CARGO_PROFILE_RELEASE_DEBUG` environment
override present when cargo runs. That build SHALL invoke cargo with the release
profile, SHALL give the Rust library and the C++ shell line tables, and SHALL NOT
require a `Cargo.toml` profile, a new CMake option, or a changed default.

#### Scenario: The documented profiling build has line tables [pw_profiling_tree]

- **WHEN** a clean build tree is configured with `BUILD_TESTING` off and
  `CMAKE_BUILD_TYPE=RelWithDebInfo`, and built with `CARGO_PROFILE_RELEASE_DEBUG`
  set in the build environment
- **THEN** cargo is invoked with the release profile, and the linked `pictura`
  binary reports line tables for both Rust and C++ compilation units

#### Scenario: The shipped build is unchanged [pw_shipped_unchanged]

- **WHEN** a packaging configure (`Release`, `BUILD_TESTING` off) is built
  without the documented override
- **THEN** the resolved Cargo profile settings and the resulting artifact's debug
  information are the same as before this change, so no shipped artifact grows

### Requirement: The sampling workflow is documented and runnable

`DEVELOPING.md` SHALL document a sampling-profiling workflow that a reader can
run as written: the profiling configure and build, a recording command for
`perf` and one for `samply` (both on `PATH`, with an install hint), the
`perf_event_open` permission prerequisite — user-space self-profiling of one
process is allowed up to `perf_event_paranoid=2`, while `<=1` or `CAP_PERFMON` is
needed for kernel frames — and the rule that the build runs under the
repository's CPU budget while the recording does not, because the budget
restricts the process to an affinity slice. The existing note that recommends a
packaging configure for C++ profiling SHALL name the Rust debug-information
override and point at this workflow.

#### Scenario: A reader finds the recipe [pw_docs_recipe]

- **WHEN** a reader opens the Profiling section of `DEVELOPING.md`
- **THEN** they find the configure, build, and record commands in order, with the
  debug-information override named for the build

#### Scenario: A reader applies the budget split [pw_docs_budget_split]

- **WHEN** a reader follows the documented commands
- **THEN** the build command runs through `scripts/cpu-budget.py` and the
  recording command does not, and the documentation states the measurement reason

#### Scenario: A reader lacks sampling permission [pw_docs_prerequisites]

- **WHEN** a reader runs the recording command on a host whose
  `perf_event_paranoid` forbids unprivileged sampling
- **THEN** the documentation has already named both ways to grant the permission,
  and says which frames each covers

#### Scenario: The documented build resolves both frame kinds [pw_frames_resolve]

- **WHEN** the documented build is recorded while a document is loaded and a
  filter is applied
- **THEN** the report attributes hot frames to a workspace Rust crate and to the
  C++ shell with a source file and line, and the same report from a build without
  the override has the symbols but not the source lines
