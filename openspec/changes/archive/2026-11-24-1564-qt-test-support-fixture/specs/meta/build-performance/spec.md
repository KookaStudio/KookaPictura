# Spec Delta

## MODIFIED Requirements

### Requirement: Generated moc output compiles with its own translation unit

Each QObject header that has a same-named `.cpp` SHALL have that `.cpp` include
the generated `moc_<header>.cpp`. A QObject header with no same-named `.cpp`
SHALL have its generated moc included from one of the `.cpp` files that includes
the header. The shell target's `mocs_compilation.cpp` SHALL contain no moc
output. A change to a QObject header SHALL recompile the translation unit that
includes its moc, not a central moc compilation unit.

A repository check SHALL enforce the ownership mechanically and SHALL run from
`scripts/verify-fast.sh` and the guards workflow, so the nine headers whose moc
output currently lands in the central unit cannot regress.

#### Scenario: moc files are included by their own units [bp_moc_includes]

- **WHEN** the C++ sources are inspected after the change
- **THEN** every matching QObject `.cpp` includes its `moc_<header>.cpp`, the header without a same-named `.cpp` has its moc included from one of its consumers, and `mocs_compilation.cpp` contains no moc include

#### Scenario: a header edit stays local [bp_moc_incremental]

- **WHEN** one QObject header changes and the project is rebuilt
- **THEN** the build recompiles that header's moc-owning translation unit and does not recompile a central moc compilation unit

#### Scenario: a missing moc owner fails the check [bp_moc_guard]

- **WHEN** a QObject header's same-named `.cpp` omits its `moc_<header>.cpp` include, or a QObject header without a same-named `.cpp` has no consumer that includes its moc
- **THEN** the repository check fails and names the header and the translation unit it expected to own the moc

## ADDED Requirements

### Requirement: A hand-written bridge edge compiles in exactly one place

Each hand-written C++ source that a Rust bridge declares SHALL be compiled by
exactly one build owner, so one edit neither produces two object files nor
compiles the same source twice. Sources that the cxx-qt build script compiles
SHALL NOT also appear in a `cmake/sources/*.cmake` fragment, and a source in a
fragment SHALL NOT also be passed to the build script's C compiler. The source
list check SHALL treat every build-script-compiled edge as an excluded path
rather than a fragment omission, so an unlisted shell translation unit still
fails.

Tracking the edge in the build script SHALL remain, because the C compiler
helper emits no change-tracking directive for the sources it compiles: a change
to an edge SHALL still rerun the build script. Removing one compile SHALL NOT
remove the rerun trigger. The build script SHALL NOT scan `src/` to discover its
inputs; the checked-in bridge-source list remains the input.

#### Scenario: each edge is compiled once [bp_bridge_edge_single_owner]

- **WHEN** the build is configured and the C++ sources of every target are inspected
- **THEN** no path under `crates/pictura-app/cpp/` appears both in a `cmake/sources/*.cmake` fragment and in the build script's compiled edge list

#### Scenario: an edge edit still reruns the build script [bp_bridge_edge_tracked]

- **WHEN** a hand-written edge source the build script compiles changes
- **THEN** the build script reruns and the Rust static library is rebuilt with the new source, and the source is not additionally compiled by CMake

#### Scenario: an unlisted shell source still fails [bp_bridge_edge_exclusion_exact]

- **WHEN** the source-list check runs after a build-script-compiled edge is removed from a fragment
- **THEN** the check passes for that edge and still fails for any other `.cpp` under `crates/pictura-app/cpp/` that no fragment lists
