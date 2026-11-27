# Spec Delta

## ADDED Requirements

### Requirement: CI Qt setup remains an adapter

The CI Qt job SHALL retain ownership of Qt and TIFF runtime installation,
compiler-cache setup, CI-specific CMake options, and runner CPU selection. It
SHALL provide the TIFF runtime before invoking the shared Qt executor so CTest
inherits the same library path. The job SHALL NOT keep separate Configure,
Build, Self-test, CTest, or Qt report steps that duplicate the shared executor.

#### Scenario: TIFF setup precedes shared execution [qe_ci_tiff]

- **WHEN** the CI Qt job prepares the TIFF runtime and invokes `scripts/test-report.sh`
- **THEN** the runtime extraction and `LD_LIBRARY_PATH` setup occur before the shared executor can start CTest

#### Scenario: CI uses the runner CPU count [qe_ci_cpu]

- **WHEN** the CI Qt job invokes the shared executor without `CTEST_PARALLEL_LEVEL`
- **THEN** the build and CTest default to the runner's full CPU count, while an explicit `CTEST_PARALLEL_LEVEL` still wins

#### Scenario: CI keeps only one Qt orchestration step [qe_ci_orchestration]

- **WHEN** the workflow is inspected
- **THEN** one step invokes the shared Qt executor and no other step directly invokes the two self-test modes, CTest, or the Qt reporter
