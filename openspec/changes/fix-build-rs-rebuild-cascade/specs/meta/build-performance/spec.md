# Spec Delta

## MODIFIED Requirements

### Requirement: QMAKE is stable across CMake and direct Cargo invocations

Every cargo invocation — the one the CMake build drives, a bare `cargo build`,
`cargo clippy`, `cargo nextest`, and rust-analyzer's flycheck — SHALL observe
the same `QMAKE` value, so the cxx-qt build script's
`rerun-if-env-changed=QMAKE` fingerprint does not change between them. The
value SHALL come from one source of truth that does not depend on which tool
started the build; the local verification scripts SHALL NOT export a `QMAKE` of
their own, and the helper script that did SHALL be removed. The value SHALL
name a qmake that exists, so the cxx-qt build script can locate Qt with it.
Selecting a genuinely different qmake SHALL still rerun the build script.

#### Scenario: alternating gates do not rerun the build script [bp_qmake_stable]

- **WHEN** a CMake build tree exists and `verify-fast.sh` runs Cargo directly after `verify-full.sh`
- **THEN** both cargo invocations observe the same `QMAKE` value and the cxx-qt build script does not rerun solely because `QMAKE` changed

#### Scenario: a different qmake still reruns [bp_qmake_change]

- **WHEN** `QMAKE` names a different qmake installation than the previous run
- **THEN** the cxx-qt build script reruns

## ADDED Requirements

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
