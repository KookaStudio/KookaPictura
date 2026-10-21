# Spec Delta

## MODIFIED Requirements

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
