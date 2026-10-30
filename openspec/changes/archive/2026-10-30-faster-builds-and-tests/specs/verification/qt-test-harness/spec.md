## MODIFIED Requirements

### Requirement: Qt Test and CTest harness

The build SHALL call `include(CTest)` gated on `BUILD_TESTING` and SHALL add a
`crates/pictura-app/cpp/tests/` directory with its own `CMakeLists.txt`. That
file SHALL build each `tst_*.cpp` as its own executable, linking `pictura_shell`
and `Qt6::Test`, and SHALL register each executable with `add_test` under
`ENVIRONMENT QT_QPA_PLATFORM=offscreen` and `TIMEOUT 120`. Each `add_test` SHALL
pass `-o <CMAKE_BINARY_DIR>/qt-test-results/<name>.xml,junitxml` so CTest itself
emits a per-executable JUnit report. The harness SHALL be gated on
`BUILD_TESTING` with no effect when it is off. CTest SHALL own only the Qt Test
executables and SHALL NOT wrap or replace `cargo nextest`; the Rust suite stays with
`scripts/test-report.sh`.

#### Scenario: Configure builds the suites

- **WHEN** CMake configures with `BUILD_TESTING` on
- **THEN** the tests subdirectory is added and each `tst_*.cpp` is built as its
  own executable linked against `pictura_shell` and `Qt6::Test`

#### Scenario: CTest runs each suite offscreen

- **WHEN** `ctest` runs a registered Qt test executable
- **THEN** it runs with `QT_QPA_PLATFORM=offscreen` and a `TIMEOUT` of 120
  seconds

#### Scenario: CTest emits per-executable JUnit

- **WHEN** `ctest` runs a registered Qt test executable
- **THEN** the executable writes a JUnit report to
  `build/qt-test-results/<name>.xml`

#### Scenario: Tests are optional

- **WHEN** CMake configures with `BUILD_TESTING` off
- **THEN** no test executable or test registration is emitted

#### Scenario: CTest does not wrap nextest

- **WHEN** the harness is built
- **THEN** no CTest test invokes `cargo nextest`
