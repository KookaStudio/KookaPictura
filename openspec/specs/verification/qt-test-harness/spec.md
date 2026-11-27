# verification/qt-test-harness Specification

## Purpose
The Qt Test + CTest harness for the C++ GUI shell: an application static library
that subsystem driver executables link, one suite per CTest entry dispatched by
name, per-suite state isolation, the seeded suites migrated off the self-test, and
the contract that the monolithic self-test only shrinks as checks move.

## Requirements

### Requirement: Application shell library

The C++ application SHALL be split into a `pictura_shell` library holding every
application source except `main.cpp` and the `selftest*.cpp`/`selftest*.h`
harness files, and a `pictura` executable holding `main.cpp` plus the self-test
harness that links `pictura_shell`. The split SHALL be a pure move with no
behavior change: the `pictura` executable's observable behavior, including the
`--headless --self-test` token stream and exit codes, SHALL be unchanged.
`pictura_shell` SHALL expose `crates/pictura-app/cpp` and the cxx-qt generated
include directory as PUBLIC include directories, so a target that links
`pictura_shell` can include both the hand-written and the generated headers.

`pictura_shell` SHALL be STATIC when `BUILD_TESTING` is off, and SHARED when
`BUILD_TESTING` is on and the platform is not Windows. In the STATIC
configuration the shell SHALL link `pictura_app` (the Rust staticlib),
`Qt6::Svg`, `Qt6::Network`, and lcms2 with PUBLIC visibility, and each consumer
SHALL link `pictura_assets` directly. In the SHARED configuration the shell
SHALL link `pictura_app`, the resource object, `Qt6::Svg`, `Qt6::Network`, and
lcms2 PRIVATE, SHALL publish the generated cxx-qt include directory, and each Qt
Test executable SHALL link the shared library and `Qt6::Test` (inheriting the
shell's public Qt modules) and SHALL NOT link the Rust staticlib, the cxx-qt
initializer object, or the resource object, so none of them is duplicated into
every executable. The `pictura` executable SHALL also link only
the shared library in that configuration. The C++ compiled by the cxx-qt build
script SHALL be position-independent in the SHARED configuration. The `pictura`
executable in the SHARED configuration SHALL resolve the library through the
build tree's runtime path.

The STATIC configuration SHALL remain the one used by release, packaging, and
`cmake --install`, and the resulting `pictura` binary SHALL remain
self-contained. Installing a SHARED development build SHALL NOT be supported.

#### Scenario: The executable keeps only main and the harness

- **WHEN** the shell library is built
- **THEN** `pictura_shell` contains no `main.cpp` and no `selftest*` source, and
  the `pictura` executable links it

#### Scenario: A test executable inherits the cxx-qt surface

- **WHEN** a test executable links `pictura_shell`
- **THEN** it can include the cxx-qt generated headers and link lcms2 without
  declaring either itself

#### Scenario: The self-test behavior is unchanged

- **WHEN** the relocated `pictura` binary runs `--headless --self-test`
- **THEN** it emits the same tokens and returns the same exit codes as before
  the split

#### Scenario: Development builds link one shared shell [qth_shell_shared]

- **WHEN** the project is configured with `BUILD_TESTING` on and built on Linux or macOS
- **THEN** `pictura_shell` is produced as a shared library, each Qt Test executable links it and `Qt6::Test` without the static archives, and every test suite still passes

#### Scenario: Release builds keep a static shell [qth_shell_static_release]

- **WHEN** the project is configured with `BUILD_TESTING` off and installed into a temporary prefix
- **THEN** `pictura_shell` is static, the installed `pictura` needs no separate shell library, and the installed binary passes `--headless --self-test`

#### Scenario: Windows keeps the static shell [qth_shell_windows]

- **WHEN** the project is configured with `BUILD_TESTING` on for Windows
- **THEN** `pictura_shell` stays static so no symbol-export declarations are required

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

#### Scenario: one process and one entry per suite [qth_suite_process]

- **WHEN** the layer is registered
- **THEN** every suite runs in its own process and owns exactly one CTest entry per share, with no suite grouped into another suite's process

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

#### Scenario: the sharded main isolates like QTEST_MAIN [qth_sharded_main_isolation]

- **WHEN** a sharded suite runs under any share
- **THEN** it constructs the state-home fixture before its window and enters through `shardedMain`, so it gets the same application and isolation as `QTEST_MAIN`

### Requirement: Self-test migration and code retirement

The Qt Test suites SHALL be populated by MOVING checks off the monolithic
self-test, not by duplicating them: a migrated check SHALL be DELETED from its
`selftest*.cpp` source. A retired self-test exit code SHALL never be reused; the
next self-test check SHALL take the next unused code. `design.md` SHALL record
every retired check by name and integer exit code. New GUI checks SHALL be
written as Qt Test cases; a new self-test check MAY be added only to preserve the
`--headless` smoke contract and the `--self-test` exit-code identity.
`scripts/check-file-size.sh` SHALL continue to classify
`crates/pictura-app/cpp/tests/` as tests under the 1400-line budget with no rule
change. The shrink-only rule SHALL be enforced mechanically by
`scripts/check-selftest-budget.sh`, which counts `ST_BEGIN` sites across
`crates/pictura-app/cpp/**/*.cpp` and fails when the count exceeds the budget in
`scripts/selftest-budget.txt`; the budget is lower-only, and the script SHALL run
from `scripts/verify-fast.sh` and the guards CI workflow. This is in addition to
the `selftest.cpp` entry in `scripts/file-size-allowlist.txt`, which remains a
ceiling that may only shrink.

#### Scenario: A migrated check leaves the self-test

- **WHEN** a check is ported to a Qt Test suite
- **THEN** its `ST_BEGIN` block is removed from the self-test and the check runs
  only through CTest

#### Scenario: A retired code is never reused

- **WHEN** a self-test check is retired
- **THEN** its integer exit code is not assigned to any later check

#### Scenario: New checks use Qt Test

- **WHEN** a new GUI check is added
- **THEN** it is a Qt Test case, unless it preserves the `--headless` smoke
  contract or the `--self-test` exit-code identity

#### Scenario: Test files get the test budget

- **WHEN** `scripts/check-file-size.sh` runs on a file under
  `crates/pictura-app/cpp/tests/`
- **THEN** it applies the 1400-line test budget

#### Scenario: The self-test budget only shrinks

- **WHEN** `scripts/check-selftest-budget.sh` runs and a new `ST_BEGIN` site
  raises the count above `scripts/selftest-budget.txt`
- **THEN** the script fails

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
to the shard registration. Shard membership and share count SHALL be derived
from the retained timing evidence and reported against that quarter budget.
Driver consolidation SHALL preserve existing registrations and both
`tst_quick_mask` and `tst_document_navigate` SHALL retain four shares until
matched timing evidence supports a change. Quick Mask SHALL retain a
300-second timeout in every share.

#### Scenario: the two long suites use disjoint complete shares [qth_long_suites_sharded]

- **WHEN** all registered shares of `tst_quick_mask` and `tst_document_navigate` run
- **THEN** each suite's test functions run exactly once across its four shares, Quick Mask keeps a 300-second timeout per share, and Document Navigate keeps the default 120-second timeout per share

#### Scenario: the critical path names more than one suite [qth_critical_path]

- **WHEN** the CTest run finishes and prints its critical path
- **THEN** the critical path names at least two suites

#### Scenario: no suite is reported as timed out [qth_suite_timeout]

- **WHEN** the CTest run finishes
- **THEN** no suite is reported as timed out

#### Scenario: the longest entry is bounded [qth_longest_entry]

- **WHEN** the Qt Test layer runs at the default parallel level
- **THEN** no entry's time in CTest's report exceeds a quarter of CTest's total real time, or the entry is named next to the shard registration with the reason it cannot be split

### Requirement: Qt Test suites wait on conditions, not fixed delays

A Qt Test suite SHALL wait for an observable condition — a widget state, a
signal, a file, a timer having fired — through a bounded condition wait that
returns as soon as the condition holds, rather than through a fixed sleep. A
wait whose only purpose is to deliver events already queued SHALL be an event
flush that does not sleep: a zero-length wait or delivery of the posted events.
A fixed delay SHALL remain only where no observable condition exists, such as
proving that something does *not* happen within a window, and each remaining
delay SHALL carry a comment on the same or the preceding line naming why no
condition can replace it. A repository guard run by local verification, the
guards CI workflow, and the Qt-aware CI job SHALL enforce this rule on the Qt
Test sources, so it runs on a C++-only change.

#### Scenario: fixed delays are justified or gone [qth_condition_wait]

- **WHEN** the repository guard scans the Qt Test sources
- **THEN** every fixed delay with a non-zero length carries a comment on its line or the line above, and the guard fails on an uncommented one

#### Scenario: a negative wait keeps its delay [qth_negative_wait]

- **WHEN** a test asserts that an event does not occur within a time window
- **THEN** it keeps a fixed delay of that window, with the comment that names it

### Requirement: A suite shares one window only when a reset makes its functions independent

A Qt Test suite MAY build one main window for all its test functions instead
of one per function only when its per-function reset restores every piece of
window and persisted state that a test function reads, so each function passes
when run alone and in any shard. A suite that shares a window and is registered
in several CTest entries SHALL keep passing in every entry. A suite whose reset
cannot be shown complete SHALL keep building a window per test function. A
suite executable SHALL run exactly the test functions named on its command
line when any are named, whether or not it is registered in shards.

#### Scenario: naming functions runs only those functions [qth_named_functions]

- **WHEN** a sharded suite executable is started with one test function named on its command line
- **THEN** it runs that function and no other

#### Scenario: a shared-window suite passes function by function [qth_shared_window_isolated]

- **WHEN** each test function of a shared-window suite is run alone by name
- **THEN** every function passes

#### Scenario: a shared-window suite passes in every shard [qth_shared_window_shards]

- **WHEN** a sharded shared-window suite runs under each value of its shard variable
- **THEN** every entry passes and together they run every test function once

### Requirement: The shared Qt Test fixture compiles once

The helper API the Qt Test suites share — the state-home and locale scopes, the
window factory, the completion and font helpers, the test-function and shard
helpers, and the sharded test main — SHALL have its non-template bodies compiled
once per build rather than in every suite that includes the fixture, and every
suite SHALL obtain them from one shared library. The fixture's header SHALL
declare them; the library SHALL define them. A template the suites instantiate
with their own test object SHALL remain in the header, because a template cannot
be compiled out of line.

The support library SHALL NOT run moc over its sources: its translation unit
includes headers that declare QObject subclasses, and a moc scan of it would
emit a second meta-object for them. The suites' existing precompiled header and
its reuse wiring SHALL be unchanged, and a suite SHALL still compile with the
support library when precompiled headers are disabled.

Scoping of a suite's persisted state SHALL keep working exactly as before: the
state-home fixture is still constructed before the window and destroys after it,
whatever translation unit its constructor runs in.

#### Scenario: A fixture body is compiled once [qts_fixture_single_object]

- **WHEN** the build compiles the Qt Test layer
- **THEN** the shared fixture's non-template bodies produce one object file, and each suite that includes the fixture links that object rather than compiling the bodies itself

#### Scenario: The state scope keeps isolating [qts_fixture_state_scope]

- **WHEN** a suite constructs the state-home fixture before its window and destroys the window before the fixture
- **THEN** `XDG_STATE_HOME`, `XDG_CONFIG_HOME`, and `XDG_CONFIG_DIRS` point at the fixture's temporary directory while it lives, and are restored to their prior values (or unset) afterwards, whichever translation unit defines the fixture

#### Scenario: The suites still pass [qts_fixture_behaviour]

- **WHEN** the Qt Test layer runs after the split
- **THEN** every registered suite runs its functions with the same assertions and the same result as before the split

#### Scenario: The support unit does not duplicate a meta-object [qts_fixture_no_moc]

- **WHEN** the support library's translation unit is compiled
- **THEN** it declares no `Q_OBJECT` of its own and no meta-object for a header it includes is emitted twice

#### Scenario: A suite builds without a precompiled header [qts_fixture_no_pch]

- **WHEN** the project is configured with precompiled headers disabled and the Qt Test layer is built
- **THEN** every suite compiles and links against the shared fixture library

### Requirement: The Qt Test fixture does not require the main window definition

The fixture's header SHALL NOT include `frame.h`. Where a fixture helper creates
or returns the application's main window, the header SHALL name the type through
a forward declaration, and the definition SHALL come from the suite that uses the
window's members. A suite that needs the complete main-window type SHALL include
`frame.h` itself, so the dependency is explicit at the suite rather than implied
by a shared header.

That a suite reaches `frame.h` through another of its own headers satisfies the
rule; the fixture SHALL NOT be that path.

#### Scenario: The fixture header is window-definition free [qts_fixture_no_frame_h]

- **WHEN** the fixture header and the translation units under `crates/pictura-app/cpp/tests/` are inspected
- **THEN** no `#include "frame.h"` appears in the fixture header, and every suite that dereferences the window includes `frame.h` or a header that provides it

#### Scenario: The check cannot regress [qts_fixture_seam_guard]

- **WHEN** a `#include "frame.h"` is added back to the fixture header
- **THEN** the repository check fails and names the fixture header

### Requirement: Shard membership and count come from retained duration evidence

Shard membership and share count SHALL come from the retained timing evidence,
not from a hand-maintained suite list. `scripts/qt_test_timings.py plan` SHALL
write `cmake/qt-test-shards.json` from the timing store: per sharded suite, a
share count, the measured seconds that justify it, and — when per-function
evidence exists — the exact test functions of each share. The count SHALL be the
fewest shares that keep one entry within a quarter of the layer's measured total,
capped by the suite's function count so no share is empty. The algorithm SHALL
sort functions longest-first and fill the emptiest share.

The test build SHALL read that plan: a suite in the plan SHALL register one CTest
entry per share under the names `<suite>_<index>` with
`PICTURA_TEST_SHARD=<index>/<count>`, a measured share SHALL name its functions on
the command line, and a share without named functions SHALL fall back to the
existing declaration-order split in `shardedMain`. A missing plan SHALL warn and
register every suite in one entry rather than fail configuration, and a plan entry
that names no registered suite SHALL fail configuration so a rename cannot
silently drop sharding. The fixed shard scheduler SHALL remain `shardedMain`; no
second scheduler SHALL be added.

`scripts/qt_test_timings.py verify` SHALL reject a plan whose share is empty,
whose function appears in more than one share, or which omits a function the
suite source declares, and SHALL run in the Qt-aware CI job. Regenerating a plan
from a partial store SHALL carry a baseline entry forward instead of silently
un-sharding its suite.

#### Scenario: a measured suite is sharded from its duration [qth_plan_duration]

- **WHEN** the timing store records a suite above a quarter of the layer's measured total
- **THEN** the generated plan gives it more than one share, names the measured seconds as its evidence, and puts each of its functions in exactly one share

#### Scenario: a share is never empty [qth_plan_no_empty_share]

- **WHEN** the duration-derived share count would exceed the suite's function count
- **THEN** the planner caps the count at the function count so no empty share is written, and a plan that declares an empty share is rejected

#### Scenario: an empty or duplicate share is rejected [qth_plan_verify]

- **WHEN** a plan's share is empty, a function repeats across shares, or a declared function is missing
- **THEN** `scripts/qt_test_timings.py verify` exits non-zero and names the suite and the problem

#### Scenario: named functions still run exactly those functions [qth_plan_named_functions]

- **WHEN** a shard entry passes its planned function names on the command line
- **THEN** the suite runs exactly those functions and no others, and the empty-shard detection is unchanged

#### Scenario: a partial store keeps a previous share decision [qth_plan_carry_forward]

- **WHEN** the planner regenerates from a store with no measurement for a suite the existing plan shards
- **THEN** the suite keeps its previous share count and the entry's evidence says it was carried forward
