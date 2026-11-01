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
- CMake >= 3.24, Ninja, and the `lld` linker (C++ links use `mold` instead when
  the compiler accepts `-fuse-ld=mold`).
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
cmake -S . -B build -G Ninja
cmake --build build --parallel
./build/pictura
```

The app is a Rust `staticlib` (`pictura_app`, built by Corrosion from Cargo)
linked into the C++ `pictura` binary. cxx-qt codegen is driven by
`crates/pictura-app/build.rs` and `src/cxxqt_object.rs`. The engine crates stay
Qt-free; only `pictura-app` touches Qt.

Development builds (`BUILD_TESTING` on, the default) compile `pictura_shell` as
a shared library, so the Qt Test suites link one small library instead of two
static archives. Release and packaging builds pass `-DBUILD_TESTING=OFF`, keep
the static shell, and are the only installable form; `cmake --install` on a
development build is not supported.

`CMakeLists.txt` lists every `.cpp`/`.h` explicitly, so a new source file must be
added there. Each QObject header's matching `.cpp` ends with
`#include "moc_<name>.cpp"` so a header edit recompiles that translation unit
instead of a central moc unit. The CMake configure also links
`compile_commands.json` into the repo root for clangd/Serena C++ navigation; if
C++ cross-file references look fuzzy, reconfigure.

### What the build reacts to

The cxx-qt build script (`crates/pictura-app/build.rs`) watches only what it
consumes: the bridge Rust sources that define the generated C++ interface and
the three hand-written bridge edges under `cpp/`. A shell translation unit, a
test source or a document does not rerun it, and the generated headers keep
their timestamps when their content is unchanged.

A change to a bridge source is picked up in **one** build. At configure time
CMake hashes the bridge sources into `build/pictura_bridge.stamp` and makes the
translation units that include a generated header depend on it, so the C++ is
recompiled against the regenerated header in the same build that regenerates
it. The stamp is rewritten only when the bridge content changes, so a
re-configure does not dirty the whole C++ tree.

No script and no build system exports a `QMAKE` of its own, so the value the
cxx-qt build script's fingerprint compares does not depend on which tool
started the build. A bare `cargo build` has no `QMAKE` and the build script
finds qmake on the PATH; the CMake build's cargo invocation gets the path the
cxx-qt module chose (`Qt::qmake`). The value has to be a path:
`qt-build-utils` runs qmake with a cleared environment, where a bare name is
not found on the PATH. An explicit `QMAKE` in the environment is honoured and
reruns the build script, as it should:

```bash
QMAKE=/path/to/qmake cargo build -p pictura_app
```

## Architecture and data flow

The engine is the source of truth for pixels and document structure; the app is
a thin Qt shell around it.

- **Bridge.** `crates/pictura-app/build.rs` plus `src/cxxqt_object.rs` drive
  cxx-qt codegen. The Rust-2018 root keeps the `#[cxx_qt::bridge]` and shared
  helpers; concern submodules live under `src/cxxqt_object/`
  (`impl_{core,layers,selection,transform,paint,history,filters}.rs`, and so
  on). Keep the root-plus-directory layout — the generated header path and the
  C++ includes depend on it. A bridge file holds declarations only: the bodies
  of the functions it declares, their helpers and tests live in its child
  module `<name>/imp.rs`, so editing them does not rerun the cxx-qt build
  script (guarded by `build_config::bridge_files_hold_declarations_only`).
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

The change list is **zero-open**: a change is authored, implemented, and archived
in the pull request that ships it, so `openspec/changes/` holds only `archive/`
on `master`. `scripts/check-openspec-archived.sh` fails a pull request that leaves
a non-archived change; it runs in the guards workflow (including on the release
pull request when `RELEASE_PLEASE_TOKEN` is set) and in `scripts/verify-fast.sh`.

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

bash scripts/verify-fast.sh                      # fmt, clippy, test-report, guards, openspec
bash scripts/verify-full.sh                      # CMake build first, then verify-fast
openspec validate --all --strict
bash scripts/check-openspec-archived.sh          # zero-open change gate
```

Oracle suites self-skip when `magick` / `psd-tools` are absent. Profiling and
GPU tests are `#[ignore]`d with a reason string and run with
`--ignored --nocapture`. A test that needs a Vulkan device is
`#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]` and fails when it is run without one,
so the report counts it as skipped, never as passed. The `gpu` nextest profile
selects those tests and the device-free GPU checks. CI's `gpu-parity` job runs
them on Mesa's lavapipe. The test-only `software-vulkan` feature lets the probe
accept a software device, which release builds reject:

