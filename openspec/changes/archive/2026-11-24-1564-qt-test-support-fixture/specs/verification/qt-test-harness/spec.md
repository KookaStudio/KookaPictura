# Spec Delta

## ADDED Requirements

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
