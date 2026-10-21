# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Repeated checker configures reuse fetched dependencies

The local build-caching check SHALL configure its throwaway build directories
with a shared FetchContent base directory, so one run fetches cxx-qt-cmake and
Corrosion once instead of once per configure, and later runs reuse the fetched
sources. The shared location SHALL live outside the repository. The check SHALL
build only in the build tree it was asked to inspect, never in the shared base,
and concurrent runs SHALL be serialized by the existing opt-in verification
lock or otherwise avoid writing the shared base at the same time.

#### Scenario: one fetch for three configures [lbc_fetchcontent_shared]

- **WHEN** the build-caching check configures its default, override, and absent cases
- **THEN** cxx-qt-cmake and Corrosion are fetched once for the whole run

#### Scenario: a later run reuses the fetch [lbc_fetchcontent_reuse]

- **WHEN** the build-caching check runs again on the same machine
- **THEN** it reuses the fetched sources instead of cloning them again
