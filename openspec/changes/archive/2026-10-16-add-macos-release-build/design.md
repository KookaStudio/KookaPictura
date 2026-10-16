# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- The Linux release path is factored as a composite action
  (`.github/actions/linux-bundle`) that a thin workflow wraps: build → stage →
  deploy runtime → pack a format. The Windows path mirrors that shape; the
  macOS path should too.
- The app code is portable POSIX C++ apart from the control server, which
  already has `#ifdef _WIN32` guards (`control_server.cpp`,
  `selftest_control.cpp`); nothing in `crates/**` uses Linux-only APIs. The
  Rust workspace has no `#[cfg(target_os = "linux")]` branches.
- GPU paths are Vulkan-only (`wgpu` features `["vulkan"]` plus raw `ash`
  interop). The compositor degrades to `GpuRender::Unavailable` when no Vulkan
  adapter exists, so a Metal-less build is acceptable. The raw `ash` interop
  escape hatch (`create_interop_state`) does **not** compile on macOS: `wgpu`
  builds its Vulkan backend only on Windows/Linux/Android/FreeBSD, so
  `wgpu::hal::api::Vulkan` is absent and `pictura_app` fails to build. It must be
  `cfg`-gated to a `None` stub on Apple targets, and the `ash` dependency moved
  behind the same `cfg`.
- `pictura` is a plain `add_executable`; macOS packaging expects an `.app`
  bundle. `macdeployqt` operates on a bundle, and `create-dmg` packs one.
- `CMakeLists.txt` links Little CMS on `NOT WIN32` with
  `find_library(... REQUIRED)`. `pictura-color` depends on the `lcms2` crate
  (`lcms2-sys` 4.0.7), which links the system library dynamically when
  pkg-config finds it and otherwise compiles its vendored C statically. On
  macOS, Homebrew's `lcms2` would be found dynamically, but `macdeployqt`
  embeds only Qt, leaving the app dependent on a Homebrew dylib.
- `main.cpp` already selects the `offscreen` QPA plugin under `--headless`, so
  `pictura --headless --self-test` is a valid smoke check for the bundle.
- release-please calls the packaging workflows from `release-please.yml`
  (`release_created == 'true'`); the packaging pull-request trigger is gated to
  the release-please Release PR (see the in-flight
  `gate-packaging-dry-runs-to-release-pr` change).
- Qt 6.11's documented macOS floor is macOS 13, and the official binaries are
  universal; GitHub's `macos-14`/`macos-15` runners are Apple Silicon.

## Goals / Non-Goals

**Goals:**
- Produce `KookaPictura-<version>-macOS.dmg` for Apple Silicon from a release
  tag, with the same trigger surface as the Linux and Windows workflows.
- Make the application build and pass its headless self-test as a macOS
  application bundle with the Apple toolchain.
- Keep the Linux and Windows builds and artifacts unaffected.

**Non-Goals:**
- Code signing / notarization (no Apple Developer account; the issue marks this
  deferred).
- Intel (x86_64) or universal binaries; a Mac App Store package; Homebrew cask.
- A GUI feature pass for macOS; this change only makes the existing app build,
  bundle, and pass its headless self-test there.

## Decisions

### Qt from `jurplel/install-qt-action`, not Homebrew

Pin Qt 6.11.1 with the same action and version the Linux and Windows release
workflows use. `brew install qt@6` would resolve to whatever minor Homebrew
ships, drift under us, and diverge from the other platforms; the issue allows
either.

### `macos-15` runner, arm64 only

GitHub's `macos-14` and `macos-15` images are Apple Silicon, so the default
Rust host target is `aarch64-apple-darwin` and the C++ build is arm64. The issue
names `macos-14/15`; `macos-15` is the current image. Intel Macs and universal
binaries are out of scope (a universal build needs a second Rust target and
`lipo`, and the issue does not ask for it). The artifact name carries no
architecture, per the issue; the docs state Apple Silicon.

### A real CMake bundle with a tracked `Info.plist` template

Build the app as a `MACOSX_BUNDLE` target on `APPLE` and set
`MACOSX_BUNDLE_INFO_PLIST` to a committed `packaging/macos/Info.plist.in`. The
template substitutes the bundle identifier, names, icon file, and the version
from `PROJECT_VERSION` (which carries the `x-release-please-version` marker), so
the bundle version tracks the tag. Use the custom template rather than Qt's
`qt6_finalize_target` so the keys and the minimum-OS declaration are explicit
and not tied to a Qt CMake version.

*Alternative considered:* assemble the `.app` directory in the workflow from a
plain executable. Rejected — it moves bundle layout out of the build, needs a
second place to substitute the version, and leaves local `cmake --install`
without a bundle.

*Consequence:* on macOS the built executable is
`build/pictura.app/Contents/MacOS/pictura`. There is no macOS CI or local
workflow today, so nothing else depends on the old path.

