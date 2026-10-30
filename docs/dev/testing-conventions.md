# Test conventions

How the tests are actually built and how they report, as of 2026-09-18. This is
descriptive, not the contract: the aspirational spec is
`docs/11-cross-cutting/testing-strategy.md` (`XC-010`), which proposes a much
larger scheme (proptest, fuzz, Criterion, Qt Test/CTest, golden manifests) that
does **not** exist yet. See §9 for where the two diverge.

There is no third-party test framework anywhere. Every layer uses
language-native or toolkit-native facilities: std `#[test]` for Rust, the Qt
toolkit's own Qt Test for the C++ GUI suites.

## 1. Layers at a glance

| Layer | Built with | Lives in | Reports as |
|---|---|---|---|
| Rust unit | std `#[test]` in inline `#[cfg(test)] mod tests` | `crates/*/src/` (4–23 files/crate) | cargo/nextest |
| Rust integration / oracle | std `#[test]` in `tests/*.rs` binaries | `crates/*/tests/` | cargo/nextest |
| C++ GUI self-test | hand-rolled `runSelfTest()` + `SelfTestReport` | `crates/pictura-app/cpp/selftest.cpp`, `selftest_report.cpp` | `pictura self-test:` tokens + `SUMMARY` + exit code |
| C++ Qt shell tests | Qt Test + CTest (`add_test`) | `crates/pictura-app/cpp/tests/` | per-executable JUnit, folded into the unified report by `scripts/test-report.sh` |
| Python oracle tooling | `argparse` CLIs, no test framework | `scripts/*.py` | stdout (machine-readable or raw bytes) + exit code |

Current inventory: **1990 `#[test]`**, **8 `#[ignore]`** (all profiling/GPU tests,
see §3), **511** `ST_BEGIN` self-test sites (**475** executed in a bare
`--headless --self-test` run), and seventeen Qt Test suites (`tst_smoke`,
`tst_command_tree`, `tst_layers_panel`, `tst_align_distribute`,
`tst_edit_clipboard`, `tst_fill_tools`, `tst_retouch_tools`, `tst_pen_tools`,
`tst_path_selection_tools`, `tst_shape_tools`, `tst_rotate_view`,
`tst_save_for_web`, `tst_type_tools`, `tst_type_panels`, `tst_color_range`,
`tst_print`, `tst_image_adjustments`) run under CTest.
`pictura-testkit` is the only dev-dependency; there are no
test-runner crates outside Qt Test.

`.config/nextest.toml` makes nextest the local and CI runner: fail-fast off, every
status streamed, JUnit written to `target/nextest/default/junit.xml`.
`scripts/test-report.sh` folds that XML, the self-test token
streams, and the per-executable Qt JUnit (`build/qt-test-results/*.xml`) into one
pytest/vitest-like report through `scripts/report_tests.py` (§6); CI uploads the
JUnit file as an artifact.

## 2. Rust tests

- **Unit tests** are inline modules: `#[cfg(test)] mod tests` in the same file as
  the code. Standard `#[test]` + `assert!`/`assert_eq!`. Test names are
  snake_case phrases describing the guarantee
  (`tolerance_boundary_passes_at_and_fails_above`, `length_mismatch_is_an_error`).
- **Integration tests** are files under `crates/<crate>/tests/`. A single
  `oracle.rs` is the common shape; multi-module suites use a `main.rs` +
  submodules directory (`pictura-filters/tests/oracle/`,
  `pictura-render/tests/gpu_parity/`).
- **One workspace runner: nextest.**
  - `scripts/verify-fast.sh:25` routes the test step through
    `scripts/test-report.sh`, which runs `cargo nextest run --workspace
    --no-fail-fast` (writing `target/nextest/default/junit.xml`), then
    both self-test invocations, then the reporter. One nextest run per gate.
    Doctests are disabled in every crate (`doctest = false`), so no separate
    `cargo test --doc` run exists.
  - CI's `test` job runs the same nextest invocation and uploads the JUnit
    report as the `nextest-junit` artifact.
- `cargo test --workspace` stays the fallback when nextest is unavailable, but
  the gate no longer parses its `test result: ok` text; the reporter reads the
  JUnit XML instead.

## 3. `#[ignore]` policy

`#[ignore = "..."]` is for tests that cannot or should not run in the default
suite — currently manual profiling (`4000x4000` region/undo, `1024x1024` filter
profile) and GPU-mandatory tests (`crates/pictura-render/src/gpu/mod.rs`). Each
carries a reason string naming how to run it (`--ignored --nocapture`).

A test that needs a Vulkan device is ignored with
`"requires a Vulkan device; run with --profile gpu --run-ignored all"` and calls
`require_gpu()`, so it fails, rather than passing silently, when it runs without
a device. The nextest `gpu` profile (`.config/nextest.toml`) selects these tests
and the device-free GPU checks, and excludes the `PICTURA_GPU_BENCH` timing
evidence. CI's `gpu-parity` job runs that profile on Mesa's lavapipe with the
test-only `software-vulkan` feature.

