# meta/local-build-caching Specification

## Purpose
Keeps local C++ compilation warm by defaulting the CMake compiler launcher to an
already-installed `sccache`, so plain configures and clean rebuilds reuse cached
objects instead of recompiling every translation unit.

## Requirements

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
`SCCACHE` reports. The proof SHALL rebuild after removing at most one probe
object, and SHALL NOT delete unrelated objects, test objects, archives, or test
executables from the build tree it inspects.

#### Scenario: a rebuild hits the cache [lbc_rebuild_hits]

- **WHEN** a build populates the cache, one probe object is removed, and the target that owns it is rebuilt
- **THEN** `sccache --show-stats` reports a non-zero cache hit count

#### Scenario: the proof leaves other outputs in place [lbc_probe_only]

- **WHEN** the rebuild proof runs against a completed build tree
- **THEN** only the probe object was removed and every other compiled object and test executable is still present

### Requirement: Local build cache location and bound are documented

`DEVELOPING.md` SHALL document that local `sccache` caching is on by defaulting
the CMake compiler launcher, the default on-disk cache location and size cap,
the environment variables that change them, and how to make a build tolerate an
unreachable cache server.

#### Scenario: cache guidance is documented [lbc_docs]

- **WHEN** a reader opens the local-build caching section of `DEVELOPING.md`
- **THEN** it names the default cache directory, the default size cap, the variables that override them, and the option that lets a build proceed when the cache server is unreachable

### Requirement: Repeated checker configures reuse fetched dependencies

The local build-caching check SHALL configure its throwaway build directories
with a shared FetchContent base directory, so one run fetches cxx-qt-cmake and
Corrosion once instead of once per configure, and later runs reuse the fetched
sources. The shared location SHALL live outside the repository. The check
SHALL build only in the build tree it was asked to inspect, never in the shared
base, and concurrent runs SHALL be serialized by the existing opt-in
verification lock or otherwise avoid writing the shared base at the same time.
In its default mode the check SHALL configure the project exactly once and
SHALL assert the launcher rule of that single configuration; the remaining
launcher cases SHALL remain available as a full variant, which the CI job
SHALL run.

#### Scenario: one fetch for three configures [lbc_fetchcontent_shared]

- **WHEN** the build-caching check configures its default, override, and absent cases
- **THEN** cxx-qt-cmake and Corrosion are fetched once for the whole run

#### Scenario: a later run reuses the fetch [lbc_fetchcontent_reuse]

- **WHEN** the build-caching check runs again on the same machine
- **THEN** it reuses the fetched sources instead of cloning them again

#### Scenario: the default mode configures once [lbc_default_one_configure]

- **WHEN** the build-caching check runs in its default mode
- **THEN** it configures the project exactly once and still asserts the launcher rule of that configuration

#### Scenario: the full variant keeps every case [lbc_full_variant]

- **WHEN** the build-caching check runs in its full variant
- **THEN** it asserts the default, explicit-override, and absent-launcher launcher rules