```bash
# any Vulkan GPU:
cargo nextest run -p pictura-render --profile gpu --run-ignored all
# no GPU (install mesa-vulkan-drivers):
cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all
```

See
[`docs/dev/testing-conventions.md`](docs/dev/testing-conventions.md) for how
each layer is built and reports.

`scripts/test-report.sh` folds the layers into one report. The two
application self-test invocations run concurrently with each other and with
the CTest layer, and a suite that CTest registered but that produced no report
is a failure, named in the report — a suite that never started is not a
silently missing line.

`scripts/check-build-tree.sh` compares the build tree with the sources under
it: the full gate fails when the application binary predates a tracked source
or when a registered suite has no executable, and the fast gate reports the
same staleness. Run `cmake --build build` before the fast gate when it warns.

The slow Qt Test suites are registered in several entries each
(`crates/pictura-app/cpp/tests/CMakeLists.txt`): an entry runs a share of the
suite's test functions, chosen by the `PICTURA_TEST_SHARD` environment
variable, so the layer's wall time is set by the parallel level rather than by
one suite. A suite may share one window across its test functions and stay
sharded when its per-function reset (`init()` and `cleanup()`) makes every
function independent; a suite whose functions build on each other's state
keeps one entry.
`CTEST_PARALLEL_LEVEL` overrides the level, which `scripts/test-report.sh`
sets to 80% of the CPUs (at least one) and CI to every CPU; plain `ctest` runs
serially unless given `-j`.

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
offscreen-compatible. `--gpu-probe dx12|vulkan` is the hidden child mode the
Windows launcher uses to test one backend without risking the parent; it is not
for interactive use.

## Profiling

`scripts/profile-tools.py` launches the built app in control mode and times
tool gestures and layer creation end to end (`--scenarios new-layer,new-group`
for the latter). `--save` and `--compare` give a before/after table.

```bash
python3 scripts/profile-tools.py --size 6000x4000 --scenarios new-layer,new-group --metrics
```

`--metrics` starts the app with `PICTURA_METRICS=1`. With that set, the app:

- prints one `pictura metrics: edit label=… composited=… wall_ms=…` line per
  committed edit, plus a `pictura metrics: phase …` line for each Qt-side
  phase such as `layers_panel_refresh`;
- answers the control server's `metrics` method with phase totals, GPU counters
  (resident hits and misses, declines by reason) and the recent edit records;
  `reset: true` clears them;
- requests device timestamps from the GPU when the device supports them, so
  GPU composites also report `gpu_ms`.

Without the variable it prints nothing and keeps no edit records or phase
totals, and output is unchanged. (The render crate's GPU counters always count:
one atomic add per composite or upload. They are only reported.)
`PICTURA_PAINT_TIMING=1` still prints the per-stroke `[paint-timing]` report.

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
- **Keep Help > Search complete.** The Shift+Space search bar rebuilds its
  index from the live app each time it opens, so new menu commands, tools
  (the engine tool catalogue, `pictura_script::tool_catalogue`), and panels with a Window > Panels toggle (`panelToggles()`)
  are found automatically. A new Preferences page goes in the list in
  `buildSearchIndex()` (`search_index.cpp`), and a panel or options-bar control
  worth finding goes in `searchControls()` (`search_controls.cpp`). See
  `docs/02-ui-ux/search-bar.md`.
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
cmake -S . -B build -G Ninja
cmake --build build --parallel
```

Heavy local verification can be serialized across worktrees. It is **off by
default**: set `PICTURA_VERIFY_LOCK=1` in your shell or the user-global env file
(see [Speeding up local builds](#speeding-up-local-builds)) to make
`scripts/verify-fast.sh` and `scripts/verify-full.sh` queue behind one another
instead of running their CMake, cargo, Qt link, and test steps at the same time.
`PICTURA_VERIFY_LOCK_TIMEOUT` bounds the wait in seconds (default 3600; `0`
waits without a limit). A docs-only `verify-fast.sh` run never takes the lock.
The lock lives under `XDG_RUNTIME_DIR` (or `TMPDIR`), so runs that use different
runtime directories do not serialize against each other.

A new worktree benefits from `sccache` (see
[Speeding up local builds](#speeding-up-local-builds)) and needs a one-time
`serena project index` if you use Serena there. Remove a finished worktree with:

```bash
git worktree remove ../kooka-pictura-worktrees/{issue}-{name}
```

The shared `../.target` survives removal; reclaim it like any `target/` with
`cargo clean` or `cargo sweep`.

## Speeding up local builds

The C++ build has three automatic fast paths. On ELF platforms, C++ links use
`mold` when the compiler accepts `-fuse-ld=mold`, then `lld`, then the compiler
default; macOS and Windows keep the native linker. A build directory that cached
an explicit `-DCMAKE_EXE_LINKER_FLAGS` needs `-U CMAKE_EXE_LINKER_FLAGS` (or a
fresh directory) to re-probe; `verify-full.sh` does that for you, so force a
choice through the `CMAKE_EXE_LINKER_FLAGS` environment variable instead of a
cached `-D` value. Development builds link `pictura_shell` as a shared library.
An optimized development build (Release or RelWithDebInfo) compiles the Rust
app with the `dev-app` profile: it inherits `release`, so runtime behaviour
matches the shipped app, but compiles incrementally with more codegen units.
A Debug configure uses `dev`; release and packaging keep the static shell and
`--release`. When a build tree's cached linker flags hide an installed mold,
configure warns once with the `-U` command above.

The same optimized development build compiles C++ at `-Og -g1` and, with GCC or
Clang, behind a precompiled header of the Qt and std headers
(`crates/pictura-app/cpp/pictura_pch.h`). C++ profiling therefore wants a
packaging configure (`-DBUILD_TESTING=OFF`). The header can hide a missing
`#include`; configure a throwaway tree with
`-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON` to check, as CI does.

