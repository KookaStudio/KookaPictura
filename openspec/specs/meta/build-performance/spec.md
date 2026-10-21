# meta/build-performance Specification

## Purpose
Keeps the local development build fast — linker, per-source optimization, link
concurrency, and moc compilation — while leaving release artifacts and
observable application behavior unchanged.

## Requirements

### Requirement: Development C++ links use mold when available

The CMake configure SHALL select `mold` as the C++ linker on ELF platforms
(Linux and FreeBSD) when a `mold` executable is found and the compiler accepts
`-fuse-ld=mold`. When mold is not usable it SHALL fall back to `lld` when the
compiler accepts it, and otherwise to the compiler's default linker. macOS and
Windows SHALL keep their native linker. An explicit linker choice supplied by
the user SHALL take precedence over all of these. Every fallback path SHALL
configure and link successfully. The Cargo linker configuration SHALL remain
`lld`, because a pinned `-fuse-ld=mold` has no automatic fallback.

#### Scenario: mold is installed [bp_mold]

- **WHEN** the project is configured on an ELF platform where `mold` is on `PATH` and the compiler accepts it
- **THEN** the generated C++ link commands use mold

#### Scenario: mold is absent [bp_mold_fallback]

- **WHEN** the project is configured on an ELF platform where `mold` is not on `PATH`
- **THEN** configuration succeeds and the C++ link commands use lld when it is usable, otherwise the compiler default

#### Scenario: an explicit linker choice wins [bp_mold_override]

- **WHEN** the user configures with an explicit linker selection while mold is also installed
- **THEN** the explicit selection is used and mold is not substituted

#### Scenario: non-ELF platforms keep the native linker [bp_mold_platform]

- **WHEN** the project is configured on macOS or Windows
- **THEN** no `-fuse-ld` flag is added and the platform linker is used

### Requirement: Development compile settings are cheaper for known-heavy units

In an optimized development configuration (`BUILD_TESTING` on with Release or
RelWithDebInfo), `command_tree.cpp` and `filter_commands.cpp` SHALL compile at
`-O1` with GCC, Clang, or AppleClang, and each Qt Test translation unit SHALL
compile at `-g1` with those compilers. Release packaging and a Debug configure
SHALL keep the normal configuration's optimization and debug settings for the
same sources.

#### Scenario: heavy command sources compile at O1 [bp_cheap_tu]

- **WHEN** an optimized development build compiles `command_tree.cpp` or `filter_commands.cpp` with GCC, Clang, or AppleClang
- **THEN** its compile command carries `-O1`

#### Scenario: Qt Test units compile with reduced debug info [bp_test_debug]

- **WHEN** a development build compiles a Qt Test translation unit with GCC, Clang, or AppleClang
- **THEN** its compile command carries `-g1` rather than full `-g`

### Requirement: Concurrent link steps are bounded

When the Ninja generator is used, the build SHALL cap concurrent link steps with
a job pool of at most four, so a highly parallel compile does not run an
unbounded number of large links at once. Compilation SHALL remain governed by
the build's parallel level.

#### Scenario: links run in a bounded pool [bp_link_pool]

- **WHEN** the generated Ninja build is inspected
- **THEN** link edges reference a link job pool with a depth of at most four

### Requirement: Generated moc output compiles with its own translation unit

Each QObject header that has a same-named `.cpp` SHALL have that `.cpp` include
the generated `moc_<header>.cpp`. A QObject header with no same-named `.cpp`
SHALL have its generated moc included from one of the `.cpp` files that includes
the header. The shell target's `mocs_compilation.cpp` SHALL contain no moc
output. A change to a QObject header SHALL recompile the translation unit that
includes its moc, not a central moc compilation unit.

#### Scenario: moc files are included by their own units [bp_moc_includes]

- **WHEN** the C++ sources are inspected after the change
- **THEN** every matching QObject `.cpp` includes its `moc_<header>.cpp`, the header without a same-named `.cpp` has its moc included from one of its consumers, and `mocs_compilation.cpp` contains no moc include

#### Scenario: a header edit stays local [bp_moc_incremental]

- **WHEN** one QObject header changes and the project is rebuilt
- **THEN** the build recompiles that header's moc-owning translation unit and does not recompile a central moc compilation unit

### Requirement: QMAKE is stable across CMake and direct Cargo invocations

Every cargo invocation — the one the CMake build drives, a bare `cargo build`,
`cargo clippy`, `cargo nextest`, and rust-analyzer's flycheck — SHALL observe
the same `QMAKE` value, so the cxx-qt build script's
`rerun-if-env-changed=QMAKE` fingerprint does not change between them. The
value SHALL come from one source of truth that does not depend on which tool
started the build; the local verification scripts SHALL NOT export a `QMAKE` of
their own, the helper script that did SHALL be removed, and the CMake build
SHALL NOT export one to the cargo invocation it drives. The value SHALL name
a qmake the cxx-qt build script can resolve to a path, because it runs qmake
with a cleared environment. Selecting a genuinely different qmake SHALL still
rerun the build script.

