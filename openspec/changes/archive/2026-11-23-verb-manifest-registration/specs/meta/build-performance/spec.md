# Spec Delta

## MODIFIED Requirements

### Requirement: The cxx-qt build script tracks only the bridge sources

The cxx-qt build script SHALL rerun only when a file it consumes changes: the
checked-in bridge-source list that names the files defining the generated C++
interface, the bridge Rust sources that list names, and the hand-written C++
headers and sources those bridges include. A change to any other file under the
crate — a shell translation unit, a test source, a document, or a Rust source
that carries no bridge module — SHALL NOT rerun the build script and SHALL NOT
cause the generated cxx-qt headers to be rewritten. The hand-written bridge edges SHALL be
tracked explicitly by the build script, because the C compiler helper it uses
emits no change-tracking directive for the sources it compiles.

#### Scenario: a non-bridge Rust source does not rerun the build script [bp_bridge_unlisted_source]

- **WHEN** a Rust source that carries no `#[cxx_qt::bridge]` module, and is not named by the checked-in bridge-source list, changes
- **THEN** the build script does not rerun and the generated headers keep their modification times

#### Scenario: a shell source does not rerun the build script [bp_bridge_tracking]

- **WHEN** a shell C++ translation unit that no bridge includes changes
- **THEN** the build script does not rerun and the generated headers keep their modification times

#### Scenario: a bridge header is tracked [bp_bridge_header_tracking]

- **WHEN** a hand-written C++ header that a bridge includes changes
- **THEN** the build script reruns and the Rust static library is rebuilt with the new header