`scripts/test-report.sh` runs the CTest suites on 80% of the CPUs; set
`CTEST_PARALLEL_LEVEL` to override it. The
local build-caching check reuses a user-level FetchContent checkout cache under
`${XDG_CACHE_HOME:-$HOME/.cache}/kooka-pictura/fetchcontent`, so repeated
configures do not re-clone cxx-qt-cmake or Corrosion. In its default mode the
check configures the project once; `--full` adds the two other launcher cases
and is what CI runs.

`sccache` is optional. When it is on `PATH`, a plain
`cmake -S . -B build` configures it as the C++ compiler launcher
automatically; an explicit `-DCMAKE_CXX_COMPILER_LAUNCHER=...` still wins. The
launcher applies on a fresh configure, so delete `build/` to pick it up. To route
Rust compilation through it too, export the wrapper before building or testing:

```bash
export RUSTC_WRAPPER=sccache
```

sccache does not cache incrementally compiled crates, so with `dev-app` the
workspace crates in the CMake build pass through it uncached; dependencies are
still cached.

`sccache` persists its cache between sessions: on Linux `~/.cache/sccache`
(macOS `~/Library/Caches/Mozilla.sccache`, Windows
`%LOCALAPPDATA%\Mozilla\sccache`), capped at 10 GiB by default. Override the
location and cap with `SCCACHE_DIR` and `SCCACHE_CACHE_SIZE` (for example
`SCCACHE_CACHE_SIZE=20G`); `sccache --show-stats` reports both plus the hit rate.
If the cache server is unreachable, `SCCACHE_IGNORE_SERVER_IO_ERROR=1` makes the
build fall back to the compiler instead of failing.

Rebuilding the same tree from a different worktree path misses the cache
(keys use absolute paths). To share C++ objects across worktrees, opt in once
per machine by creating the user-global env file from the committed example:

```bash
mkdir -p ~/.config/kooka-pictura
cp .env.example ~/.config/kooka-pictura/env
# then uncomment KOOKA_SCCACHE_SHARE=1 in that file
```

`.env.example` lists every environment variable the project uses and is a no-op
as shipped. The real file lives outside the repository and is never committed —
repo-local `.env` files are gitignored — so the opt-in cannot be forced on other
contributors.

With `KOOKA_SCCACHE_SHARE=1` set (in that file or the process environment),
`.githooks/post-checkout` writes an sccache config — `$SCCACHE_CONF` if set, else
Linux `${XDG_CONFIG_HOME:-$HOME/.config}/sccache/config` and macOS
`~/Library/Application Support/Mozilla.sccache/config` — that lists every
worktree root as a `basedirs` entry and disables `hash_working_directory`, so an
object built in one worktree is reused in a sibling. The list refreshes when a
worktree is added (a removed worktree leaves a stale entry until then, which is
harmless), and the daemon restarts only when the file changes. The hook touches
only a file whose first line is its repo-specific managed-header comment; your
own sccache config, or another clone's, is left alone and sharing simply does
not engage. Opt out by deleting that file and running `sccache --stop-server`.