`#[ignore]` is **not** used to hide unimplemented behavior. The oracle suites
that used to depend on stubs (`pictura-select`, `pictura-render`) un-ignore once
the task lands.

## 4. Oracle test conventions

The oracles are *differential*: our implementation is compared against a second,
independent implementation. Crates with oracles: `adjust`, `codec`, `color`,
`filters`, `ops`, `render`, `select`. Each has a `tests/README.md` documenting
the tool mapping table and per-class tolerances.

Shared rules:

1. **Self-skip, never fail-to-run.** If the external tool is missing, print
   `eprintln!("skipping: ...")` and `return`:
   ```rust
   if Command::new("magick").arg("-version").output().is_err() {
       eprintln!("skipping: `magick` not on PATH");
       return;
   }
   ```
   CI's `test` job installs ImageMagick 7, `psd-tools`, and `ag-psd` before its
   nextest run, so these run for real.
2. **Prove the harness has teeth.** Every oracle includes a test that a wrong
   output actually fails (`differential_harness_detects_perturbation`,
   `dimension_check_bites`, `reference_script_is_present`). An oracle that cannot
   fail is not evidence.
3. **Compare through `pictura-testkit::compare`** with an explicit per-class
   tolerance; never hand-roll a byte diff in the test.
4. **Tools are invoked as `Command::new("python3").arg(script())`**; the Rust
   test owns the comparison, the Python script owns the ImageMagick/psd-tools
   invocation.
5. **Tolerance honesty.** Where ImageMagick's algorithm differs from Photoshop's
   (e.g. HSL-space blend modes, morphology structuring element), the mode is
   excluded from the oracle and covered by hand-computed unit tests instead of
   being asserted with a wide tolerance. The reason is recorded in the
   `tests/README.md` table.

## 5. `pictura-testkit`

The one shared test utility (`crates/pictura-testkit/src/lib.rs`):

- `Diff { samples, differing, max_delta, sum_delta }` + `mean_delta()`,
  `is_empty()`.
- `compare(a: &[u8], b: &[u8], tolerance: u8) -> Result<Diff, String>` — equal
  length required, per-sample absolute tolerance.
- `hash_bytes(&[u8]) -> u64` — FNV-1a, for determinism checks.

`pictura-diff` is the matching CLI:
`pictura-diff [--tolerance N] <a> <b>`, nonzero exit on any mismatch, for use in
shell pipelines.

## 6. C++ self-test conventions

Invocation: `./build/pictura --headless --self-test [file.psd]`. `--headless`
selects the offscreen QPA plugin before `QApplication` and forces
`--self-test` when no document is given (`crates/pictura-app/cpp/main.cpp:110`).
The first check asserts the platform is `offscreen`.

`runSelfTest()` (`cpp/selftest.cpp`, one long sequential function) follows:

- **Output grammar — the token protocol.** The harness
  `crates/pictura-app/cpp/selftest_report.{h,cpp}` wraps every check and emits
  one flushed token on stderr:
  ```
  pictura self-test: SUITE <suite> <name>
  pictura self-test: PASS <suite> <name> [detail]
  pictura self-test: SKIP <suite> <name> <reason>
  pictura self-test: FAIL <suite> <name> <code> <message>
  pictura self-test: SUMMARY passed=<n> failed=<n> skipped=<n>
  ```
  Checks use the `ST_BEGIN` / `ST_PASS` / `ST_SKIP` / `ST_FAIL(code, ...)` /
  `ST_FINISH` macros; `ST_FAIL` prints its message and returns `code`. `SUITE` is
  emitted only when the suite changes; one `SUMMARY` closes the run. Legacy human
  `key=value` and `FAIL:` lines still appear, but only the tokens are
  machine-read — the human lines do not match the token grammar.
- **Names carry no milestone.** Check names and suites are descriptive
  (`float_close`, `compact_shade`, `platform_headless`); milestones appear only
  in comments, docs, and specs. With no `m<NN>` prefix the harness groups every
  check under the default suite `core`.
- **Exit code is the failure identity.** Each `ST_FAIL` returns its check's code;
  codes run from `2` upward, roughly one per check (currently into the `190s`,
  e.g. `float_close` = 194, `compact_shade` = 195, `move_preview_cache` = 197).
  Exit `0` = all passed.