`LSMinimumSystemVersion` is 13.0 (Qt 6.11's documented floor), and the CMake
configure passes `-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0` so the C++ build cannot
link symbols newer than the declared floor.

### Static Little CMS on macOS

Enable the `lcms2` crate's `static` feature for `cfg(target_os = "macos")` in
`crates/pictura-color/Cargo.toml` via a target-specific dependency edge, and
narrow the CMake system-lcms2 guard from `NOT WIN32` to `UNIX AND NOT APPLE`.
`lcms2-sys` then compiles its vendored copy into `pictura_app` on macOS exactly
as it already does on Windows, so the DMG carries no external dylib and a local
macOS build needs no Homebrew `lcms2`.

*Alternative considered:* keep the Homebrew dylib and bundle it with
`dylibbundler`. Rejected — an extra tool and `install_name_tool` rewrites for a
dependency the crate already vendors.

*Risk if wrong:* Cargo feature unification is per-platform; the `static` edge
must not leak to Linux. Verified with `cargo metadata` for both targets.

### Gate the Vulkan interop probe on Apple targets

`crates/pictura-app/src/gpu.rs` exports raw `VkDevice`/`VkImage` handles from a
live wgpu device through `wgpu::hal::api::Vulkan` and `ash`. `wgpu` compiles
that backend only where `wgpu-core` enables
`wgpu-core-deps-windows-linux-android` (Windows, Linux, Android, FreeBSD), so on
Apple targets the `wgpu::hal::api::Vulkan` type does not exist and the crate
fails to build. Gate `create_interop_state` with `#[cfg(not(target_vendor =
"apple"))]` and add an Apple stub that returns `None`; the C++ side already
stubs the probe out (`interop.cpp`'s `QT_CONFIG(vulkan) && __has_include(...)`
guard), so `--interop-probe` reports unavailable there and callers keep the CPU
path. Move the `ash` dependency behind the same target `cfg` so it is not
compiled on macOS.

*Alternative considered:* switch the backends to `Backends::PRIMARY` so macOS
uses Metal. Rejected for this change — the GPU compositor and stroke kernels are
only validated against the CPU oracle on Vulkan, and enabling a second backend
is a feature decision, not a packaging fix. A Metal-less build is already
accepted above.

### Commit `assets/icons/app.icns`

Generate the icon from `assets/icons/app.png` and commit it, mirroring
`app.ico` on Windows. CMake references it with `MACOSX_BUNDLE_ICON_FILE` and
`MACOSX_PACKAGE_LOCATION Resources` so it lands in `Contents/Resources`.
ImageMagick cannot write ICNS, so it is generated with a small stdlib Python
packer (PNG-backed `ic07`/`ic08`/`ic09`/`ic10` entries) and recorded in
`assets/PROVENANCE.md`.

*Alternative considered:* generate the `.icns` in CI with `iconutil` from the
PNG. Rejected — nondeterministic across Xcode versions and the icon must exist
at configure time.

### Deploy Qt with an explicit `macdeployqt` call

After the build, run `macdeployqt build/pictura.app` in the action. The repo
already deploys explicitly on Linux (linuxdeploy) and Windows (windeployqt);
the Qt CMake deploy API would couple packaging to install time.

### Pack with `create-dmg --skip-jenkins`

`create-dmg` builds the DMG with an `/Applications` drop link and the volume
icon. `--skip-jenkins` skips the Finder AppleScript (the runners have no
interactive Finder session), which only costs icon positioning inside the DMG;
no signing options are passed.

### Unsigned until an Apple Developer account exists

No signing or notarization step, no secrets: an unsigned `.app` inside an
unsigned DMG. `DEVELOPING.md` states this and that Gatekeeper requires a
right-click → Open (or `xattr -d com.apple.quarantine`) until a certificate
exists. A `ponytail:` comment in the workflow names the ceiling. This matches
the issue's explicit deferral.

### Smoke-check the deployed bundle

Run `pictura.app/Contents/MacOS/pictura --headless --self-test` after
`macdeployqt`, with `QT_PLUGIN_PATH`/`QML2_IMPORT_PATH` unset so the embedded
plugins are the ones that resolve. Assert exit 0 and
`SUMMARY passed=<n> failed=0`.

## Risks / Trade-offs

- **No macOS host to validate against locally** → The CMake bundle properties,
  `macdeployqt`, and `create-dmg` are only exercised on a macOS runner. The
  first real validation is the workflow dispatch (or the Release PR dry run);
  the change is not "done" until that run is green. This is the accepted
  limitation of the task.
- **CI cannot prove the macOS 13 floor** → The runner is macOS 15; running on
  macOS 13 is a manual verification step, as with the Windows 10 floor.
- **`--skip-jenkins` DMG cosmetics** → The DMG opens with default icon
  positions; the app and Applications link are still present. Accepted for CI.
- **Unsigned app Gatekeeper friction** → Users must right-click → Open the
  first time; documented, and lifted when a Developer ID exists.
- **macdeployqt version drift** → Qt is pinned at 6.11.1 via the shared action
  pin, so deployment behavior matches the other platforms' Qt.
- **`CMAKE_OSX_DEPLOYMENT_TARGET` vs the Rust std floor** → Rust's
  `aarch64-apple-darwin` floor (macOS 11) is lower than the declared 13.0, so
  the mixed build does not violate the declaration.

## Migration Plan

1. Land the CMake bundle, lcms2, icon, packaging, action, and workflow changes.
2. Add the macOS job to `release-please.yml`.
3. Verify on a dispatch (or the Release PR dry run), then a real release.
4. Rollback: remove the `macos` job from `release-please.yml`; the workflow and
   packaging files can stay dormant. The CMake/lcms2 changes are Apple-only and
   behavior-preserving on Linux and Windows.