Two things are machine-global, not per-repo: the config file (so
`hash_working_directory = false` applies to every project using sccache on this
account) and the daemon restart (adding a worktree can restart the server a
concurrent build is using — export `SCCACHE_IGNORE_SERVER_IO_ERROR=1` so such
builds fall back to the compiler instead of failing). Only identical source and
compiler flags hit — a worktree on a different branch still rebuilds — and Rust
is unaffected because worktrees already share `../.target`.

`target/` grows to tens of GB. Reclaim space with `cargo sweep` (install
`cargo-sweep` if you want it) or a plain `cargo clean`.

`scripts/verify-fast.sh` skips fmt, clippy, and tests when every changed path is
`docs/`, `openspec/`, a `*.md`, or under `.serena/`, running only the guard and
spec validation. Otherwise its independent script checks run concurrently and
their logs are reported together. `scripts/verify-full.sh` proves the sccache
wiring by rebuilding one probe object, not by deleting the build tree.

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
   release, and calls the packaging workflows for the tag (`appimage.yml`,
   `linux-packages.yml`, `snap.yml`, `flatpak.yml`, `windows.yml`, `macos.yml`);
   they attach the AppImage, `.deb`/`.rpm`, `.snap`, `.flatpak`, Windows
   installer/portable archive, and macOS DMG to the release.

### Linux packages

Both packaging workflows start from the same bundle: the composite action
`.github/actions/linux-bundle` builds on `ubuntu-24.04` (so every package needs
glibc 2.39+), stages `cmake --install` into an AppDir, and deploys Qt 6.11 and
its library dependencies into it with linuxdeploy and its Qt plugin. Qt is
bundled because no distro Qt is new enough everywhere (Ubuntu 24.04 ships 6.4).
The install rules, the desktop file (`packaging/pictura.desktop`), and the icon
all use the binary's name, `pictura`.

- **AppImage** (`appimage.yml`): packs the AppDir, runs `--headless --self-test`
  against the AppImage inside a clean `ubuntu:24.04` container (so the runner's
  Qt cannot hide a missing bundled library), and uploads
  `KookaPictura-X.Y.Z-x86_64.AppImage`.
