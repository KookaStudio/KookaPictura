# Tech stack

Workspace layout/commands: `mem:core`, `mem:suggested_commands`.

## Language & toolchain

- **Rust**, edition 2021, pinned to **1.98** by `rust-toolchain.toml`
  (components: `rustfmt`, `clippy`, `llvm-tools-preview`). Workspace `rust-version = "1.98"`.
- Cargo workspace (`Cargo.toml`) = 10 engine crates + `pictura-app`. `resolver = "2"`.
- `[profile.test]` lowers deps to opt-level 0 but keeps every workspace crate at **opt-level 2**:
  pixel/oracle tests are far faster compiled than at opt-level 0. Keep this when adding crates.
- Shared workspace dependency: `thiserror = "2"` (the only one). `pictura-testkit` is the only
  dev-dependency/test utility.

## Qt / FFI

- **System Qt 6** (Debian/Ubuntu), modules `Core Gui Widgets Svg` (`find_package(Qt6 ...)`).
  `Qt6::Svg` is required for SVG icons/cursors.
- cxx-qt **0.10**, `cxx = "1.0"`, `cxx-qt-lib 0.10` with `qt_gui` (for `QImage`).
- `qmake6` is the Qt6 qmake; the bare `qmake` is Qt5 — CMake pins `qmake6` explicitly.
- CMake 3.24+; C++17; AUTOMOC/AUTORCC on. Rust is built into C++ through Corrosion via
  `cxx_qt_import_crate` (CxxQt fetched from GitHub tag `0.10.0` when not installed).

## GPU / imaging

- `wgpu 30.0.1` (`default-features = false`, features `std`, `vulkan`, `wgsl`), `pollster = "1"`,
  `ash = "0.38"` (Vulkan interop). Used by both `pictura-app` and `pictura-render`.
- `lcms2 6.2.0` binds the **system Little CMS 2.19** (color management).

## Linker / build

- `.cargo/config.toml` sets `-C link-arg=-fuse-ld=lld` for
  `x86_64-unknown-linux-gnu`; `lld` is required for Rust test binaries.
- CMake links C++ with `mold` when the compiler accepts `-fuse-ld=mold`, then
  `lld`, then the compiler default; macOS and Windows keep the native linker.
  CMake builds with Ninja.
- Development builds (`BUILD_TESTING` on) use a shared `pictura_shell`; release
  and packaging use `-DBUILD_TESTING=OFF` and the static shell.
- No script and no build system exports a `QMAKE` of its own, so the value the
  cxx-qt build script's fingerprint compares does not depend on which tool
  started the build: a bare Cargo invocation has none and the build script
  finds qmake on the PATH, the CMake build's cargo invocation gets the path
  the cxx-qt module chose (`Qt::qmake`). The value has to be a path, because
  `qt-build-utils` runs qmake with a cleared environment, where a bare name is
  not found on the PATH. An explicit `QMAKE` in the environment is honoured and
  reruns the build script.
- The cxx-qt build script watches only the bridge sources and the hand-written
  bridge edges; CMake hashes the bridge sources into `build/pictura_bridge.stamp`
  at configure time, and the translation units that include a generated header
  depend on it, so a bridge change rebuilds the C++ in the same build.

## Test/report/spec tooling

- **nextest** is the runner (`.config/nextest.toml`: fail-fast off, JUnit to
  `target/nextest/default/junit.xml`). `cargo test --workspace` is the fallback; **nextest skips
  doctests**, so `cargo test --workspace --doc` is a separate step.
- `scripts/test-report.sh` + `scripts/report_tests.py` fold JUnit + doctests + the C++ self-test
  token streams into one report. The two self-test invocations run concurrently
  with each other and with the CTest layer, and a registered suite that produced
  no report fails the run.
- `scripts/check-build-tree.sh` fails the full gate (and warns in the fast one)
  when the build tree predates the sources under it.
- **OpenSpec 1.13.2** for change proposals/specs (minimum 1.7.0 for nested spec paths). Oracles: ImageMagick 7 (`magick`), `psd-tools`.
- No test framework anywhere (std `#[test]`, hand-rolled C++ self-test, argparse Python CLIs).