- **The reporter consumes this layer.** `scripts/report_tests.py` parses both
  self-test invocations' token streams and merges checks by `(suite, name)`, so a
  check present in both runs counts once (the later stream's status wins).
- **The source stays within its allowlist.** Instrumentation replaced the old
  two-line `fprintf`/`fflush` sites with single `ST_PASS` calls, and migrating
  suites to Qt Test shrank it further; `selftest.cpp`'s entry in
  `scripts/file-size-allowlist.txt` is now a **6449-line** ceiling that may only
  shrink.
- **No Qt Test inside `runSelfTest()`.** The self-test runs only as an explicit
  step in `scripts/verify-full.sh` and the CI `qt-headless` job; the Qt Test
  suites are a separate layer (see below).
- **State isolation:** before running, `main.cpp` points `XDG_STATE_HOME` at a
  temporary dir, so the self-test never touches real preferences/recovery state.
- The self-test is left with dirty documents on purpose and sets a non-interactive
  unsaved-choice so headless shutdown does not open a modal prompt.

### Qt Test layer

The app C++ is split so the UI can be tested without a second `main`: a STATIC
library `pictura_shell` holds every app source except `main.cpp` and the
`selftest*.{cpp,h}` files, and the `pictura` executable keeps `main.cpp` plus the
self-test and links `pictura_shell`.

- **Layout.** `crates/pictura-app/cpp/tests/` holds `CMakeLists.txt`,
  `qt_test_support.h` (the shared `ScopedStateHome` fixture and
  `makeMainWindow`), and one `tst_*.cpp` per suite. Each is built as its own
  executable linking `pictura_shell` + `Qt6::Test`, registered with `add_test`
  under `QT_QPA_PLATFORM=offscreen` and `TIMEOUT 120`.
- **Gated on `BUILD_TESTING`.** `include(CTest)` and `find_package(Qt6
  COMPONENTS Test)` live in an `if(BUILD_TESTING)` branch, so a build can opt out.
- **Seed suites.** `tst_smoke` (the `ScopedStateHome` temp-`XDG_STATE_HOME`
  fixture, constructed before the window), `tst_command_tree` (menus/dispatch),
  `tst_layers_panel` (row controls/chrome, group nesting, and drag/drop),
  `tst_edit_clipboard` (raster copy/cut/paste/purge). The tool ports add
  `tst_fill_tools` (Gradient, Paint Bucket), `tst_retouch_tools` (Blur,
  Sharpen, Smudge, Dodge, Burn, Sponge), `tst_pen_tools` (Pen, Freeform
  Pen, Add / Delete Anchor Point, Convert Point), `tst_path_selection_tools`
  (Path Selection, Direct Selection), `tst_shape_tools` (the six shape
  tools), and `tst_type_tools` (Horizontal /
  Vertical Type and their Type Mask tools).
- **Migration rule.** New GUI checks are written as Qt Test cases; the self-test
  only shrinks. Three suites were migrated off `runSelfTest()` and their `ST_*`
  blocks deleted, retiring codes 25, 26, 110, 123, 135, 138, 139, 200, 210, 211,
  212, 213, and 529. The tool-port checks for Gradient, Paint Bucket, and Blur
  followed into `tst_fill_tools` / `tst_retouch_tools`, retiring 552, 553, and
  554. Retired exit codes are append-only and never reused. The
  mechanical guard is `scripts/check-selftest-budget.sh`: it counts `ST_BEGIN`
  sites across `crates/pictura-app/cpp/**/*.cpp` and fails when the count exceeds
  the lower-only budget in `scripts/selftest-budget.txt` (currently 511). It runs
  from `scripts/verify-fast.sh` and the guards CI workflow, in addition to the
  `selftest.cpp` ceiling in `scripts/file-size-allowlist.txt`.
- **Reports as per-executable JUnit.** `add_test` passes
  `-o <CMAKE_BINARY_DIR>/qt-test-results/<name>.xml,junitxml`, so CTest itself
  emits one JUnit file per executable; `scripts/test-report.sh` runs
  `ctest --test-dir build -R '^tst_'` and folds `build/qt-test-results/*.xml`.

## 7. Python oracle CLI conventions

`scripts/*.py` are tools, not a test suite (no pytest/unittest): `argparse`
subcommands, deterministic output, meaningful exit codes.

- `generate-fixtures.py` — authors PSD fixtures with `psd-tools` + PIL; every
  pixel/layer/offset is fixed and output is byte-identical across runs.
- `validate_output.py` — opens a PSD with `psd-tools` and prints
  `<path>: <W>x<H> mode=<MODE> depth=<N> layers=<K>` for Rust to parse.
- `im_compose.py`, `color_oracle.py`, `ops_oracle.py`, `select_oracle.py`,
  `filter_oracle.py` — wrap ImageMagick operations. Raw image interchange is
  **planar or interleaved 8-bit** (documented per script); `im_compose.py` routes
  through PNG internally because raw RGBA makes some IM operators mangle alpha.
- Subcommand shape is typically `version` / `convert|apply|compose` / `gen` /
  `check`; `check` regenerates into a temp dir and diffs, so fixture drift is a
  test failure.

Fixtures: `crates/*/tests/fixtures/` holds raw `.rgba`/`.png` and PSDs. They are
generated, deterministic, and committed; regeneration must not churn the tree.

## 8. Recipes

**Add a Rust unit test:** `#[cfg(test)] mod tests` in the file, one `#[test]`
named for the guarantee, `assert_eq!` on the smallest input that fails if the
logic breaks.

**Add an oracle test:** put the operation in the relevant `scripts/*_oracle.py`
as a subcommand, add a test in `tests/oracle.rs` that self-skips without the
tool, compares via `pictura-testkit::compare`, and add a teeth test if you
introduced a new comparison helper. Document the mapping + tolerance in
`tests/README.md`.

**Add a C++ self-test check:** append a block in `runSelfTest()` using
`ST_BEGIN("<name>")` (give it a descriptive, milestone-free name),
`ST_PASS("...")` on success, and `ST_FAIL(<unused_code>, "...")` on
failure. Take the next free code — codes are append-only so they stay stable
identifiers. Keep `selftest.cpp` within its `scripts/file-size-allowlist.txt`
ceiling. Milestones belong in comments, docs, and specs, never in names.

## 9. Divergences from `XC-010` (the spec)

`docs/11-cross-cutting/testing-strategy.md` is `Draft` and says every name in it
is a proposal. What actually shipped:

| XC-010 proposes | Reality |
|---|---|
| `proptest` property tests | none; plain `#[test]` |
| `cargo-fuzz` targets | none; no `fuzz/` crate |
| Criterion + `QBENCHMARK` perf gates | manual `#[ignore]`d profiling tests |
| Qt Test + CTest (`add_test`) | **shipped**: `pictura_shell` static lib + CTest registering `tst_smoke`, `tst_command_tree`, `tst_layers_panel`, `tst_edit_clipboard`, `tst_fill_tools`, `tst_retouch_tools`, `tst_pen_tools`, `tst_path_selection_tools`, `tst_shape_tools`, `tst_type_tools`; `runSelfTest()` remains for the rest |
| Structured unified test reporting | **shipped**: nextest JUnit + the C++ token protocol + `scripts/report_tests.py` |
| Captured-CS6 golden references | ImageMagick 7 + `psd-tools` differential oracles |
| Golden manifests, PSNR/DSSIM/ΔE2000, `xtask` | `pictura-testkit::compare` (max-abs tolerance) + `pictura-diff` only |

Treat XC-010 as direction, not current state. If a proposal there is adopted,
update this file.

## 10. Reproduction / inspection commands

`scripts/report_tests.py` prints every test by name under its package (Rust) or
suite (app self-test), in run order, with a status glyph: `✓` green passed, `✗`
red failed, `○` yellow skipped/ignored. A passing self-test check appends its
`detail` from the `PASS` token, aligned and dimmed. Each layer keeps its suite
counts, subtotal, the `TOTAL` line, and the `FAILURES` section; the process
still exits non-zero on any failure. Pass `-q`/`--quiet` (alias `--summary`) for
the old counts-only output — suites, subtotals, `TOTAL`, and `FAILURES` with no
per-test lines. `--color auto|always|never` controls ANSI: `never` emits none,
`always` emits it even when piped.

```bash
# Fast local gate (fmt, clippy, unified report, file-size, guard, openspec):
bash scripts/verify-fast.sh

# Full gate (CI-equivalent; builds the CMake app first so the self-test runs):
bash scripts/verify-full.sh

# Unified report on its own (nextest + both self-test invocations + Qt Test):
bash scripts/test-report.sh [auto|always|never]

# Reporter only, against captured logs (writes nothing):
python3 scripts/report_tests.py --junit target/nextest/default/junit.xml \
    --selftest <log> --color never

# Counts only (no per-test lines) for CI/log-tight use:
python3 scripts/report_tests.py --quiet --junit target/nextest/default/junit.xml

# The parser's runnable check:
python3 scripts/report_tests.py --self-check

# Tests the way CI runs them:
cargo nextest run --workspace         # writes target/nextest/default/junit.xml

# One oracle suite explicitly:
cargo test -p pictura-render --test document_oracle

# The C++ self-test, offscreen:
./build/pictura --headless --self-test

# The Qt Test suites via CTest (offscreen, after a CMake build; each writes
# build/qt-test-results/<name>.xml, which test-report.sh folds):
ctest --test-dir build -R '^tst_' --output-on-failure
ctest --test-dir build -R tst_layers_panel --output-on-failure

# Inventory:
grep -rc '#\[test\]' crates --include=*.rs
grep -rn  '#\[ignore' crates --include=*.rs
grep -rho 'ST_BEGIN' crates/pictura-app/cpp --include='*.cpp' | wc -l
```
