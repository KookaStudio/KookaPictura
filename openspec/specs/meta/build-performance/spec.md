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

When a CMake build tree exists, the local verification scripts SHALL export the
absolute `QMAKE` path recorded in that tree's `CMakeCache.txt`
(`QMAKE_EXECUTABLE`) before running Cargo, so the cxx-qt build script's
`rerun-if-env-changed=QMAKE` fingerprint does not change between a CMake build
and a direct Cargo run. When no build tree exists, the scripts SHALL resolve the
same `qmake6`-then-`qmake` order the configure uses from `PATH`. Selecting a
genuinely different qmake SHALL still rerun the build script.

#### Scenario: alternating gates do not rerun the build script [bp_qmake_stable]

- **WHEN** a CMake build tree exists and `verify-fast.sh` runs Cargo directly after `verify-full.sh`
- **THEN** it exports that tree's cached `QMAKE_EXECUTABLE` and the cxx-qt build script does not rerun solely because `QMAKE` changed

#### Scenario: a different qmake still reruns [bp_qmake_change]

- **WHEN** `QMAKE` names a different qmake installation than the previous run
- **THEN** the cxx-qt build script reruns
