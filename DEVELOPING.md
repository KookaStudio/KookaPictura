# Developing Kooka Pictura

An onboarding guide to the codebase, its build, its tests, and its workflow.
Before your first change, read [`CONTRIBUTING.md`](CONTRIBUTING.md): the
provenance and licensing rules there are not optional.

## Orientation

Kooka Pictura is two halves stitched together:

- **Engine crates** (`crates/pictura-*`) — the spec math: document model, codecs,
  color, adjustments, filters, selection, ops, paint, rendering, test
  utilities. They are **Qt-free** and testable headless. `pictura-core` has no
  dependencies.
- **The app** (`crates/pictura-app`) — the only Qt crate. A Rust `staticlib`
  exposes a cxx-qt `PictureView` QObject and links into the C++/Qt `pictura`
  binary. All UI lives in its `cpp/` tree.

```
crates/        10 engine crates + the app
  core|codec|color|adjust|filters|ops|paint|select   spec math, no Qt
  pictura-render/   CPU compositing + GPU (wgpu/Vulkan) compute
  pictura-testkit/  golden-image compare / hashing
  pictura-app/      the only Qt crate: Rust cxx-qt bridge + C++/Qt shell
docs/          spec corpus + docs/dev/ working notes
openspec/      change proposals (changes/) + capability specs (specs/)
scripts/       verification gates + Python oracle tools
```

| Crate | Responsibility |
|---|---|
| `pictura-core` | Document/Layer/Channel/Mask/BlendMode/AdjustmentData; no dependencies |
| `pictura-codec` | PSD/PSB read/write: composite and layer channels, masks, adjustment keys, document channels, image resources and metadata |
| `pictura-color` | ICC profiles (sRGB/AdobeRGB/ProPhoto), convert/assign, intents, black-point compensation |
| `pictura-adjust` | Destructive adjustments and the native-depth kernels |
| `pictura-filters` | Blur, sharpen, noise, stylize, pixelate, distort, render, and the artistic families |
| `pictura-select` | Selection coverage, boolean/modify ops, wand, color range |
| `pictura-ops` | Image resize, canvas size, rotate/flip, arbitrary rotation |
| `pictura-render` | CPU compositor, GPU compositor/filter path, document and layer ops |
| `pictura-testkit` | Golden-image compare/hash and the `pictura-diff` CLI |
| `pictura-paint` | Brush/pencil dab-splatting stroke engine |
| `pictura-app` | cxx-qt `PictureView`, Qt C++ shell, panels, tools, bridge |

The documentation corpus has three layers:

- [`docs/`](docs/README.md) — the long-form behavioral contract: what the
  application is supposed to do. **Read-only unless the task allows docs changes**
  (a `docs/` commit needs `TASK-ALLOWS-DOCS`).
- [`docs/dev/STATE.md`](docs/dev/STATE.md) — the resume anchor: where things
  are right now. Read it first, but treat its counts as a snapshot.
- [`docs/dev/testing-conventions.md`](docs/dev/testing-conventions.md) — how
  every test layer is built and reports; the source of truth for test mechanics.

Where to start: read this file, then `docs/dev/STATE.md`, then the crate that
owns the behavior you are changing.

## Prerequisites

- Rust 1.98, pinned by `rust-toolchain.toml`.
- Qt 6 with the Core, Gui, Widgets, Svg, and Network modules, plus the matching
  `qmake6` (`qt6-base-dev` on Debian/Ubuntu).
- Qt 6 GUI **private** headers, found as the `Qt6::GuiPrivate` component
  (`interop.cpp` includes `<rhi/qrhi.h>`). On Debian/Ubuntu that is
  `qt6-base-private-dev`; the official Qt binaries (and CI's
  `install-qt-action`) already ship them. Configuration fails naming the missing
  package when they are absent.
- CMake >= 3.24, Ninja, and the `lld` linker.
- Little CMS 2 development headers (`liblcms2-dev`). The C++ binary links
  `-llcms2` because a Rust `staticlib` does not propagate its native link
  dependencies.
- Optional, for the oracle tests: ImageMagick 7 (`magick`) and Python
  `psd-tools>=1.19`. Suites self-skip when these are absent.
