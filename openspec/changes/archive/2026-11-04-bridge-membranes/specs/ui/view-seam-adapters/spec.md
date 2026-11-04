# Spec Delta

## MODIFIED Requirements

### Requirement: Adapters are the only C++ surface over covered bridge operations

Production C++ outside `crates/pictura-app/cpp/api/` SHALL reach the Rust bridge only
through (a) adapter namespaces (`document_api`, `layers_api`, `selection_api`,
`paint_api`, `transform_api`, `history_api`, `type_api`) for bridge operations covered
by a landed migration unit, or (b) a domain membrane header under `cpp/api/` for every
other bridge operation. Generated cxx-qt headers SHALL be included only by translation
units and headers under `cpp/api/`; no deferral is permitted outside that directory.
The type adapter's own header (`cpp/api/type_api.h`) is the named exception that owns
the generated `type_tools*.cxxqt.h` headers, as the existing type-adapter requirement
requires. Adapters SHALL exist only where they absorb real complexity (string
protocols, call ordering, batching, or a unit façade that centralizes multi-call
sequences); a 1:1 forwarder of a single invokable SHALL NOT be introduced, and no
document algorithm SHALL live in an adapter.

#### Scenario: A panel includes only adapter headers

- **WHEN** a production panel, dialog, command, tool, or control-server TU needs a
  bridge operation
- **THEN** it does not include a generated `*.cxxqt.h` header

#### Scenario: Adapter registration is explicit

- **WHEN** a new adapter `.cpp`/`.h` pair or a new membrane `.h` is added under `cpp/api/`
- **THEN** it is listed explicitly in `CMakeLists.txt` and builds into `pictura_shell`

#### Scenario: Pass-through adapters are rejected at review

- **WHEN** a proposed adapter is a 1:1 forwarder of a single invokable with no
  batching, no typed protocol, no ordering guarantee, and no unit-level call-sequence
  centralization
- **THEN** the adapter is not added; callers use the domain membrane instead

#### Scenario: Uncovered operations are named deferrals

- **WHEN** a bridge operation has no landed migration unit
- **THEN** it is reached through a domain membrane header and no generated-header
  deferral is recorded outside `cpp/api/`

## ADDED Requirements

### Requirement: Generated headers are confined to cpp/api

Generated cxx-qt headers SHALL be included only by headers and translation units under
`crates/pictura-app/cpp/api/`. Every C++ file outside that directory, including
`cpp/tests/` and `frame_includes.h`, SHALL include a domain membrane or an adapter
header instead, except for the generated type bridge headers governed by the existing
type-adapter requirement. The type bridge headers remain owned by
`cpp/api/type_api.{h,cpp}`, and the existing `cpp/tests/` allowance for them is
unchanged. A generated header with no C++ consumer is exempt from the membrane mapping.
A membrane SHALL contain an include guard or `#pragma once`, includes, and comments
only: no function bodies, no state, and no document algorithm.

#### Scenario: Generated headers are confined to cpp/api

- **WHEN** the C++ tree outside `crates/pictura-app/cpp/api/` is searched for an
  include of a generated `*.cxxqt.h` header
- **THEN** there is none, except the type bridge headers governed by the existing
  type-adapter requirement

#### Scenario: A membrane carries no logic

- **WHEN** a membrane header is reviewed or parsed
- **THEN** it contains `#pragma once` (or an include guard), includes, and comments
  only, with no function bodies or state

### Requirement: Domain membranes bound the per-translation-unit include surface

Generated headers SHALL be grouped into domain membranes by consumer co-inclusion, so
that a translation unit includes the fewest membranes that cover the verbs it uses. No
membrane SHALL include every generated header, and each membrane's generated includes
SHALL be a proper subset of the set of generated headers. The global bridge stamp still
rebuilds every membrane and adapter consumer on any bridge edit; this change does not
change rebuild fan-out.

#### Scenario: No membrane is a universal header

- **WHEN** the membranes under `cpp/api/` are inspected
- **THEN** no single membrane includes all generated headers, and each membrane's
  generated includes are a proper subset of the set of generated headers

### Requirement: Bridges with two or fewer functions are folded

A standalone `#[cxx_qt::bridge]` file that exposes at least one `extern "Rust"` or
`extern "RustQt"` function and two or fewer SHALL NOT exist. Its declarations SHALL
live in a domain-neighbour bridge file, its `imp` module SHALL be re-homed (or the file
SHALL remain as a plain, non-bridge module), and the build script's bridge-file list
SHALL reflect the folded set. A bridge module that only imports `extern "C++"`
functions (no `extern "Rust"`/`extern "RustQt"` function) is exempt.

#### Scenario: The small bridges are gone

- **WHEN** the bridge files are counted and each is inspected for exposed functions
- **THEN** every bridge that exposes at least one Rust/RustQt function declares more
  than two, import-only bridges are exempt, and the bridge count is `<= 69`

#### Scenario: A folded verb keeps its call sites working

- **WHEN** a function moves from a folded bridge into its domain neighbour
- **THEN** its generated C++ symbol and signature are unchanged (or
  `decode_bridge::copy_bytes` stays unnamespaced), and the build and Qt suites pass
  without call-site edits

### Requirement: The bridge seam ratchet only shrinks

A repository check SHALL fail when a C++ file outside `cpp/api/` includes a generated
`*.cxxqt.h` (except the type bridge headers governed by the existing type-adapter
requirement), when the bridge-file count exceeds 69, or when
`crates/pictura-app/src/cxxqt_object.rs` exceeds 1,204 lines. The check SHALL run from
`scripts/verify-fast.sh`.

#### Scenario: A new direct generated include fails the gate

- **WHEN** a C++ file outside `cpp/api/` gains an include of a generated `*.cxxqt.h`
  other than the type bridge headers
- **THEN** `scripts/verify-fast.sh` fails and names the file

#### Scenario: Bridge growth fails the gate

- **WHEN** a change adds a bridge file or grows `cxxqt_object.rs` past 1,204 lines
- **THEN** `scripts/verify-fast.sh` fails
