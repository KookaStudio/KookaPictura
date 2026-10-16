# Spec Delta

## Purpose

Keeps local C++ compilation warm by defaulting the CMake compiler launcher to an
already-installed `sccache`, so plain configures and clean rebuilds reuse cached
objects instead of recompiling every translation unit.

## ADDED Requirements

### Requirement: Local CMake builds default the C++ compiler launcher to sccache

When the project is configured and `sccache` is found on `PATH`, the CMake
configure SHALL set the C++ compiler launcher to `sccache` unless the user
already supplied a launcher. A user-supplied launcher — via
`-DCMAKE_CXX_COMPILER_LAUNCHER=...` or the `CMAKE_CXX_COMPILER_LAUNCHER`
environment variable — SHALL be left unchanged and SHALL take precedence over
the default. When `sccache` is not on `PATH`, configuration SHALL proceed
without a compiler launcher and the generated build rules SHALL NOT contain an
unresolved launcher command, so the build still succeeds. The default SHALL
apply to targets created after it is set, without requiring any command-line
flag.

#### Scenario: sccache is found and no launcher is set [lbc_default]

- **WHEN** the project is configured with no compiler-launcher override and `sccache` is on `PATH`
- **THEN** the generated build rules launch the C++ compiler through `sccache`

#### Scenario: an explicit launcher wins [lbc_override]

- **WHEN** the project is configured with an explicit `CMAKE_CXX_COMPILER_LAUNCHER` (for example `ccache`) while `sccache` is also on `PATH`
- **THEN** the build rules use the explicit launcher and do not substitute `sccache`

#### Scenario: sccache is absent [lbc_absent]

- **WHEN** the project is configured and `sccache` is not on `PATH`
- **THEN** configuration succeeds, the build rules name no compiler launcher, and no unresolved launcher command is generated

### Requirement: A clean rebuild reuses cached C++ objects

Once a build has populated the local `sccache` cache, rebuilding unchanged
sources SHALL reuse cached objects, observable through the cache hit counter
`SCCACHE` reports.

#### Scenario: a rebuild hits the cache [lbc_rebuild_hits]

- **WHEN** a build populates the cache and the same sources are built again after removing the build outputs
- **THEN** `sccache --show-stats` reports a non-zero cache hit count

### Requirement: Local build cache location and bound are documented

`DEVELOPING.md` SHALL document that local `sccache` caching is on by defaulting
the CMake compiler launcher, the default on-disk cache location and size cap,
the environment variables that change them, and how to make a build tolerate an
unreachable cache server.

#### Scenario: cache guidance is documented [lbc_docs]

- **WHEN** a reader opens the local-build caching section of `DEVELOPING.md`
- **THEN** it names the default cache directory, the default size cap, the variables that override them, and the option that lets a build proceed when the cache server is unreachable
