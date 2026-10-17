# Flatpak offline memo

Status: historical investigation note, written 2026-09-19 and superseded by the
shipped Flatpak manifest `flatpak/io.github.KookaStudio.KookaPictura.yml` and
the packaging sections of `DEVELOPING.md`. The findings below are kept for
context and describe the pre-manifest state.

The question: can Kooka Pictura be built as a Flatpak with no network at build
time, handed to someone with no network, and then run under Flatpak on their
machine?

Short answer: yes on all three, with conditions. Nothing here was decided and
no task was open at the time. This records what the build system and Flatpak
actually allowed, so a later proposal would not have to rediscover it.

## What was true at the time

- The repo has no Flatpak manifest, no application id, no `install()` rules, no
  desktop file, no AppStream metainfo, and no MIME definitions. `project()`
  carries no version, and there is no `--version` flag.
- `docs/01-architecture/build-and-packaging.md` already names Flatpak as the
  primary distribution target, with native `.deb` and `.rpm` alongside. Snap and
  AppImage are rejected there.
- The local machine has Flatpak 1.18.0 with the Flathub remote and
  `org.kde.Platform//6.10` installed. It has no `org.kde.Sdk`.
- Flathub publishes `org.kde.Platform//6.11` and `org.kde.Sdk//6.11`, so the
  runtime matches the project's Qt 6.11 target.

## The finding that matters most

`crates/pictura-app/cpp/interop.cpp:3` includes `<rhi/qrhi.h>`, a private Qt
API. At the time, the include path was assembled by hand from qmake's version at
`CMakeLists.txt:138`, which was fragile across distributions and was the reason
for the existing Debian comment.

The blast radius is small. `interop.h` includes no Qt header. The QRhi code
runs only from the `--interop-probe` path at `crates/pictura-app/cpp/main.cpp:126`
through `:141`. `selftest.cpp` includes `interop.h` but never calls the function.

That gives two ways to clear the private-header dependency:

1. The KDE SDK ships Qt private headers and `Qt6::GuiPrivate`. Linking that
   target fixes both Fedora and Flatpak, and no code moves.
2. The KDE SDK does not ship them. Then guard `interop.cpp` and its one call
   site behind a CMake option, and the app builds with no private Qt API. The
   probe becomes a build-time extra rather than a requirement.

Which case holds is unverified. The SDK is not installed here, and the Platform
runtime ships no headers at all, which is expected since headers belong to the
SDK. This is the first thing a feasibility check would settle.

## Offline build

`flatpak-builder` supports it directly:

- `--download-only` fetches the manifest sources and stops.
- `--disable-download` then guarantees no network I/O, and fails if a source is
  missing.
- `--extra-sources=DIR` feeds pre-fetched sources in the state-directory layout.

The catch was what counted as a source. The manifest only covers what it
declares. At the time, Kooka Pictura resolved two dependency layers over the
network during the build:

- Cargo crates from crates.io.
- `cxx-qt-cmake` 0.10.0, pulled by `FetchContent` at `CMakeLists.txt:24`.

Both had to become manifest sources. The Cargo side is the standard
`flatpak-cargo-generator.py` output committed as `cargo-sources.json`, with the
build running `cargo build --offline`. The cxx-qt side is the source vendored as
a module and the fetch redirected with `-DFETCHCONTENT_SOURCE_DIR_CXXQT=<path>`,
which is the variable name FetchContent derives from the declared target name
`CxxQt`.

One more condition sits on top. The Rust toolchain is pinned to 1.98 by
`rust-toolchain.toml` and `rust-version`. The freedesktop `rust-stable` SDK
extension installs plain `cargo`, which ignores `rust-toolchain.toml`, and its
rustc version tracks the SDK release. If it is older than 1.98 the build stops
on the version requirement. A newer extension branch, a vendored pinned
toolchain, or lowering the pin are the ways around it.

## Offline share

Flatpak documents two mechanisms and is explicit that they differ in what they
carry.

**`flatpak create-usb` is the intended one.** It copies the app and its runtime
dependencies into an OSTree repository on the drive. The target installs from it
with `flatpak install --sideload-repo=...`. Three prerequisites, all about
provenance rather than the file:

- The repositories supplying the app and every dependency must be GPG signed.
- Both sides must have a collection ID set.
- The target must already have the matching remotes configured.

The KDE runtime comes from Flathub, which is signed and carries
`org.flathub.Stable`. A self-built app needs its own repo, a collection ID, a
signing key, and a `.flatpakrepo` file to add the remote offline.

**Single-file bundles are the fallback.** The command reference is blunt:
bundles do not include dependencies. `flatpak build-bundle --runtime` exports a
runtime instead of an app, so a self-contained handover means one bundle for
`org.kde.Platform` and one for the app, installed in that order. This avoids
collection IDs and the remote dance, at the cost of a manual two-step install
and a signing question for the app bundle.

Either way the recipient still needs Flatpak installed. A Flatpak links against
its runtime for Qt, glibc, and everything else, so extracting the app and running
it standalone is not a thing. Running it without Flatpak is a native build, not
a Flatpak.

## What the app build needs before any of this

The Flatpak path runs `cmake --install` into `/app`. With no `install()` rules
the result would be empty, so install rules for the binary, desktop file,
metainfo, and icon come first. The `.desktop` file is what gives `flatpak run`
something to register and is where the PSD and PSB MIME associations live. The
manifest also wants the `finish-args` already written down at
`docs/01-architecture/build-and-packaging.md:58`: `--share=ipc`,
`--socket=wayland`, `--socket=fallback-x11`, `--device=dri`.

## Risks worth naming

- Private Qt headers in the SDK. Covered by the interop fallback, but the
  cleanest outcome is finding `Qt6::GuiPrivate` there.
- Rust 1.98 against whatever the SDK extension ships.
- Rust build caches and vendored crates add real size to the offline payload;
  the KDE runtime itself is on the order of a gigabyte on the drive.
- Oracle tests need ImageMagick and psd-tools, neither of which is in the KDE
  SDK, so they self-skip inside the sandbox. The C++ self-test does not.
- Host GPU access needs a driver and `--device=dri`; the freedesktop GL
  extension handles Nvidia. The app already has OpenGL and software fallbacks.

## Open questions

- Whether `org.kde.Sdk//6.11` ships `rhi/qrhi.h` and `Qt6::GuiPrivate`.
- Whether the rust-stable extension is at or above 1.98.
- The application id and whether it is a private id or a Flathub-shaped
  `io.github.*` id.
- Whether the eventual target is Flathub or a private air-gapped handoff. Flathub
  requires the desktop and metainfo files and forbids network during build;
  private distribution is looser on the metadata.
- Whether the `--interop-probe` diagnostic is worth keeping as a build-time
  option, or should stay a required part of every build.