- Optional, for the `--interop-probe` diagnostic: Vulkan development headers
  (`libvulkan-dev`). The app and its wgpu GPU path build and run without them;
  the probe compiles to a stub when `vulkan.h` is missing.
- **OpenSpec CLI, pinned to 1.13.2** — required, not optional. It is the spec
  workflow for every non-trivial change and runs entirely from the terminal;
  see [OpenSpec — the required spec workflow](#openspec--the-required-spec-workflow).
- Optional, for symbol navigation and LLM-assisted work: the Serena MCP server
  on `PATH`. See [AI-assisted development](#ai-assisted-development).

## Build and run

```bash
cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld
cmake --build build --parallel
./build/pictura
```

The app is a Rust `staticlib` (`pictura_app`, built by Corrosion from Cargo)
linked into the C++ `pictura` binary. cxx-qt codegen is driven by
`crates/pictura-app/build.rs` and `src/cxxqt_object.rs`. The engine crates stay
Qt-free; only `pictura-app` touches Qt.

`CMakeLists.txt` lists every `.cpp`/`.h` explicitly, so a new source file must be
added there. The CMake configure also links `compile_commands.json` into the
repo root for clangd/Serena C++ navigation; if C++ cross-file references look
fuzzy, reconfigure.

## Architecture and data flow

The engine is the source of truth for pixels and document structure; the app is
a thin Qt shell around it.

- **Bridge.** `crates/pictura-app/build.rs` plus `src/cxxqt_object.rs` drive
  cxx-qt codegen. The Rust-2018 root keeps the `#[cxx_qt::bridge]` and shared
  helpers; concern submodules live under `src/cxxqt_object/`
  (`impl_{core,layers,selection,transform,paint,history,filters}.rs`, and so
  on). Keep the root-plus-directory layout — the generated header path and the
  C++ includes depend on it.
- **Boundary.** Qt decodes images to packed RGBA8888 at the app edge
  (`cpp/decode_image.cpp`); the engine never sees a Qt type.
- **Shell.** `cpp/main.cpp` is startup only: it selects the offscreen QPA plugin
  before `QApplication` for `--headless` (which implies `--self-test` when no
  document is given), then calls `runSelfTest`. Large classes are split across
  several translation units registered in `CMakeLists.txt` (for example
  `frame.cpp` → `frame_{columns,session,menus,build,test}.cpp`).
- **Rendering.** The CPU compositor is the reference; the wgpu/Vulkan path is an
  accelerator, never an oracle. GPU interop notes live in
  `crates/pictura-app/GPU-INTEROP-NOTES.md`.

A typical edit therefore touches: engine crate (math + unit test), the cxx-qt
bridge (expose it), the C++/Qt shell (UI), and `CMakeLists.txt` for any new
file.

## OpenSpec — the required spec workflow

OpenSpec turns work into reviewable change proposals before code is written,
and it is **mandatory for any non-trivial change**. The canonical capability
specs are the contract; a change is a proposal plus validated requirement deltas
plus a task list. You can drive the whole thing from the terminal — no AI agent
is needed. The `/opsx-*` commands and skills under `.opencode/` are conveniences
that wrap the same CLI.

Install the pinned CLI:

```bash
npm i -g @fission-ai/openspec@1.13.2   # >=1.7.0 required for nested specs
```

The day-to-day loop:

```bash
openspec list                                   # active changes
openspec view                                   # one-screen dashboard
openspec show <name> --diff                     # a change and its requirement diffs

openspec new change <kebab-name> --description "..."
# write proposal.md, design.md, specs/<domain>/<capability>/spec.md, tasks.md

openspec status --change <name>                 # which artifact is next
openspec instructions specs --change <name>     # how to write the next artifact
openspec validate <name> --strict
openspec validate --all --strict                # the gate verify-fast.sh runs
openspec archive <name> --yes                   # merge deltas into openspec/specs/
```

Layout and format:

- Changes stay flat under `openspec/changes/<name>/`; only the delta path inside
  a change nests, mirroring the main spec it targets.
- Capabilities live in the two-level tree
  `openspec/specs/{domain}/{capability}/spec.md`. A capability's id is its path
  relative to `specs/` (for example `compositing/layer-compositing`).
- The domains and the mirrored delta-path rule are declared in
  `openspec/config.yaml`.
- Spec format is strict: `### Requirement:` then `#### Scenario:` (exactly four
  `#`), SHALL/MUST wording, at least one scenario per requirement.
- Prefer a new capability name over `MODIFIED` unless the requirement itself
  changes.

After a CLI upgrade, run `openspec update` to regenerate
`.opencode/skills/openspec-*` and `.opencode/commands/opsx-*`, then commit the
diff. The workflow commands are `/opsx-explore`, `/opsx-propose`, `/opsx-apply`,
`/opsx-archive`, and `/opsx-sync`.

## Test and verify

Rust tests are std `#[test]` only (no framework); nextest runs them.

```bash
cargo fmt --all                                  # CI runs --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace                    # one crate/test: -p <crate> [name]
cargo test --workspace --doc                     # doctests; nextest skips them

bash scripts/verify-fast.sh                      # fmt, clippy, test-report, guards, openspec
bash scripts/verify-full.sh                      # CMake build first, then verify-fast
openspec validate --all --strict
```

Oracle suites self-skip when `magick` / `psd-tools` are absent. Profiling and
GPU tests are `#[ignore]`d with a reason string and run with
`--ignored --nocapture`. See
[`docs/dev/testing-conventions.md`](docs/dev/testing-conventions.md) for how
each layer is built and reports.

## Headless

```bash
./build/pictura --headless --self-test [file.psd]
```

`--headless` selects the offscreen QPA plugin before `QApplication` starts and
implies `--self-test` when no document is given, so it never blocks. The first
check asserts the platform is `offscreen`.

The C++ self-test is a hand-rolled sequential oracle in
`crates/pictura-app/cpp/selftest*.cpp`. It emits one
`pictura self-test: PASS|SKIP|FAIL <suite> <name>` token to stderr per check,
closes with `SUMMARY passed=<n> failed=<n> skipped=<n>`, and uses the exit code
as the failure identity: `ST_FAIL(code)` returns its code, and codes are
append-only. Names carry no milestone. The self-test only shrinks — new GUI
checks are Qt Test cases under `crates/pictura-app/cpp/tests/`, not new
`runSelfTest()` checks.

`--interop-probe` requires a real platform Vulkan instance and is not
offscreen-compatible.

## Common tasks

- **Change behavior.** Write an OpenSpec change first
  ([above](#openspec--the-required-spec-workflow)); implement against its tasks,
  then archive it.
- **Add an engine filter or adjustment.** Add to the owning crate
  (`pictura-filters`, `pictura-adjust`, …) with an inline `#[cfg(test)]` unit
  test, then expose it through the cxx-qt bridge and the Qt shell.
- **Add a GUI check.** Add a Qt Test case under `crates/pictura-app/cpp/tests/`
  (registered in `CMakeLists.txt`). Do not add `runSelfTest()` checks.
- **Add a self-test check (shrinking path).** Put it in a `selftest_*.cpp`
  translation unit, take the next free exit code, and register the file in
  `CMakeLists.txt`. Never grow `selftest.cpp`; keep within its
  `scripts/file-size-allowlist.txt` ceiling.
- **Change `docs/`.** A docs change fails `scripts/guard.sh` unless the commit
  carries `TASK-ALLOWS-DOCS` (or you run with `TASK_ALLOWS_DOCS=1`).

## Worktrees

Work on separate issues in parallel with linked worktrees. Enable the committed
hooks once per clone:

```bash
git config core.hooksPath .githooks
```

This replaces the clone's default `.git/hooks` entirely — any personal hooks
installed there stop running, and checkouts of branches that predate
`.githooks/` run no hooks at all.

Create a worktree per issue, next to the repo rather than inside it:

```bash
git worktree add -b type/issue-number-kebab-description ../kooka-pictura-worktrees/{issue}-{name}   # branch convention: see AGENTS.md
```

The `.githooks/post-checkout` hook wires each linked worktree as it appears: it
symlinks `reference-fixtures` to this checkout's copy and points `target` at a
shared `../.target`, so worktrees under the same root reuse one cargo artifact
tree instead of copying it. Treat that tree as **one builder at a time**: cargo
serializes concurrent builds on its build-directory lock, and two worktrees
building different branches ping-pong workspace recompiles. Per-worktree on
purpose: `build/`, `compile_commands.json`, the `.serena` index, and cmake
configure — configure a fresh worktree with:

```bash
cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld
cmake --build build --parallel
```

A new worktree benefits from `sccache` (see
[Speeding up local builds](#speeding-up-local-builds)) and needs a one-time
`serena project index` if you use Serena there. Remove a finished worktree with:

```bash
git worktree remove ../kooka-pictura-worktrees/{issue}-{name}
```

The shared `../.target` survives removal; reclaim it like any `target/` with
`cargo clean` or `cargo sweep`.

## Speeding up local builds

`sccache` is optional. To route Rust compilation through it, export the wrapper
before building or testing:

```bash
export RUSTC_WRAPPER=sccache
```

`scripts/verify-full.sh` adds `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` to the
CMake configure automatically when `sccache` is on `PATH`. It applies only on a
fresh configure, so delete `build/` to pick it up.

`target/` grows to tens of GB. Reclaim space with `cargo sweep` (install
`cargo-sweep` if you want it) or a plain `cargo clean`.

`scripts/verify-fast.sh` skips fmt, clippy, and tests when every changed path is
`docs/`, `openspec/`, a `*.md`, or under `.serena/`, running only the guard and
spec validation.

## AI-assisted development

The repository is set up for LLM coding agents; a human contributor can use the
same tooling. AI use is optional — see the project's
[`A note on AI`](README.md#a-note-on-ai) — and every contribution is
human-reviewed. The spec workflow itself is
[OpenSpec](#openspec--the-required-spec-workflow), which is CLI-first and does
not need an agent.

### Serena

Serena is a symbol-level MCP server. Its project config and memories are
committed (`.serena/project.yml`, `.serena/memories/`); `cache/`,
`project.local.yml`, and `compile_commands.json` are ignored. The MCP server is
declared in `opencode.json`, so it needs the `serena` executable on `PATH`.
After a fresh clone, index the project once:

```bash
serena project index
```

Serena updates the index itself as files change. C++ cross-file navigation reads
`compile_commands.json` from the repo root, which the CMake configure links from
`build/`; reconfigure if it is missing.

### How the pieces divide

- `AGENTS.md` — the agent rules (verification, git, non-goals); `CLAUDE.md` just
  imports it.
- `.opencode/` — the OpenSpec skills and slash commands (wrappers over the CLI).
- `.serena/memories/` — durable project knowledge; update it when architecture
  or conventions change.
- `docs/` — the long-form contract, read-only unless the task allows it.

## Conventions

- **Definition of done.** Non-trivial logic (a branch, loop, parser, or
  money/security path) ships with one runnable check: a unit test, a `demo()`
  self-check, or an integration test. Trivial one-liners need none.
- **File size.** Target under 800 LOC; hard cap 1200 for code and 1400 for
  tests. A file over its cap must be listed in
  `scripts/file-size-allowlist.txt`, a ceiling that only shrinks.
- **No unrequested abstractions, no new dependencies** without a stated reason.
- **Milestones (`mNN`) live only in comments, docs, and specs** — never in
  identifiers, string literals, or test names
  (`scripts/check-milestone-names.py`).
- **Docs are gated.** A change under `docs/` fails `scripts/guard.sh` unless the
  commit message carries `TASK-ALLOWS-DOCS` (or you run with
  `TASK_ALLOWS_DOCS=1`).
- **Spec workflow.** Per-change requirements live in `openspec/changes/<name>/`
  and are validated with `openspec validate --all --strict`; archiving merges
  them into `openspec/specs/`.
- **Never weaken a test to make CI pass.** A changed golden baseline needs an
  explicit note in the task result.

## Releases

Releases are automated by
[release-please](https://github.com/googleapis/release-please). Every push to
`master` runs `.github/workflows/release-please.yml`, which maintains a single
Release PR. Merging that PR writes `CHANGELOG.md`, bumps the version, and tags
the release. The first release is `v0.1.0`; its changelog covers the project
history, since there is no prior release.

Versioning follows Conventional Commits: `fix` bumps the patch, `feat` the
minor, and a breaking change (`!` or a `BREAKING CHANGE:` footer) the major. To
force an exact version, add a `Release-As: X.Y.Z` footer to a commit before the
Release PR is cut. (The `release-as` config key exists but upstream marks it
deprecated; prefer the footer.)

Three version files are updated together in the Release PR: `version.txt` (the
strategy's version file), `Cargo.toml` (`[workspace.package].version`), and
`CMakeLists.txt` (`project(... VERSION ...)`); `.release-please-manifest.json`
tracks the last released version. The Cargo and CMake versions are bumped
through the `x-release-please-version` marker on their version line, so keep
that comment when editing them.

`Cargo.lock` is not updated by the Release PR: the workspace-crate versions in
it lag until the next `cargo` command rewrites them, so a release commit is not
`--locked`-buildable until then (`scripts/third-party-licenses.py` uses
`cargo metadata --locked` and will fail first). Packaging workflows must not
assume `--locked` at the tag.

### Release token

`GITHUB_TOKEN` can open the Release PR only if *Settings → Actions → General →
Workflow permissions → Allow GitHub Actions to create and approve pull requests*
is enabled for the repository and allowed by the organization; both are set
here. It still does not start `ci.yml` or `guards.yml` on the Release PR
(GitHub suppresses runs caused by `GITHUB_TOKEN`), so that PR carries no checks.

That is fine while `master` is unprotected. If required status checks are
enabled, set a `RELEASE_PLEASE_TOKEN` secret — a fine-grained PAT with
`contents`, `pull-requests`, and `issues` write — so the PR is authored by a
real token and CI runs on it. The workflow prefers that secret automatically.
The tag points at the Release PR's merge commit, so packaging workflows key off
the release/tag event, not `master`.

### Cutting a release

1. Land work on `master` with Conventional Commit PR titles; release-please keeps
   the open `chore(master): release X.Y.Z` PR current after every push.
2. Review that PR's `CHANGELOG.md` and version bumps, then squash-merge it.
3. The `release-please` run on that merge tags `vX.Y.Z`, publishes the GitHub
   release, and calls `appimage.yml` and `linux-packages.yml` for the tag; they
   attach the AppImage, `.deb`, and `.rpm` to the release.

### Linux packages

Both packaging workflows start from the same bundle: the composite action
`.github/actions/linux-bundle` builds on `ubuntu-24.04` (so every package needs
glibc 2.39+), stages `cmake --install` into an AppDir, and deploys Qt 6.11 and
its library dependencies into it with linuxdeploy and its Qt plugin. Qt is
bundled because no distro Qt is new enough everywhere (Ubuntu 24.04 ships 6.4).
The install rules, the desktop file (`packaging/pictura.desktop`), and the icon
all use the binary's name, `pictura`.

- **AppImage** (`appimage.yml`): packs the AppDir, runs `--headless --self-test`
  against the AppImage, uploads `KookaPictura-X.Y.Z-x86_64.AppImage`.
- **deb / rpm** (`linux-packages.yml`): `packaging/build-packages.sh` moves the
  AppDir to `/opt/kooka-pictura` (the `$ORIGIN` rpath and `qt.conf` keep working),
  links `/usr/bin/pictura`, and installs the desktop entry and icon under
  `/usr/share`. Dependencies are the host libraries the bundle does not carry:
  `dpkg-shlibdeps` computes the deb's `Depends`, and the rpm's ELF scan runs with
  the bundled sonames excluded from `Requires`/`Provides`. Each package is
  installed into a clean `ubuntu:24.04` / `fedora:latest` container and
  self-tested before `KookaPictura_X.Y.Z_amd64.deb` and
  `KookaPictura-X.Y.Z-1.x86_64.rpm` are uploaded.

- **Re-run for an existing tag** (failed upload, packaging fix):
  `gh workflow run appimage.yml -f tag=vX.Y.Z` or
  `gh workflow run linux-packages.yml -f tag=vX.Y.Z`, also available from
  *Actions → Run workflow*; `--clobber` replaces existing assets. Dispatching
  with `--ref <branch>` runs that branch's workflow file but checks out the
  tag, so bundle or packaging edits only fully validate on the Release PR.
- **Dry run** — the release-please Release PR runs both builds (and the
  Windows one below) and keeps their output as workflow artifacts instead of
  uploading it. Feature PRs do not run them: a packaging change is validated
  when the Release PR next syncs after that change merges to `master`.
- **Local bundling against a distro Qt is not representative.** linuxdeploy's
  bundled `strip`/`patchelf` mishandle RELR-packed libraries (Fedora's Qt), and
  the result segfaults on plugin load; CI bundles the official Qt binaries.
  `build-packages.sh` itself runs anywhere with `dpkg-dev` / `rpmbuild`.

### Windows packages

`.github/workflows/windows.yml` builds the Windows x64 release artifacts on a
`windows-2022` runner: a portable archive
(`KookaPictura-X.Y.Z-windows-x64-portable.zip`) and an NSIS installer
(`KookaPictura-X.Y.Z-windows-x64-setup.exe`). Supported operating systems are
64-bit Windows 10 (version 1809 or later) and Windows 11; the executable
manifest generated by Qt declares both.

The shared composite action `.github/actions/windows-bundle` builds with MSVC,
stages `cmake --install` into `stage/`, and runs `windeployqt --release` over
`stage/bin/pictura.exe`, so both artifacts carry the Qt runtime and the MSVC
runtime — the portable archive runs without a preinstalled Qt or Visual C++
redistributable. The workflow packs the archive with `Compress-Archive`, builds
the installer with `makensis` and `packaging/windows/installer.nsi`, and
smoke-tests the staged binary with `--headless --self-test`.

- **Re-run for an existing tag**: `gh workflow run windows.yml -f tag=vX.Y.Z`,
  also available from *Actions → Run workflow*; `--clobber` replaces existing
  assets. As for the Linux workflows, `--ref <branch>` runs the branch's
  workflow file against the tag's tree.
- **Dry run** — the release-please Release PR runs the same build and keeps
  both artifacts as workflow artifacts instead of uploading them; feature PRs
  never run it.
- **The Windows build cannot run on Linux.** MSVC and `windeployqt` are
  Windows-only, so this job always runs on a Windows runner.

### macOS packages

`.github/workflows/macos.yml` builds the Apple Silicon DMG on a `macos-15`
runner: `KookaPictura-X.Y.Z-macOS.dmg`. The bundle targets macOS 13 or later
(Qt 6.11's floor) and is **unsigned** — until an Apple Developer account
exists, Gatekeeper requires a right-click → Open (or
`xattr -d com.apple.quarantine`) on first launch.

The shared composite action `.github/actions/macos-bundle` builds the
`MACOSX_BUNDLE` target `pictura.app` (`Info.plist` from
`packaging/macos/Info.plist.in`, icon `assets/icons/app.icns`), then runs
`macdeployqt` to embed the Qt frameworks and plugins into `Contents/`. The
workflow smoke-tests the deployed bundle with `--headless --self-test`, then
packs the DMG with `create-dmg`; `--skip-jenkins` skips the Finder AppleScript,
which needs an interactive session. Little CMS is linked statically on macOS
(`lcms2-sys`'s vendored copy, as on Windows), so the DMG needs no Homebrew
dylibs.

- **Re-run for an existing tag**: `gh workflow run macos.yml -f tag=vX.Y.Z`,
  also available from *Actions → Run workflow*; `--clobber` replaces existing
  assets. As for the other workflows, `--ref <branch>` runs the branch's
  workflow file against the tag's tree.
- **Dry run** — the release-please Release PR runs the same build and keeps the
  DMG as a workflow artifact instead of uploading it; feature PRs never run it.
- **The macOS build cannot run on Linux.** clang/arm64, `macdeployqt`, and
  `create-dmg` are macOS-only, so this job always runs on a macOS runner.

## Where to go next

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — provenance, asset, and dependency rules.
- [`docs/README.md`](docs/README.md) — how to read the spec corpus.
- [`ROADMAP.md`](ROADMAP.md) — what ships, what is planned, what is not.
- [`docs/dev/STATE.md`](docs/dev/STATE.md) — resume anchor: where things are.
- [`docs/dev/psd-support-roadmap.md`](docs/dev/psd-support-roadmap.md) — the
  detailed PSD/PSB gap ledger.
