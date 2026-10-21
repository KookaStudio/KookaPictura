# Spec Delta

## MODIFIED Requirements

### Requirement: Application shell library

The C++ application SHALL be split into a `pictura_shell` library holding every
application source except `main.cpp` and the `selftest*.cpp`/`selftest*.h`
harness files, and a `pictura` executable holding `main.cpp` plus the self-test
harness that links `pictura_shell`. The split SHALL be a pure move with no
behavior change: the `pictura` executable's observable behavior, including the
`--headless --self-test` token stream and exit codes, SHALL be unchanged.
`pictura_shell` SHALL expose `crates/pictura-app/cpp` and the cxx-qt generated
include directory as PUBLIC include directories, so a target that links
`pictura_shell` can include both the hand-written and the generated headers.

`pictura_shell` SHALL be STATIC when `BUILD_TESTING` is off, and SHARED when
`BUILD_TESTING` is on and the platform is not Windows. In the STATIC
configuration the shell SHALL link `pictura_app` (the Rust staticlib),
`Qt6::Svg`, `Qt6::Network`, and lcms2 with PUBLIC visibility, and each consumer
SHALL link `pictura_assets` directly. In the SHARED configuration the shell
SHALL link `pictura_app`, the resource object, `Qt6::Svg`, `Qt6::Network`, and
lcms2 PRIVATE, SHALL publish the generated cxx-qt include directory, and each Qt
Test executable SHALL link the shared library and `Qt6::Test` (inheriting the
shell's public Qt modules) and SHALL NOT link the Rust staticlib, the cxx-qt
initializer object, or the resource object, so none of them is duplicated into
every executable. The `pictura` executable SHALL also link only
the shared library in that configuration. The C++ compiled by the cxx-qt build
script SHALL be position-independent in the SHARED configuration. The `pictura`
executable in the SHARED configuration SHALL resolve the library through the
build tree's runtime path.

The STATIC configuration SHALL remain the one used by release, packaging, and
`cmake --install`, and the resulting `pictura` binary SHALL remain
self-contained. Installing a SHARED development build SHALL NOT be supported.

#### Scenario: The executable keeps only main and the harness

- **WHEN** the shell library is built
- **THEN** `pictura_shell` contains no `main.cpp` and no `selftest*` source, and
  the `pictura` executable links it

#### Scenario: A test executable inherits the cxx-qt surface

- **WHEN** a test executable links `pictura_shell`
- **THEN** it can include the cxx-qt generated headers and link lcms2 without
  declaring either itself

#### Scenario: The self-test behavior is unchanged

- **WHEN** the relocated `pictura` binary runs `--headless --self-test`
- **THEN** it emits the same tokens and returns the same exit codes as before
  the split

#### Scenario: Development builds link one shared shell [qth_shell_shared]

- **WHEN** the project is configured with `BUILD_TESTING` on and built on Linux or macOS
- **THEN** `pictura_shell` is produced as a shared library, each Qt Test executable links it and `Qt6::Test` without the static archives, and every test suite still passes

#### Scenario: Release builds keep a static shell [qth_shell_static_release]

- **WHEN** the project is configured with `BUILD_TESTING` off and installed into a temporary prefix
- **THEN** `pictura_shell` is static, the installed `pictura` needs no separate shell library, and the installed binary passes `--headless --self-test`

#### Scenario: Windows keeps the static shell [qth_shell_windows]

- **WHEN** the project is configured with `BUILD_TESTING` on for Windows
- **THEN** `pictura_shell` stays static so no symbol-export declarations are required
