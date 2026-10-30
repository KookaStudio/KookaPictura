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
the user SHALL take precedence over all of these. When that explicit choice is
a `-fuse-ld=` in the cached linker flags and mold is installed and accepted,
the configure SHALL print a warning that names the cached flag and the
`cmake -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS` command that
clears it, so a build tree configured before the mold preference does not keep
an older linker without saying so. The warning SHALL be printed once per build
tree and cached flag value, so a deliberate choice is not warned about on every
configure. Every fallback path SHALL configure and link successfully. The Cargo
linker configuration SHALL remain `lld`, because a pinned `-fuse-ld=mold` has
no automatic fallback.

#### Scenario: mold is installed [bp_mold]

- **WHEN** the project is configured on an ELF platform where `mold` is on `PATH` and the compiler accepts it
- **THEN** the generated C++ link commands use mold

#### Scenario: mold is absent [bp_mold_fallback]

- **WHEN** the project is configured on an ELF platform where `mold` is not on `PATH`
- **THEN** configuration succeeds and the C++ link commands use lld when it is usable, otherwise the compiler default

#### Scenario: an explicit linker choice wins [bp_mold_override]

- **WHEN** the user configures with an explicit linker selection while mold is also installed
- **THEN** the explicit selection is used and mold is not substituted

#### Scenario: a cached linker flag that masks mold is reported once [bp_mold_masked_warning]

- **WHEN** a build tree whose cached linker flags carry `-fuse-ld=lld` is reconfigured on an ELF platform where mold is installed and accepted, and then reconfigured again with the same cached flag
- **THEN** both configures keep lld, the first prints a warning naming the cached flag and the `-U` command that clears it, and the second prints no warning

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
a `QMAKE` value that does not change between the invocations that share a
build tree, so the cxx-qt build script's
`rerun-if-env-changed=QMAKE` fingerprint does not change between them. The
local verification scripts SHALL NOT export a `QMAKE` of their own, and the
helper script that did SHALL be removed. The value SHALL be one the cxx-qt
build script can run: it runs qmake with a cleared environment, where a bare
name is not found on the PATH, so only a path works. Selecting a genuinely
different qmake SHALL still rerun the build script.

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

When the `#[cxx_qt::bridge]` module of a bridge Rust source changes, the C++
translation units that include the generated cxx-qt headers SHALL be
recompiled by the same build that regenerates those headers, and SHALL compile
against the regenerated content. The build system SHALL detect the bridge
change before it decides what to compile, by refreshing — at configure time —
a stamp that records the content of the bridge modules and that the C++
objects depend on. The stamp SHALL cover, for each bridge file, the bridge
module together with the outer attributes and doc comments directly above it,
and the file's path, and nothing else in that file. An edit outside that text
(an implementation body, a helper, a test) SHALL NOT change the stamp and
SHALL NOT recompile the C++. The stamp SHALL record the whole file instead when
it cannot find the bridge attribute or the module's closing line, or when the
text it extracted fails its consistency check: unequal `{` and `}` counts, an
odd number of unescaped `"`, or a `/*` block-comment opener. A failed check
therefore rebuilds too much rather than too little. Adding or removing a Rust source, or changing the
build script that lists the bridge files, SHALL refresh the stamp. A build that
follows a bridge change SHALL NOT require a second build to pick the new
interface up, and a build in which nothing changed SHALL NOT recompile the C++.

#### Scenario: the bridge change lands in one build [bp_bridge_same_build]

- **WHEN** a bridge module in a bridge Rust source changes and the project is built once
- **THEN** the C++ translation units that include the generated header are recompiled in that build, and the next build recompiles none of them

#### Scenario: an unchanged build recompiles nothing [bp_bridge_no_change]

- **WHEN** nothing under the bridge sources or the shell sources changed and the project is rebuilt
- **THEN** no C++ translation unit is recompiled

#### Scenario: an implementation edit in a bridge file recompiles no C++ [bp_bridge_impl_edit]

- **WHEN** a bridge Rust source changes only outside its `#[cxx_qt::bridge]` module and the attributes above it, and the project is rebuilt
- **THEN** the bridge stamp is unchanged and no C++ translation unit is recompiled

#### Scenario: a module the stamp cannot extract cleanly is hashed whole [bp_bridge_stamp_fallback]

- **WHEN** the stamp finds no closing line for a bridge module, or the extracted text has unbalanced braces, an odd number of unescaped quotes, or a `/*` (for example because a string or block comment inside the module holds a column-0 `}` line)
- **THEN** the stamp records the whole file, so any edit to that file rebuilds the dependent C++

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

### Requirement: Development builds compile the Rust library incrementally

An optimized development configure — `BUILD_TESTING` on, a single-config
generator, and `CMAKE_BUILD_TYPE` Release or RelWithDebInfo — SHALL compile the
application's Rust library with a development Cargo profile that enables
incremental compilation. An environment that sets `CARGO_INCREMENTAL=0`, as CI
does, MAY override the incremental setting. That profile SHALL keep the release
profile's runtime semantics: the same overflow-check and debug-assertion
settings, so the development application behaves like the shipped one. A
configure with `BUILD_TESTING` off SHALL compile the Rust library with the
release profile, and a Debug configure SHALL keep the `dev` profile it uses
today. Any other configuration SHALL keep Corrosion's default profile mapping.

#### Scenario: a development configure uses the incremental profile [bp_dev_profile]

- **WHEN** the project is configured locally with `BUILD_TESTING` on, a single-config generator, and the default RelWithDebInfo build type, without `CARGO_INCREMENTAL` set, and the Rust library is built
- **THEN** cargo is invoked with the development profile and incremental compilation is enabled for the workspace crates

#### Scenario: packaging keeps the release profile [bp_release_profile]

- **WHEN** the project is configured with `BUILD_TESTING` off
- **THEN** cargo is invoked with the release profile

#### Scenario: a Debug configure keeps the debuggable profile [bp_debug_profile]

- **WHEN** the project is configured with `BUILD_TESTING` on and the Debug build type
- **THEN** cargo is invoked with the `dev` profile, not the development profile

#### Scenario: the development profile keeps release semantics [bp_dev_profile_semantics]

- **WHEN** the development profile's resolved settings are compared with the release profile's
- **THEN** overflow checks and debug assertions are set the same way in both
