# Design

## Context

See proposal.md (Why) for the motivation. These facts about the current tree shaped the
design:

- `cxx_qt_import_crate` (cxx-qt-cmake 0.10.0) parses only `CXX_QT_EXPORT_DIR`, `QMAKE` and
  `QT_MODULES`, and forwards every other argument to `corrosion_import_crate`
  (`CxxQt.cmake:22-24`), so `PROFILE` reaches Corrosion 0.5.2 without a fork or a wrapper.
- Corrosion turns `PROFILE` into `--profile=<name>` for every configuration and builds into
  `<target>/<profile>`. Without it, non-Debug configurations get `--release` and Debug gets
  the `dev` profile (`Corrosion.cmake:711-752`). `CXX_QT_EXPORT_DIR` (`build/cxxqt/`) and the
  initializer object do not depend on the profile, and no script or workflow hard-codes a
  `cargo/build/*/release` path.
- Corrosion builds into `${CMAKE_BINARY_DIR}/cargo/build`, not the workspace `target/`. The
  CMake build and `cargo nextest` therefore never share Rust build output, whatever the
  profiles say.
- In Corrosion 0.5.2, `CRATE_TYPES` filters which of the manifest's crate types are
  imported. It does not override them, so `crate-type = ["staticlib", "rlib"]` must stay in
  `crates/pictura-app/Cargo.toml` for Corrosion to find the staticlib.
- The workspace defines no `[profile.release]`, so release is Cargo's default: opt-level 3,
  16 codegen units, no LTO, unwinding panics.
- The 77 bridge files hold 28,027 lines. Their `#[cxx_qt::bridge]` modules hold 6,610 of
  them; the rest is implementation the generated C++ does not depend on.
- cxx-qt-build 0.10.0 generates C++ from the bridge module alone. Items outside it are passed
  through to cxx-gen, which ignores everything but `#[cxx::bridge]` modules, and no bridge
  file contains one. The one exception is the module's outer attributes: cxx-gen attaches
  every outer attribute and doc comment of the module, including those written above
  `#[cxx_qt::bridge]` (`cxx-gen 0.7.202, src/bridge/file.rs`). No bridge file has one today.
- `crates/pictura-app/build.rs` lists the bridge files explicitly in `.files([...])`. That
  list and the files CMake detects are the same 77.
- `cargo fmt --check` gates every pull request. A rustfmt-formatted top-level `mod` ends at a
  `}` in column 0. rustfmt does not touch string contents, so a multi-line string or raw
  string inside the module can still hold a column-0 `}` line. No bridge module does today.