#### Scenario: alternating gates do not rerun the build script [bp_qmake_stable]

- **WHEN** a CMake build tree exists and `verify-fast.sh` runs Cargo directly after `verify-full.sh`
- **THEN** both cargo invocations observe the same `QMAKE` value and the cxx-qt build script does not rerun solely because `QMAKE` changed

#### Scenario: a different qmake still reruns [bp_qmake_change]

- **WHEN** `QMAKE` names a different qmake installation than the previous run
- **THEN** the cxx-qt build script reruns

### Requirement: The cxx-qt build script tracks only the bridge sources

The cxx-qt build script SHALL rerun only when a file it consumes changes: the
bridge Rust sources that define the generated C++ interface, and the
hand-written C++ headers and sources those bridges include. A change to any
other file under the crate — a shell translation unit, a test source, a
document — SHALL NOT rerun the build script and SHALL NOT cause the generated
cxx-qt headers to be rewritten. The hand-written bridge edges SHALL be
tracked explicitly by the build script, because the C compiler helper it uses
emits no change-tracking directive for the sources it compiles.

#### Scenario: a shell source does not rerun the build script [bp_bridge_tracking]

- **WHEN** a shell C++ translation unit that no bridge includes changes
- **THEN** the build script does not rerun and the generated headers keep their modification times

#### Scenario: a bridge header is tracked [bp_bridge_header_tracking]

- **WHEN** a hand-written C++ header that a bridge includes changes
- **THEN** the build script reruns and the Rust static library is rebuilt with the new header

### Requirement: Generated headers keep their timestamp when their content is unchanged

When the cxx-qt build script regenerates the cxx-qt headers, it SHALL write
each header only when the generated content differs from the content already
on disk. A rerun that produces identical headers SHALL leave their
modification times unchanged, so the C++ build does not treat them as
changed. When the content does differ, the header's modification time SHALL
advance, so the C++ build sees the new interface.

#### Scenario: an identical rerun leaves the timestamps alone [bp_header_stable_mtime]

- **WHEN** the build script reruns and the regenerated headers are byte-identical to the ones on disk
- **THEN** their modification times are unchanged

#### Scenario: a changed header advances its timestamp [bp_header_changed_mtime]

- **WHEN** the build script reruns and a regenerated header differs from the one on disk
- **THEN** that header's modification time advances

### Requirement: A bridge change rebuilds the C++ in the same build

When a bridge Rust source changes, the C++ translation units that include the
generated cxx-qt headers SHALL be recompiled by the same build that
regenerates those headers, and SHALL compile against the regenerated content.
The build system SHALL detect the bridge change before it decides what to
compile, by refreshing — at configure time — a stamp that records the content
of the bridge sources and that the C++ objects depend on. A build that follows
a bridge change SHALL NOT require a second build to pick the new interface
up, and a build in which nothing changed SHALL NOT recompile the C++.

#### Scenario: the bridge change lands in one build [bp_bridge_same_build]

- **WHEN** a bridge Rust source changes and the project is built once
- **THEN** the C++ translation units that include the generated header are recompiled in that build, and the next build recompiles none of them

#### Scenario: an unchanged build recompiles nothing [bp_bridge_no_change]

- **WHEN** nothing under the bridge sources or the shell sources changed and the project is rebuilt
- **THEN** no C++ translation unit is recompiled

### Requirement: The heaviest C++ translation units are measured and cheapened

The build system SHALL measure the cost of the heaviest C++ translation units
— the shell's menu construction and the Qt Test translation units — and SHALL
cheapen the ones that dominate, by splitting a heavy source along a concern
seam, by reducing their debug information, or by reducing the per-suite moc
cost, whichever the measurement shows pays. A cheapening SHALL be kept only
when it measurably shortens the clean build; a measured change that does not
pay SHALL be recorded as skipped in the change's design notes rather than
shipped, and the record SHALL name the numbers it was decided on.

#### Scenario: a heavy source is split or the split is declined [bp_heavy_tu_cheapened]

- **WHEN** the menu-construction source is measured as the heaviest translation unit
- **THEN** it is split along a concern seam as a pure move, or the measurement is recorded as not worth the split

#### Scenario: the test-unit cost is reduced or declined [bp_test_tu_cost]

- **WHEN** the Qt Test translation units are measured as the dominant C++ compile cost
- **THEN** the per-suite moc cost is reduced, or the measurement is recorded as not worth changing
