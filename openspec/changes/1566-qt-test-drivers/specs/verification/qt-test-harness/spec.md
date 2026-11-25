# Spec Delta

## MODIFIED Requirements

### Requirement: Qt Test and CTest harness

The build SHALL call `include(CTest)` gated on `BUILD_TESTING` and SHALL add a
`crates/pictura-app/cpp/tests/` directory with its own `CMakeLists.txt`. That file
SHALL own one explicit registry of Qt Test suites and SHALL build those suites as
a smaller set of subsystem driver executables linked to `pictura_shell`,
`Qt6::Test`, and the shared test support library. Each registered suite SHALL map
to exactly one driver. Configuration SHALL fail when a standalone suite source
is missing from the registry or a suite appears in more than one subsystem list.
Sources explicitly listed as included implementation files SHALL NOT be treated
as standalone suites. Each CTest entry SHALL invoke that driver with
`--suite <name>` and SHALL run in its own process. Sharded suites SHALL keep one
entry per share, with the share selected by `PICTURA_TEST_SHARD`.

Each `add_test` SHALL run under `ENVIRONMENT QT_QPA_PLATFORM=offscreen`, keep its
per-suite `TIMEOUT`, and pass
`-o <CMAKE_BINARY_DIR>/qt-test-results/<entry>.xml,junitxml`. The driver SHALL
require `--suite` as its first argument and a non-empty registered suite name as
its second argument. It SHALL pass the suite name as `argv[0]` and every later
argument unchanged to the selected suite, and SHALL return that suite's exit
status unchanged. A missing, incomplete, malformed, unknown, or duplicate suite
selection SHALL fail before any test starts. The harness SHALL be gated on
`BUILD_TESTING` with no effect when it is off. CTest SHALL own only the Qt Test
drivers and SHALL NOT wrap or replace `cargo nextest`; the Rust suite stays with
`scripts/test-report.sh`.

#### Scenario: Configure builds the suites

- **WHEN** CMake configures with `BUILD_TESTING` on
- **THEN** the tests subdirectory is added, every `tst_*.cpp` belongs to exactly
  one `tst_driver_<subsystem>` executable linked against `pictura_shell` and
  `Qt6::Test`, no per-suite executable is emitted, and a missing, unregistered,
  or duplicate registry entry fails configuration

#### Scenario: CTest runs each suite offscreen

- **WHEN** `ctest` runs a registered Qt test entry
- **THEN** one driver process selects the named suite, the entry runs with
  `QT_QPA_PLATFORM=offscreen` and keeps its `TIMEOUT`, and a sharded entry runs
  only its share

#### Scenario: CTest emits per-executable JUnit

- **WHEN** `ctest` runs a registered Qt test entry
- **THEN** the selected suite writes a JUnit report to
  `build/qt-test-results/<entry>.xml`

#### Scenario: Tests are optional

- **WHEN** CMake configures with `BUILD_TESTING` off
- **THEN** no driver executable or test registration is emitted

#### Scenario: CTest does not wrap nextest

- **WHEN** the harness is built
- **THEN** no CTest test invokes `cargo nextest`

#### Scenario: The driver selects exactly one suite [qth_driver_selection]

- **WHEN** a driver receives `--suite <name>` followed by Qt Test arguments
- **THEN** it invokes only that suite, uses the suite name as `argv[0]`, forwards every later argument unchanged, and returns its exit status

#### Scenario: Invalid dispatch fails [qth_driver_invalid]

- **WHEN** a driver receives no suite selection, an incomplete or wrong-position selector, an empty name, a name with a `.cpp` suffix, an unknown suite, or duplicate registrations for the requested name
- **THEN** it exits nonzero without starting a suite

### Requirement: Per-suite state isolation

Each Qt Test suite SHALL isolate persisted state so no suite depends on or
mutates another's preferences or recovery state. Each suite SHALL declare a
`pictura::test::ScopedStateHome` RAII member constructed BEFORE its main window,
so the fixture outlives the window's session save. The fixture's constructor
SHALL point `XDG_STATE_HOME` at a fresh temporary directory, and its destructor
SHALL restore the previous value, including unsetting it only when it was not
set before, rather than blanket-unsetting. Isolation SHALL NOT rely on
`initTestCase()`: the fixture must be alive across window construction and
destruction, which `initTestCase()` cannot guarantee. The frame SHALL read
`XDG_STATE_HOME` and `QApplication` SHALL NOT, so the suite's existing main
SHALL remain responsible for constructing its test object and application.

