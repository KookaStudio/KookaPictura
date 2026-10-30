## ADDED Requirements

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

## MODIFIED Requirements

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