- **deb / rpm** (`linux-packages.yml`): `packaging/build-packages.sh` moves the
  AppDir to `/opt/kooka-pictura` (the `$ORIGIN` rpath and `qt.conf` keep working),
  links `/usr/bin/pictura`, and installs the desktop entry and icon under
  `/usr/share`. Dependencies are the host libraries the bundle does not carry:
  `dpkg-shlibdeps` computes the deb's `Depends`, and the rpm's ELF scan runs with
  the bundled sonames excluded from `Requires`/`Provides`. Each package is
  installed into a clean `ubuntu:24.04` / `fedora:latest` container and
  self-tested before `KookaPictura_X.Y.Z_amd64.deb` and
  `KookaPictura-X.Y.Z-1.x86_64.rpm` are uploaded. Both packages install the
  license texts under `/usr/share/doc/kooka-pictura/` and the native AppStream
  metainfo (`packaging/pictura.metainfo.xml`) for `pictura.desktop`.

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
the installer with `makensis` and `packaging/windows/installer.nsi`. It then
smoke-tests the shipped forms, not only the staging folder: it extracts the
portable archive and runs `--headless --self-test`, silent-installs `setup.exe`
into a temporary directory and self-tests there, then uninstalls and asserts the
directory is gone. The installer writes an install marker and refuses a
non-empty, unmarked directory; the uninstaller removes files only when that
marker is present.

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
(Qt 6.11's floor) and is **unsigned** — there is no Apple Developer account, so
it is not notarized either. Gatekeeper blocks the first launch; macOS 15 removed
the old right-click → Open shortcut. The user has to open the app once, then go
to **System Settings → Privacy & Security → Open Anyway** (or run
`xattr -d com.apple.quarantine /Applications/Kooka\ Pictura.app`). Signing and
notarization are tracked as follow-up #704.

The shared composite action `.github/actions/macos-bundle` builds the
`MACOSX_BUNDLE` target `pictura.app` (`Info.plist` from
`packaging/macos/Info.plist.in`, icon `assets/icons/app.icns`), then runs
`macdeployqt -always-overwrite` to embed the Qt frameworks and plugins into
`Contents/` (the flag also rewrites the pre-copied offscreen plugin's framework
paths), then ad-hoc signs the bundle. The workflow packs the DMG with the pinned
`create-dmg`; `--skip-jenkins` skips the Finder AppleScript, which needs an
interactive session. It then mounts the DMG, runs `--headless --self-test` from
the bundle on the volume with `DYLD_*` and Qt plugin variables unset, and checks
`qt.conf` and bundle-relative `otool -L` output. Little CMS is linked
statically on macOS
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

### Snap packages

`.github/workflows/snap.yml` builds a strictly-confined snap for x86_64 from a
release tag and attaches it to the release as
`kooka-pictura_X.Y.Z_amd64.snap`. The snap is **unsigned** — install it with
`snap install --dangerous ./kooka-pictura_X.Y.Z_amd64.snap`. Without the flag
snapd stops with "cannot find signatures". The upload job adds this command to
the release notes, once per release; `README.md`, `public_docs/installing.md`,
and the site's Download panel carry it too.

The snap is built from source inside snapcraft (`snap/snapcraft.yaml`), not from
the AppDir the Linux packages share: the `kde-neon-6` extension supplies Qt from
its content snap and plugs, so bundling the linuxdeploy Qt stack would mix two
copies. The build installs the pinned Rust toolchain (`rust-toolchain.toml`) it
needs, and the application reuses `packaging/pictura.desktop` and
`assets/icons/app.png`.

- **Snap Store is not wired yet.** Publishing to the store (credentials, name
  registration, channels) is a future change; for now the `.snap` is a release
  asset only.
- **Re-run for an existing tag**: `gh workflow run snap.yml -f tag=vX.Y.Z`,
  also available from *Actions → Run workflow*; `--clobber` replaces an existing
  asset. As for the other workflows, `--ref <branch>` runs the branch's workflow
  file against the tag's tree.
- **Dry run** — the release-please Release PR runs the same build and keeps the
  `.snap` as a workflow artifact instead of uploading it; feature PRs never run
  it.

### Flatpak packages

`.github/workflows/flatpak.yml` builds the x86_64 bundle
`KookaPictura-X.Y.Z-x86_64.flatpak` from the manifest
`flatpak/io.github.KookaStudio.KookaPictura.yml`. The application id is
`io.github.KookaStudio.KookaPictura`; the runtime is `org.kde.Platform` /
`org.kde.Sdk` 6.11, and x86_64 is the only architecture.

The build runs on the KDE SDK. Before compiling, `flatpak-builder` fetches the
declared sources: the Rust crates from the committed
`flatpak/cargo-sources.json` and the `cxx-qt-cmake`/`corrosion` archives are
downloaded over the network, and the repository tree is copied locally. CMake
and Cargo then run offline against those vendored trees.
Regenerate the cargo sources with
`flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json`
(`flatpak/flatpak-builder-tools`) whenever `Cargo.lock` changes, and commit the
result; `scripts/check-flatpak-cargo-sources.py` fails the lint job when a locked
crate is missing. The Rust toolchain comes from the freedesktop `rust-stable`
SDK extension, not from `rust-toolchain.toml`.

The bundle does **not** embed its runtime: `org.kde.Platform//6.11` comes from
Flathub on the installing machine, so the user needs Flatpak installed and the
Flathub remote configured. A cheap `flatpak-builder-lint` job validates the
manifest and metainfo on pull requests that change them.

- **Re-run for an existing tag**: `gh workflow run flatpak.yml -f tag=vX.Y.Z`,
  also available from *Actions → Run workflow*; `--clobber` replaces the existing
  asset. As for the other workflows, `--ref <branch>` runs the branch's workflow
  file against the tag's tree.
- **Dry run** — the release-please Release PR runs the same build and keeps the
  bundle as a workflow artifact instead of uploading it.
- **Not yet wired**: Flathub submission (#708). The metainfo's `<release>`
  version is updated by release-please; its date and release notes are not.
- **Portal confinement is not proven**: the smoke test proves the sandbox
  launches the app, not that portal-backed file dialogs work. That part of the
  confinement story remains #647.

## Where to go next

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — provenance, asset, and dependency rules.
- [`docs/README.md`](docs/README.md) — how to read the spec corpus.
- [`ROADMAP.md`](ROADMAP.md) — what ships, what is planned, what is not.
- [`docs/dev/STATE.md`](docs/dev/STATE.md) — resume anchor: where things are.
- [`docs/dev/psd-support-roadmap.md`](docs/dev/psd-support-roadmap.md) — the
  detailed PSD/PSB gap ledger.
