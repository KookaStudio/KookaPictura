# Spec Delta

## MODIFIED Requirements

### Requirement: Every C++ translation unit appears exactly once

Each `.cpp` file under `crates/pictura-app/cpp/`, excluding
`crates/pictura-app/cpp/tests/` and the three translation units the Rust build
script compiles instead of CMake
(`crates/pictura-app/cpp/decode_image.cpp`,
`crates/pictura-app/cpp/encode_image.cpp`, and
`crates/pictura-app/cpp/panels/actions_tool_recording_bridge.cpp`), SHALL appear
in exactly one fragment, and each fragment SHALL append to the variable for the
target that owns it: shell sources to `PICTURA_SHELL_SOURCES`, `main.cpp` and
`selftest*` sources to `PICTURA_SELF_TEST_SOURCES`. Headers MAY be listed but
are not required. A path in the Rust-built exclusion set SHALL NOT be listed in
a fragment, and a path outside that set SHALL NOT be excused.

#### Scenario: The fragments cover the tree [csf_complete]

- **WHEN** the `.cpp` paths in `cmake/sources/*.cmake` are compared with the `.cpp` files under `crates/pictura-app/cpp/` excluding `cpp/tests/` and the three Rust-built translation units
- **THEN** every translation unit is listed and none is missing

#### Scenario: No path is listed twice [csf_no_duplicates]

- **WHEN** the paths from all fragments are concatenated
- **THEN** no path appears more than once, the shell list holds 685 unique paths after `decode_image.cpp`, `encode_image.cpp`, and their two headers leave it, and the self-test list holds 76

#### Scenario: Each source belongs to its target [csf_target_owner]

- **WHEN** the self-test fragment is inspected
- **THEN** it contains only `main.cpp` and `selftest*` sources, and no such source appears in a shell fragment

#### Scenario: A Rust-built edge is not listed [csf_rust_built_excluded]

- **WHEN** a path in the Rust-built exclusion set appears in a fragment, or the exclusion set is widened to cover a path outside it
- **THEN** the source-list check fails and names the offending path and fragment