- The main checkout's `build/CMakeCache.txt` carries `CMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld`,
  left from a configure that ran before the mold preference (archived change
  `2026-10-21-speed-up-cmake-and-verify`, task 3.2). The probe treats it as an explicit
  choice, so `build.ninja` links with lld while `/usr/bin/mold` exists. The probe is correct;
  the cache is stale. `scripts/verify-full.sh` already reconfigures with
  `-U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS`, and `DEVELOPING.md` ("Speeding up
  local builds") documents the same fix, so only a plain `cmake` reconfigure keeps the stale
  flag.
- CI sets `CARGO_INCREMENTAL: 0` for every job, which overrides a profile's `incremental`
  setting. sccache does not cache incremental crates, so CI keeps it off.

## Goals / Non-Goals

**Goals:**

- One-line edit in a non-bridge app file: a cargo step well under the 13.2 s `release`
  costs. The bridge-file cargo step (about 55 s to 59 s) is out of reach: the cxx-qt build
  script regenerates and recompiles all 77 bridges on any bridge-file edit, whatever the
  profile.
- An implementation-only edit in a bridge file recompiles no C++.
- CI compiles the workspace in `lint`, `test` and `qt-headless`, and `pictura-render` with
  its dependencies in `gpu-parity`, with no job that exists only to run tests again.

**Non-Goals:**

- Sharing Rust build output between the CMake build and nextest (ruled out by Corrosion's
  target directory).
- Changes to the Qt Test fixture, PCH, or C++ optimisation flags (#1043).
- Restructuring bridge files into thin adapters.

## Decisions

### D1. A `dev-app` profile that inherits `release`

```toml
[profile.dev-app]
inherits = "release"
incremental = true
codegen-units = 256
debug = "line-tables-only"

[profile.dev-app.package.pictura_app]
opt-level = 2
```

`CMakeLists.txt` passes `PROFILE dev-app` to `cxx_qt_import_crate` when `BUILD_TESTING` is on,
the generator is single-config, and `CMAKE_BUILD_TYPE` matches `^(Release|RelWithDebInfo)$`.
That is the same "optimized development configuration" the existing heavy-unit `-O1` rule
uses (`CMakeLists.txt:841`). Every other configuration (Debug, MinSizeRel, a multi-config
generator, `BUILD_TESTING` off) passes no `PROFILE` and keeps Corrosion's default mapping.

- **Why `inherits = "release"` and not `dev`:** `dev` turns on overflow checks and debug
  assertions. A panic inside a cxx-qt call aborts the process, so arithmetic that wraps
  silently in the shipped build would crash the development app and the Qt Test suites.
  That would be a behaviour change, not a speed change. Inheriting `release` keeps the
  runtime semantics, and the speed comes from incremental compilation and codegen units.
- **Alternative: plain `dev` with profile sharing** (the plan sketched in exploration).
  Rejected: Corrosion's separate target directory means no sharing happens, and the
  semantic change above.
- **Alternative: the issue's `dev-app` with dependencies at opt-level 2.** Dependencies
  rarely rebuild and stay cached in `build/cargo`, so their opt-level does not affect the
  edit loop. Keep release's 3.
- **`pictura_app` at opt-level 2** (measured; the starting value was 1): a one-line edit
  recompiles in about 5 s at both 1 and 2, so 2 costs nothing in the edit loop. It keeps
  the app optimised like the test profile's opt-level 2; runtime was not compared between
  the two. Engine crates keep release's 3.
- **`debug = "line-tables-only"`:** the default configure is RelWithDebInfo, whose C++ carries
  debug info, and release builds of the staticlib give panic backtraces no line numbers.
  Line tables are the cheapest debug info that fixes both. It costs compile and link time,
  so task 8.4 measures it against `debug = false` and drops it if it costs more than it is
  worth.
- **The profile ships only if it pays.** It is kept only if a one-line app edit measurably
  shortens the cargo step against `release`. `codegen-units` and the app opt-level are
  starting values. Task 8.4 measures them with `cargo build --timings`, and the Measurements
  section records the result. If no setting pays, task 8.5 removes the profile and
  everything that names it.
- **Incremental in CI:** CI's `CARGO_INCREMENTAL: 0` disables it in `qt-headless`. That is
  intended, since sccache cannot cache incremental crates; the profile's other settings
  still apply there.

### D2. The bridge stamp hashes the bridge module, checked, or the whole file

For each bridge file, the configure step:

1. Finds the bridge attribute by searching for `#[cxx_qt::bridge` without the closing `]`, so
   `#[cxx_qt::bridge(namespace = "...")]` is found too. If it is not found, the file is
   hashed whole.
2. Moves the start back over the contiguous lines directly above the attribute that begin
   with `#[` or `///`, so outer attributes and doc comments that cxx-gen attaches to the
   module are hashed. This is the primary mechanism for those attributes.
3. Takes the end at the first `\n}\n` after the attribute. If there is none, the file is
   hashed whole.
4. Checks the extracted text: its `{` and `}` counts must be equal, its count of `"`
   (after dropping `\\` and `\"`) must be even, and it must not contain `/*`. If any check
   fails, the file is hashed whole.
5. Appends `<sha256 of the text> <relative path>` to the digest. The path stays in the
   digest because the generated header's name comes from it.

Why three checks: a column-0 `}` line inside a string or a block comment ends the slice
early. The brace count alone does not catch that, because that `}` can balance the module's
opening `{`. A cut inside an ordinary string leaves its opening quote unpaired, so the quote
count catches it. A cut inside a block comment leaves balanced braces and even quotes, so a
slice holding `/*` is rejected outright; no bridge module uses a block comment today. A cut
inside a raw string whose content holds an odd number of quotes still passes all three
checks (see Risks). Braces or quotes inside strings and comments can make a correct slice
fail a check. That falls back to the whole file, so it rebuilds too much, never too little.
All 77 bridge modules pass the checks today.

Second guard: a Rust unit test in `pictura_app` (`bridge_modules_end_at_column_zero`) checks
every file that contains `cxx_qt::bridge`:

- the bridge attribute is `#[cxx_qt::bridge` at column 0, exactly once;
- the first `\n}\n` after it equals the end of the module found by brace counting that skips
  comments and string and char literals;
- the nearest line above the attribute that is neither blank nor a plain `//` comment is not
  an outer attribute, an outer doc comment, or the tail of a multi-line attribute. Inner
  attributes (`#![`) and `//!` lines belong to the file and are allowed. Step 2 cannot see
  an attribute separated by a blank line or a comment, or one that spans lines; the test
  also rejects the contiguous forms step 2 does hash, so the module never carries an outer
  attribute at all. No file has one today.

A formatting change that would make the CMake shortcut wrong therefore fails nextest.

- **Alternative: brace counting in CMake.** Rejected. CMake has no character-level parser,
  `;` in Rust source breaks CMake list handling, and a character loop over 28k lines at
  configure time is slow. The count-and-parity check above is two regex matches per file.
- **Alternative: thin adapter modules.** Rejected: it moves about 21k lines for the same
  outcome.
- **Alternative: declare the generated headers as cargo-step byproducts and rely on Ninja
  `restat`.** Rejected. It is the more principled fix, but it depends on Corrosion internals
  and was not needed once the stamp is narrow.

### D3. Reconfigure only for bridge files, `build.rs`, and new files

- The bridge files are the union of the files carrying the bridge marker and the `src/*.rs`
  paths in `build.rs`'s `.files([...])` list (read with `file(STRINGS ... REGEX)`), sorted.
  Today both sets are the same 77 files. A listed file without a bridge is hashed whole.
- `CMAKE_CONFIGURE_DEPENDS` lists those files and `crates/pictura-app/build.rs`.
- The source glob uses `CONFIGURE_DEPENDS`, so adding or removing a `.rs` file still re-runs
  the configure step.
- A file that gains a bridge without any file being added is caught either way round. If
  the bridge is written first, adding the file to `build.rs`'s list (needed to generate
  anything) reconfigures and the scan finds it. If the file is listed first, it is already a
  configure dependency, so the edit that adds the bridge reconfigures. Without this, the
  file would stay out of both the stamp and the configure dependencies, and every later
  edit to its bridge would need a second build.
- The existing literal glob-pattern entries (`crates/pictura-app/src/*.rs`) are removed: they
  name files that do not exist. Task 0.1 records, from the main checkout's existing build and
  without building, whether they make every build reconfigure.
- An implementation edit in a bridge file still re-runs the configure step, because the file
  is a configure dependency. It no longer rebuilds C++, but the configure time stays in the
  edit loop, so task 8.6 measures it.

### D4. Doctests are disabled, not just unrun

Each workspace library sets `[lib] doctest = false`. The doctest steps are removed from
`scripts/test-report.sh`, `scripts/report_tests.py` and `ci.yml`. If the steps were only
dropped, a later doc example would compile into nothing any gate runs. The unit test that
checks this reads the workspace `members` list, so a member outside `crates/` is covered.

### D5. Oracle tools move into the `test` job

The `test` job installs, before nextest runs:

- ImageMagick: the same AppImage. The downloaded file is cached with `actions/cache` under
  `~/.cache/imagemagick`, keyed on its version, and extracted to `/opt/imagemagick` on every
  run. Caching `/opt` directly is avoided because the cache restores as the runner user.
- `psd-tools`.
- `ag-psd`, at a pinned version, with `npm i --no-save --prefix "$RUNNER_TEMP/agpsd"`, and
  `NODE_PATH` pointing at its `node_modules`. `agpsd_oracle.rs` already honours `NODE_PATH`,
  and the checkout gets no `package.json` or `node_modules`.

The `oracles` job is deleted. Its `if:` was its only gating; the `changes` job's classifier
never named it. The `pictura_app` exclusion the job used goes away, because `test` already
installs Qt, and `pictura_app` has no oracle tests.

The `test` job now runs the oracle tests itself instead of beside a parallel job, so its
wall time can grow even though CI compiles less. Task 8.9 records it against the job's
`timeout-minutes: 30`.

### D6. The `software-vulkan` clippy moves into `lint`

`lint` already has the toolchain and the Vulkan headers. The feature is `software-vulkan = []`,
so it changes no dependency's features, and the feature's clippy reuses the workspace
clippy's check output for every other crate. `gpu-parity` keeps only its test run.

### D7. The mold warning sits next to the probe, once per build tree

When the cached EXE or SHARED linker flags already carry `-fuse-ld=` on an ELF host, the probe
still checks mold. The flag it considers is the first `-fuse-ld=` other than mold in either
set, so `-fuse-ld=mold` for executables does not hide `-fuse-ld=lld` for the shared shell.
If mold is found and accepted, configure prints a `WARNING` that names that flag and the
fix,
`cmake -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS <build-dir>`. That is the same
`-U` form `scripts/verify-full.sh` uses and `DEVELOPING.md` documents.

The warning prints once per build tree and flag value. Configure stores the flag it warned
about in an internal cache entry, `PICTURA_LINKER_MASK_WARNED`, and stays quiet while the
cached flag still equals it. A deliberate `-fuse-ld=lld` therefore warns on the first
configure only. The explicit-choice rule is unchanged.

### Declined items from #1042

- **`cargo nextest archive` + `--partition` across runners.** The suite runs in 34 s, and
  the per-runner setup (toolchain, Qt, cache restore) costs more than it saves.
- **Aligning `profile.test` with `dev`.** Nothing shares its output (see Context), so the
  alignment buys nothing.
- **Dropping `rlib`, or building the staticlib only via `CRATE_TYPES`.** Not possible with
  Corrosion 0.5.2 (see Context). `pictura_app` has no integration tests, binaries, examples
  or benches, so nextest does not need the staticlib. The doctest build is the likely
  producer of the 817 MB of archives under `target/debug/deps`. Task 8.8 checks that the
  archive no longer appears once doctests are gone, and the Measurements section records it.

## Risks / Trade-offs

- **`dev-app` makes the app slower in Qt Tests.** The app crate compiles at opt-level 2
  instead of 3, with 256 codegen units. Qt Test layer wall time is compared with the
  baseline in Measurements; a regression over 5% would raise the app back to release's 3.
- **The bridge slice ends early.** A column-0 `}` line inside a string or block comment in
  the bridge module cuts the slice short, so edits after the cut would not change the stamp.
  The quote-parity check catches a cut in an ordinary string, and the `/*` check rejects any
  slice with a block comment; both hash the whole file. They miss a cut inside a raw string
  whose own content holds an odd number of quotes. The unit test catches that case, but only
  when nextest runs: a developer who only runs `cmake --build` would get an under-rebuild
  until CI fails. No bridge module has a multi-line string today.
- **An outer attribute the walk-back cannot see** (separated by a blank line or a plain
  comment, or spanning several lines) would change the header without changing the stamp. The unit test rejects
  both forms, with the same nextest-only caveat.
- **A bridge-module edit that does not change the generated header** (a comment inside the
  module) still changes the stamp and rebuilds the dependent C++. That over-rebuild is
  accepted.
- **The ImageMagick download fails in the `test` job.** The oracle tests would self-skip,
  hiding the failure. The install step runs `magick -version`, as the `oracles` job does
  today, so a broken install fails the job. A failing `pip` or `npm` install fails its step.
- **A consumer of the report format.** Anything that parses the doctest line or passes
  `--doctests` to `report_tests.py` breaks. Nothing in the repository does after this change.
  No required status check names `oracles` (verified: `master` has no branch protection, and
  the only ruleset is disabled).

## Migration Plan

- Local: an existing `build/` picks up `dev-app` on its next configure, and Corrosion writes
  to a fresh `build/cargo/build/.../dev-app` directory, so the first build is cold. The
  generated headers come out identical, and the build script keeps their timestamps, but
  the stamp now hashes extracted module text instead of whole files, so the first configure
  changes `pictura_bridge.stamp` and rebuilds the dependent C++ (about 325 objects) once. A
  tree with a cached `-fuse-ld=lld` sees the warning once.
- CI: the `oracles` check disappears from pull requests.
- Rollback is reverting the commit. No cache or artifact format changes.

## Measurements

Filled in by tasks 0.1 and 8.x. Baseline numbers come from #1042 (master @ 1f730219) and the
main checkout's existing build, without building in this worktree before the code is written.

### Baseline (task 0.1)

| Gate | Baseline | Source |
|---|---|---|
| Cargo step, one-line app edit | 58.7 s (`--release`, non-incremental) | #1042 |
| C++ rebuilt by any bridge-file edit | 331 TUs, about 5000 CPU-s | #1042 |
| No-change build reconfigures | No: `ninja -n -d explain` starts at the cargo edge, with no CMake re-run edge | main checkout `build/` |
| Qt Test layer wall time (`ctest -R '^tst_' -j 4`) | 257.2 s, 200 tests passed | main checkout `build/` (built 2026-10-29 17:19, after 1f730219) |
| `libpictura_app-*.a` under `target/debug/deps` | 8 archives, 347–818 MB each (the 817 MB one from #1042 among them) | main checkout `target/` |
| CI compiles of the workspace per PR | 5 (`lint`, `test` + doctests, `qt-headless`, `oracles` + doctests; `gpu-parity` for `pictura-render`) | #1042, `ci.yml` |
| CI job wall times | `test` 8m05s (Test 6m06s, Doctests 3s), `oracles` 5m42s (Test 5m02s), `lint` 4m11s (Clippy 2m39s), `gpu-parity` 2m36s (clippy 18s, GPU parity 1m41s), `qt-headless` 9m47s (Build 4m27s, CTest 3m16s) | master run 40892417046 |

### After the change (tasks 8.x)

Measured on the same machine as the main-checkout baseline, building with 12 of 16 CPUs. The
cargo step is timed alone (`ninja _cargo-build_pictura_app`) for a one-line body edit.

| Gate | Before | After |
|---|---|---|
| Cargo step, edit in a non-bridge app file (`history.rs`) | 13.2 s (`release`, measured in a `-DBUILD_TESTING=OFF` dir) | 5.3 s (`dev-app`) |
| Cargo step, body edit in a bridge file (`paint_tools.rs`) | 56.6 s (`release`); #1042: 58.7 s | about 55 s (`dev-app`) |
| C++ objects recompiled by a body edit in a bridge file | 325: the whole-file stamp changed, so the same set as a module edit (#1042: 331 TUs, about 5000 CPU-s) | 0 |
| C++ objects recompiled by a new function in a bridge module | 325 | 325 in the first build, 0 in the next (same-build guarantee holds) |
| Whole build for a body edit in a bridge file | — | 66–68 s: the cargo step, a 1–2 s reconfigure, and relinks |
| Reconfigure on a non-bridge `.rs` edit | yes (every `.rs` was a configure dependency) | no (measured) |
| Reconfigure on adding a `.rs` file, or on a `build.rs` edit | yes | yes, by construction (the `CONFIGURE_DEPENDS` glob; `build.rs` is a configure dependency); not timed |
| Reconfigure on a no-change build | no | no |
| Qt Test layer wall time (`ctest -R '^tst_' -j 4`) | 257.2 s | 231.5 s (200 passed): no regression from `dev-app`. The two numbers come from different build trees, so the difference is not a speed-up claim |
| `libpictura_app-*.a` written by a nextest run | yes (the doctest build) | no: the `pictura_app` nextest build wrote no archive or rlib |
| CI workspace compiles per PR | 5 | 3 (`lint`, `test`, `qt-headless`), plus `pictura-render` in `gpu-parity` |

Tuning (task 8.4), one-line non-bridge edit, `dev-app` with the app crate's settings varied:

| `pictura_app` setting | Cargo step |
|---|---|
| opt-level 1, 256 CGUs, line tables (starting value) | 5.0–5.8 s |
| opt-level 2 | 4.9–5.7 s (kept) |
| opt-level 1, `debug = false` | 5.1–5.5 s (no gain; line tables kept) |
| opt-level 1, 16 CGUs | 8.3–9.5 s |
| opt-level 0 | 3.0 s (rejected without a runtime measurement, to keep the app optimised like the test profile's opt-level 2) |

What the numbers say:

- `dev-app` pays for ordinary app edits: 13.2 s to 5.3 s. It keeps release semantics and
  shows no Qt Test regression, so it ships.
- A body edit in a bridge file no longer recompiles any C++. But its cargo step stays at
  about 55 s under either profile. That cost is the cxx-qt build script: a change to any
  bridge file reruns `build.rs`, which regenerates and recompiles the C++ for all 77 bridges.
  No profile setting reaches it. Moving implementation out of bridge files (#1042's
  thin-adapter idea, declined here) or splitting the build script's work is what would cut
  it; that is follow-up work.
- The cxx-qt initializer object (`initializers.o`) is rebuilt by every build, including a
  no-change one; that predates this change and is not a shell translation unit.
- The CI job wall times after the change come from the pull request's run (task 9.2).
