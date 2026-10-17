# Spec Delta

## MODIFIED Requirements

### Requirement: The application builds offline against the KDE runtime

The Flatpak build SHALL build the C++ shell and the Rust static library against
`org.kde.Platform`/`org.kde.Sdk` version 6.11. `flatpak-builder` SHALL fetch
the sources declared in the manifest before the build, and the compile phase
SHALL perform no network I/O. The Rust crates SHALL be resolved
from a committed `cargo-sources.json` with cargo in offline mode, and the
`cxx-qt-cmake` and `corrosion` dependencies that CMake would otherwise fetch
SHALL be provided as vendored sources that `FetchContent` is pointed at. The
Rust toolchain SHALL come from the freedesktop `rust-stable` SDK extension,
which satisfies the workspace's minimum Rust version.

#### Scenario: The application builds from manifest sources only

- **WHEN** `flatpak-builder` has fetched the sources declared in the manifest
- **THEN** it configures and compiles the Rust static library and the C++ shell
  from those sources without network access and produces the `pictura` binary

#### Scenario: CMake fetches nothing over the network

- **WHEN** the build configures the project
- **THEN** `FetchContent` resolves both `CxxQt` and `Corrosion` from their
  vendored source directories rather than cloning `cxx-qt-cmake` or `corrosion`
  over the network