Each CTest entry SHALL start a fresh driver process even when several entries
use the same driver executable. Dispatching to another suite in the same
process SHALL NOT satisfy this isolation rule.

#### Scenario: Suites do not share state

- **WHEN** two suites run in the same `ctest` invocation
- **THEN** each entry runs in its own process with its own temporary state home, so neither observes state written by the other

#### Scenario: Each run starts clean

- **WHEN** a suite writes preferences and the suite runs again
- **THEN** it starts from a clean temporary state directory

#### Scenario: A pre-existing state home is restored

- **WHEN** `XDG_STATE_HOME` is already set in the environment and the fixture is destroyed
- **THEN** the previous value is restored, not unset

### Requirement: Seeded suites

The initial harness SHALL seed four Qt Test suites: the three migrated suites
`tst_command_tree`, `tst_layers_panel`, and `tst_edit_clipboard`, plus
`tst_smoke`, the harness smoke suite that exercises the state-home fixture and
proves CTest runs a suite offscreen. These suites SHALL remain registered when
subsystem drivers replace per-suite executables. `tst_command_tree` SHALL cover
the command registry and menu tree, including the widget-level menu checks
`menus`, `dispatch`, `menu_count`, `menubar_clear`, `menus_panel`,
`panelMenus_tools_icons`, and `widgetmenu_button`. `tst_layers_panel` SHALL cover
Layers panel row controls and chrome, the opacity label, `%` suffix, semantic
lock icons, tree drag enabled, group expand and collapse, and drop bands via
`lpc_nesting`, `lpc_chrome`, `lpr_rows`, `lpr_drag`, and `lpr_drop`; it does not
assert row selection. `tst_edit_clipboard` SHALL cover the whole
`edit_clipboard` check.

#### Scenario: The seeded suites run

- **WHEN** `ctest` runs the harness
- **THEN** it runs `tst_smoke`, `tst_command_tree`, `tst_layers_panel`, and `tst_edit_clipboard` through their registered drivers

#### Scenario: The migrated checks leave the self-test

- **WHEN** the seed migration is complete
- **THEN** none of the named checks emits a self-test token

### Requirement: The Qt Test layer's critical path is not a single suite

The Qt Test layer SHALL be split so that the CTest run's critical path is not
one suite: the suites that determine the layer's wall time SHALL number at
least two, so the layer's duration is set by the number of suites and the
parallel level rather than by one slow suite. A suite that is inherently long
SHALL be split into additional registered entries, and the CTest parallel
level SHALL remain overridable through `CTEST_PARALLEL_LEVEL`. At the default
parallel level, no single registered entry SHALL take more than a quarter of
the layer's total wall time, both as CTest reports them, unless the entry's
test functions share state; such an entry SHALL be named, with its reason, next
to the shard registration.

`tst_quick_mask` and `tst_document_navigate` SHALL use the existing shard
mechanism. Their shares together SHALL run every test function exactly once.
Quick Mask's 300-second timeout SHALL apply to every share. Consolidation into
drivers SHALL NOT remove or rename any other existing registration.

#### Scenario: the critical path names more than one suite [qth_critical_path]

- **WHEN** the CTest run finishes and prints its critical path
- **THEN** the critical path names at least two suites

#### Scenario: no suite is reported as timed out [qth_suite_timeout]

- **WHEN** the CTest run finishes
- **THEN** no suite is reported as timed out

#### Scenario: the longest entry is bounded [qth_longest_entry]

- **WHEN** the Qt Test layer runs at the default parallel level
- **THEN** no entry's time in CTest's report exceeds a quarter of CTest's total real time, or the entry is named next to the shard registration with the reason it cannot be split

#### Scenario: the two long suites use disjoint complete shares [qth_long_suites_sharded]

- **WHEN** all registered shares of `tst_quick_mask` and `tst_document_navigate` run
- **THEN** each suite's test functions run exactly once across its four shares, Quick Mask keeps a 300-second timeout per share, and Document Navigate keeps the default 120-second timeout per share
