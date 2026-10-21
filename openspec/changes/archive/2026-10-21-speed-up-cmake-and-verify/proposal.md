# Proposal

## Why

The CMake build and the local verify loop cost far more than the work they do.
The issue's `build/.ninja_log` evidence (not reproduced in this worktree): 284
C++ shell objects take 1588 CPU-s, the 122 Qt Test executables spend 1157 CPU-s
linking, and every `tst_*` binary statically links the shell archive plus the
Rust staticlib, producing about 18 GB under `build/`. `verify-full.sh` then
deletes every object in that tree to prove a cache hit, so the next build pays
for the test objects a second time. CTest runs the 122 suites one at a time. The
critical path starts with a 208 s `cargo rustc --release`.

Developers run this loop many times a day and CI repeats most of it.

## What Changes

**Verify loop**

- Run CTest with a bounded parallel level instead of serially.
- Stop `verify-full.sh` from deleting every object under the real `build/`; prove
  the sccache hit by rebuilding one probe object and leave test objects alone.
- Stop paying for three fresh CMake configures per `verify-fast.sh` run; share a
  persistent `FETCHCONTENT_BASE_DIR` so cxx-qt and Corrosion are fetched once per
  run and reused on the next.
- Run the independent guard scripts in `verify-fast.sh` concurrently and fail on
  the first non-zero status after collecting all results.

**CMake and C++**

- Use `mold` for C++ links on Linux/FreeBSD when the compiler accepts
  `-fuse-ld=mold`, falling back to `lld` when it is installed and to the
  compiler's default otherwise. An explicit user linker choice wins. Cargo keeps
  `lld`; mold is not pinned in `.cargo/config.toml` because it has no automatic
  fallback, and macOS and Windows keep their native linker.
- Build `pictura_shell` as a shared library in dev/test builds (`BUILD_TESTING`
  on, not Windows). The shell links the Rust staticlib and the resource object
  privately and republishes the generated cxx-qt include directory, so each
  `tst_*` links one small library plus `Qt6::Test` instead of two static
  archives. Release and packaging keep the static shell.
- Cap concurrent link steps with a Ninja job pool.
- Compile `command_tree.cpp` (and `filter_commands.cpp` when it pays) at `-O1`
  with GCC/Clang and give the Qt Test translation units `-g1` on the same
  compilers. No split DWARF, to keep sccache hits.
- Include each header's generated `moc_<name>.cpp` at the end of its matching
  `.cpp`, leaving no moc output in `mocs_compilation.cpp`.

**Rust**

- Keep the standard `release` Cargo profile for the development build. The
  proposed lighter `app` profile was measured and rejected: its Rust edge took
  249 s against 182 s for `--release` on the same machine.
- Trim debug info for dependency and test builds where it pays.
- Make `QMAKE` consistent between CMake and direct cargo runs so the cxx-qt
  build script stops rerunning on every alternation of the two gates.
- Keep the existing `RUSTC_WRAPPER=sccache` documentation; it is already present
  in `.env.example` and `DEVELOPING.md`.

**Docs**: `DEVELOPING.md`, `AGENTS.md`, `.env.example`, and
`scripts/install-deps-fedora.sh` describe the new flags, the dev-versus-release
build shape, and the install rule (install from a static configuration).

## Capabilities

### New Capabilities

- `meta/build-performance`: how the development build selects a linker,
  per-source optimization, test debug info, link concurrency, and moc
  compilation, without changing release artifacts.

### Modified Capabilities

- `verification/qt-test-harness`: the application shell library is static in
  release but shared when `BUILD_TESTING` is on (except Windows).
- `verification/test-reporting`: the unified report runs the Qt Test layer with
  a bounded CTest parallel level.
- `verification/verification-harness`: the local full gate preserves the build
  tree, the fast gate runs its independent checks concurrently and reuses a
  FetchContent cache, and CI runs CTest in parallel with mold installed.
- `meta/local-build-caching`: the clean-rebuild proof uses one probe object
  instead of deleting every object under the build tree.

## Impact

- `CMakeLists.txt`, `crates/pictura-app/cpp/tests/CMakeLists.txt`,
  `crates/pictura-app/build.rs`, `Cargo.toml`.
- `scripts/test-report.sh`, `scripts/verify-fast.sh`,
  `scripts/verify-full.sh`, `scripts/check-local-build-caching.sh`,
  `scripts/install-deps-fedora.sh`.
- `.github/workflows/ci.yml` (mold install, bound CTest).
- About 110 `crates/pictura-app/cpp/**/*.cpp` files gain one `moc_*` include.
- `DEVELOPING.md`, `AGENTS.md`, `.env.example`.
- Packaging is unaffected: all five packagers (flatpak, snap, linux, macos,
  windows) configure with `-DBUILD_TESTING=OFF`. `cmake --install` is only
  supported from that static configuration.
- No new runtime dependency. `mold` stays optional.

**BREAKING**: none for users. A development build's `pictura` binary gains a
runtime dependency on `libpictura_shell.so`; the build tree's RUNPATH resolves
it. Installing a development build is not supported; install and package with
`-DBUILD_TESTING=OFF`.
