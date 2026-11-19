# Design

## Context

See `proposal.md` — Why. What matters for the approach is the state of the four
configurations the build already offers:

| Configure | Cargo profile | Rust debug info | C++ debug info |
|---|---|---|---|
| default local (`RelWithDebInfo`, `BUILD_TESTING` on) | `dev-app` (inherits `release`, incremental, 256 CGU) | `line-tables-only` | `-Og -g1` + PCH |
| `Release` + `BUILD_TESTING` on | `dev-app` | `line-tables-only` | `-Og -g1` + PCH |
| `Debug` | `dev` | full | `-g` |
| packaging (`Release`, `BUILD_TESTING` off) | `--release`, no `[profile.release]` section | none for workspace crates (precompiled `std` keeps its own) | none (`-O3` only) |

- `CMakeLists.txt:38` forces `RelWithDebInfo` when no build type is given;
  `CMakeLists.txt:150-161` maps `BUILD_TESTING` + `Release|RelWithDebInfo` to
  `PROFILE dev-app` and otherwise leaves Corrosion's default mapping; the
  `dev-app` settings are `Cargo.toml:113-120`.
- The packaging actions pass `-DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF`
  (`.github/actions/{linux,macos,windows}-bundle/action.yml`), and snap/flatpak
  pass `-DCMAKE_BUILD_TYPE=Release`.
- Which packaging paths keep symbols is per-path, not general:
  - the AppImage keeps them — the bundler sets `NO_STRIP=1` for `linuxdeploy`;
  - the RPM spec keeps them — `packaging/build-packages.sh:94-96` sets
    `__os_install_post %{nil}`, which skips rpm's strip scripts;
  - Windows and macOS keep them — `windeployqt`/`macdeployqt` are not given a
    strip step in `.github/actions/{windows,macos}-bundle/action.yml`;
  - snap sets no `build-attributes: [debug]`, and the flatpak manifest sets no
    strip option, so those two packagers apply their own default stripping.

  On the first three paths the missing debug info was never emitted, not
  removed; a shipped-debug-info decision would have to cover snap and flatpak
  explicitly.
- `meta/build-performance` pins the profile mapping (`bp_release_profile`:
  packaging invokes cargo with the release profile; `bp_compile_change_release`:
  no development-only optimisation or debug flag appears in packaging compile
  commands). `meta/cpu-budget` requires the wrapper for direct heavy commands
  and pins the budget to a shared affinity slice.

## Goals / Non-Goals

**Goals:**

- One documented, copy-pasteable recipe that yields a release-mode build whose
  Rust and C++ frames both carry source-line attribution, with zero
  build-system change.
- Shipped artifacts keep their current profile settings and size.
- The documented commands keep the CPU budget where it belongs and say why.

**Non-Goals:**

- Changing `[profile.release]`, adding a Cargo profile, or adding a CMake option.
- Frame-pointer builds (`-C force-frame-pointers=yes`,
  `-fno-omit-frame-pointer`): they change codegen, so the profiled build stops
  being the shipped build. Worth an optional note, not the recipe.
- Further in-app instrumentation (#941, #944; the #651 metrics half shipped) and
  GPU timing (`PICTURA_METRICS`) — the existing Profiling section already covers
  what ships.
- A CI gate that builds and inspects a release binary: a second full release
  configure buys little for a documentation contract.
- Shipping debug info in release artifacts (a crash-reporting decision).

## Decisions

### D1. Enable Rust debug info with `CARGO_PROFILE_RELEASE_DEBUG`, not a profile

`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` (or `=2`) is applied to the build
invocation only. Cargo resolves it into the release profile without any file
change, so `bp_release_profile` still holds (cargo is invoked with the release
profile) and packaging compile commands gain no flag. The recipe also adds no
project flag and changes no build-system default: the C++ `-g` comes from the
reader's chosen `CMAKE_BUILD_TYPE=RelWithDebInfo`, not from a project-added
option, so `bp_compile_change_release` — a constraint on project-added flags —
is untouched.

Alternatives considered:

- **`[profile.release] debug = "line-tables-only"`.** One line, but it grows all
  six packaging paths and turns this into a field-crash-reporting decision.
  Deferred, not rejected: recorded in the design's open questions.
- **A dedicated `profiling` Cargo profile.** Needs `_pictura_cargo_profile`
  plumbing in `CMakeLists.txt`, a MODIFIED `meta/build-performance` requirement,
  and still needs the C++ `-g` decision below — the same outcome with more
  moving parts.
- **`debug = 2` by default.** `.debug_info` is what lets a report attribute
  inlined frames; line tables cannot. It costs link time and size, so it is the
  documented escalation (`CARGO_PROFILE_RELEASE_DEBUG=2`) when inlining matters,
  not the default.

`debug` is part of cargo's fingerprint: toggling it rebuilds the Rust library.
The recipe therefore uses a separate `build-prof` tree, which also keeps the
development tree's cached flags untouched.

### D2. `RelWithDebInfo` with `BUILD_TESTING` off for the profiling tree

- C++ needs `-g`; the packaging `Release` configure has none, and `-g` does not
  change codegen.
- `BUILD_TESTING` off keeps `PICTURA_DEV_OPTIMIZED` off, so Corrosion passes
  `--release` and the profile mapping stays exactly as `meta/build-performance`
  describes.
- The honest limitation: `RelWithDebInfo` is `-O2 -g -DNDEBUG` while packaging is
  `-O3 -DNDEBUG`, one optimization level apart. The documentation states it and
  shows `-DCMAKE_CXX_FLAGS_RELWITHDEBINFO="-O3 -DNDEBUG -g"` for readers who
  need the shipped codegen exactly; `-g` alone adds no optimization change.
- Configuring `build-prof` removes and recreates the repo-root
  `compile_commands.json` symlink so it points at
  `build-prof/compile_commands.json` (`CMakeLists.txt:27-36`); a
  `-DCMAKE_EXPORT_COMPILE_COMMANDS=OFF` does not prevent it, because line 32
  sets the variable to `ON` unconditionally. The doc therefore says to
  reconfigure `build` afterwards when clangd or Serena should keep using the
  development tree.
- Rejected: profiling the default development configure. It already has line
  tables on both sides, but it is incremental with 256 codegen units and C++ at
  `-Og -g1` behind a PCH, so it is not representative — and the PCH can hide a
  missing include.

### D3. Build under the CPU budget, record outside it

The build command the documentation pins is

```bash
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  python3 scripts/cpu-budget.py exec -- cmake --build build-prof --parallel
```

The assignment must precede the helper: after `exec --` it would be executed as a
program name. `meta/cpu-budget` calls for the wrapper on direct heavy commands,
and a full release build of the dependency graph is one. Inside the helper,
`CMAKE_BUILD_PARALLEL_LEVEL` is exported, so `--parallel` picks the budgeted
level up.

The recording must not run inside that slice: the helper restricts affinity to
the lowest budget CPUs, parallel crates size their pools from the affinity mask,
and thread placement and shared-cache topology follow. A budgeted profile
describes a smaller machine. Record on an idle machine, and compare runs on the
same machine from the same build — the convention `scripts/profile-tools.py`
already states for itself.

### D4. Verification is a shell check plus a reviewer-run profile

- Machine-checkable, no `perf` needed:
  `readelf --debug-dump=decodedline build-prof/pictura | grep -m1 'crates/pictura-render/src'`
  and the same grep for `crates/pictura-app/cpp` show line tables for both
  toolchains; `ninja -C build-prof -t commands | grep -- '--release'` observes
  the cargo invocation.
- Addresses for a line-table spot check come from
  `nm -C build-prof/pictura | grep -m1 'pictura_render'` plus one C++ symbol;
  `addr2line -f -C -e build-prof/pictura <addr>` resolves each to a source path
  and line.
- The acceptance run itself belongs to the reviewer: this worktree has no `perf`
  or `samply` and no privilege to install them. `perf_event_paranoid=2` still
  allows a one-process user-space profile; `<=1` or `CAP_PERFMON` is needed only
  to include kernel frames. CI's headless job builds with `BUILD_TESTING` on, so
  an automated version needs a second release configure and a privileged runner.
  The change puts the command and the expected report shape in the PR body for
  the reviewer to run.

### D5. Documentation lives in the existing Profiling section

A `### Sampling profiles` subsection inside `## Profiling` (`DEVELOPING.md:312`)
keeps the two workflows next to each other: the in-app harness measures a gesture
end to end, a sampler finds the hot frames. The contradictory sentence in
"Speeding up local builds" (the paragraph at `DEVELOPING.md:443-448`) is
rewritten to name the override and point at the subsection, rather than deleted,
so a reader arriving from the PCH discussion still lands on the right build.
Verification greps for `CARGO_PROFILE_RELEASE_DEBUG` and `Sampling profiles`,
not for the phrase the rewrite removes.

## Risks / Trade-offs

- **The spec's only carrier is prose.** A future build change can silently
  invalidate the recipe. → The `meta/profiling-workflow` scenarios name the
  contents, and the manual rows give the reviewer exact commands to re-run.
- **`-O2` profiling build vs `-O3` shipped build.** → Stated in the doc, with the
  flag override for exact codegen.
- **A reader forgets the override on a rebuild** and profiles a build with no
  line tables. → The doc puts the variable on the build command line and says a
  dropped variable triggers a full Rust rebuild rather than a silent partial one.
- **`compile_commands.json` moves with the profiling configure.** → The recipe
  says so and names the reconfigure fallback, since the export flag cannot
  prevent it.
- **Recording while other worktree builds run.** → The same idle-machine note the
  profile-tools harness already carries.

## Open Questions

- Should shipped artifacts carry `line-tables-only` for field panic/abort
  backtraces? Deferred: it is a release-artifact size and disclosure decision
  across the six packagers, tracked separately from #661.
