# Testing Strategy

- **Spec ID:** `XC-010`
- **Status:** `Draft`
- **Parity tier:** `Core` — this spec explains how every other parity tier is verified.
- **New in CS6:** `No` — CS6 has no user-facing testing feature. Two CS6 behaviors
  are testability contracts, though: **Background Save** (`File > Save` completes
  out of band) and **Auto-Recovery** (`Automatically Save Recovery Information`),
  both of which must be exercised by this strategy. See
  `11-cross-cutting/crash-recovery-and-autosave.md`.
- **Depends on:** `ARCH-003` performance-targets, `ARCH-008` document-model,
  `ARCH-011` file-formats, `ARCH-012` undo-history, `ARCH-004`
  build-and-packaging, `11-cross-cutting/crash-recovery-and-autosave.md`,
  `00-overview/licensing-and-provenance.md`.

> The repository contains a working implementation; this file stays the
> behavioural contract for the feature, and `docs/dev/doc-code-parity-gaps.md`
> records what shipped and what did not. Every crate, harness, and tool name
> below is a **design proposal**. Comparison thresholds marked *(proposed)* are
> engineering hypotheses to validate, not facts about CS6.

## CS6 behavior

Testing is not a Photoshop feature, so this section records the **observable
CS6 contracts** the harness must lock down rather than a UI surface:

- **Rendering is deterministic for a fixed document + operation + environment.**
  Applying the same filter, adjustment, or blend to the same document on the
  same machine yields the same pixels. This is the core assumption a golden-image
  harness relies on; it is *(inferred)* from normal graphics-pipeline behavior,
  not documented in the CS6 Help.
- **Native round-trips are lossless in structure.** A PSD saved by CS6 and
  reopened preserves layers, masks, styles, paths, ICC/EXIF/IPTC/XMP, and the
  bytes of unknown blocks. This is documented behavior of *Maximize
  Compatibility* and the PSD format (`ARCH-011`), and is the strongest available
  oracle: pixels and structure can be compared exactly, without CS6 in the loop.
- **The visual result is the contract.** For operations whose algorithm Adobe
  never published (filters, interpolation, color conversion), no source of truth
  exists except CS6's own output. Parity there is *behavioral*, verified against
  captured reference renderings, never against an assumed formula.
- **CS6 is the reference renderer, not a dependency.** Reference images are
  produced once from CS6 13.0.1 on documented hardware and stored as test assets;
  CI never runs Photoshop. Provenance and licensing are governed by
  `00-overview/licensing-and-provenance.md`.
- **Background Save and Auto-Recovery are testable events.** Save progress is
  shown in the document tab and the status bar while the UI stays responsive;
  Auto-Recovery writes a separate backup at the configured interval and reopens
  it as a "Recovered" document after a crash. These become the fault-injection
  and background-save test targets below.

## UI surface

No user-facing test UI. The diagnostics a developer uses map to build targets
and CI jobs:

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `xtask test` (proposed) | CLI | — | Runs the full tiered suite: unit, property, integration, round-trip. |
| `xtask golden` (proposed) | CLI | — | Regenerates/compares golden images; writes a diff report. |
| `xtask fuzz` (proposed) | CLI | — | Wraps `cargo fuzz run <target>` with the regression corpus. |
| `xtask bench` (proposed) | CLI | — | Runs Criterion + Qt `QBENCHMARK` and checks budgets. |
| CTest (`enable_testing`) | Build target | `ctest` | Runs the Qt Test executables under `-platform offscreen`. |
| `Write-Approval.txt` / diff artifacts | CI artifact | — | Golden failures upload expected/actual/diff PNGs. |
| `QTest` debug build | Dev tool | — | `-vs`, `-v2`, `-repeat`, `QTEST_FATAL_FAIL` for flake hunting. |

## Parameters & ranges

Tolerance and run-policy controls for the comparison harness. All values are
*(proposed)* until a CS6 reference corpus is measured.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Comparison metric | enum | `dssim` | exact, MAE, RMSE, PSNR, max-abs, DSSIM, ΔE2000 | Selected per operation class (table below). |
| `max_abs_delta` (per channel) | int LSB | `1` | `0`…`2` | Integer modes; `0` for lossless ops. |
| Outlier pixel budget | percent | `0.5` | `0`…`100` | Fraction of pixels allowed to exceed `max_abs_delta`. |
| PSNR floor | dB | `40` | `30`…`60` | Filter/resample classes. |
| DSSIM ceiling | float | `0.001` | `0`…`0.01` | Perceptual metric (`dssim` crate). |
| ΔE2000 (mean / max) | float | `1.0` / `3.0` | — | Color-management classes. |
| 32-bit relative error | float | `1e-5` | — | Per-component relative; `1e-5` on 0..1 scale. |
| 32-bit ULP budget | int | `2` | — | Secondary check near zero. |
| GPU-vs-CPU slack | int LSB | `1` | `0`…`2` | GPU paths compared to CPU reference, not CS6. |
| Property-test cases | int | `256` | `16`…`100k` | `proptest` `PROPTEST_CASES`. |
| Property-test seed | u64 | fixed per CI | any | Determinism; failing seeds are committed. |
| Fuzz wall-clock budget | seconds | `60` | CI-bounded | Per target per PR; longer on nightly. |
| Bench regression threshold | percent | `5` | `1`…`20` | Fail if p50 worsens beyond threshold run-to-run. |
| Round-trip corpus size | int | full | — | Curated PSD/PSB/TIFF/PNG/JPEG set. |
| Golden corpus size | int | full | — | Operation × mode matrix. |

## Algorithms & pipeline

### Test tiers

| Tier | Tool | Scope | Speed | Runs |
|---|---|---|---|---|
| Unit | `cargo test` | Pure functions, parsers, blend math, color transforms | ms | Every PR |
| Property | `proptest` | Round-trips, invariants, algebra | s | Every PR (fixed seed) |
| Integration | `cargo test --test` | Cross-crate flows (open→edit→save) | s | Every PR |
| GUI | Qt Test / Qt Quick Test | Widgets, models, shortcuts, dialogs | s | Every PR (offscreen) |
| Golden image | custom harness | Rendered output vs CS6 reference | min | Nightly + label |
| PSD round-trip | custom harness | Structural + pixel fidelity | min | Every PR (small corpus), nightly (full) |
| Fuzz | `cargo-fuzz` / libFuzzer | Parsers and command replay | min | PR smoke + nightly soak |
| Performance | Criterion + Qt `QBENCHMARK` | Latency/throughput budgets | min | Nightly + release gate |

### Golden-image comparison methodology

This is the central harness. It must be concrete and reproducible:

1. **Inputs are recipes, not just files.** A golden case is a manifest:
   `{ doc_id, operation: <structured steps>, bit_depth, color_mode, working_space, seed }`.
   Reference rendering and manifest are versioned together. A rendered PNG alone
   is not a test because it cannot be re-derived.
2. **Decode before comparing.** Both images are decoded to a common
   representation: planar, **premultiplied alpha**, f32, **linear light** in a
   single comparison space (default sRGB primaries, linear). Integer code values
   are normalized by bit depth (255 / 65535 / 1.0). This removes gamma and
   premultiplication ambiguity from the metric.
3. **Apply the class metric.** Each operation class has exactly one primary
   metric and optional guards:

   | Operation class | Primary | Guard |
   |---|---|---|
   | Lossless structural (PSD channel decode/encode, whole-pixel move, invert, alpha ops) | exact (0 difference) | — |
   | Integer arithmetic (blend modes at 8/16-bit, threshold, posterize) | max-abs ≤ 1 LSB | ≤0.5% pixels > 1 LSB, none > 2 LSB |
   | Resampling / transform (image size, free transform, warp) | PSNR ≥ 45 dB | max ΔE2000 ≤ 2.0 |
   | Separable/convolution filters (blur, sharpen, noise) | PSNR ≥ 40 dB | DSSIM ≤ 0.001 |
   | Color conversion / ICC (mode change, profile assign) | mean ΔE2000 ≤ 1.0 | max ΔE2000 ≤ 3.0 |
   | 32-bit float HDR | relative ≤ 1e-5 | ULP ≤ 2 near zero |

4. **Alpha-aware comparison.** Composite over a fixed checkerboard *and* compare
   premultiplied channels. A separate unpremultiplied check uses a relaxed
   tolerance because unpremultiply amplifies low-alpha quantization.
5. **Region scoping.** Difference maps are computed per document region
   (layer bounds, selection, ROI). A failure reports the offending layer/ROI, not
   "the canvas", so the diff artifact is actionable.
6. **Determinism gate.** Every golden case is rendered twice in the same job; a
   non-bit-identical pair fails as `FLAKY` before any tolerance is applied. This
   catches unseeded RNG, parallel reduction order, and GPU nondeterminism.
7. **Backend matrix.** A golden case runs on CPU and, where the operation has a
   GPU path, on the GPU (or the software `lavapipe`/`llvmpipe` backend in CI).
   GPU vs CS6 uses the same primary metric; GPU vs CPU additionally uses the
   GPU-vs-CPU slack.

Reference renderings are produced by a documented, manual capture procedure
(CS6 13.0.1, fixed color settings, fixed document, scripted action), stored with
hardware/OS/version metadata. They are treated as read-only test assets, never
committed as "expected output" of our own code.

### Property-based tests

`proptest` strategies generate structured documents and operation sequences:

- **Round-trip:** for any generated document `D`, `parse(serialize(D)) ≡ D`
  (structural equality + pixel equality at the document bit depth).
- **Undo invariants:** for any command sequence `C`, `undo(replay(C))` restores
  the pre-state hash; `redo` restores the post-state; non-linear and snapshot
  behavior per `ARCH-012`.
- **Blend algebra:** commutativity/identity where the mode mathematically
  has it, plus boundary tests at 0/1 alpha and 0/255/65535 codes.
- **Selection algebra:** union/intersect/subtract are associative/commutative as
  defined; selection masks stay in `[0,1]`.
- **Color:** identity transform is exact; two profiles composed equals direct.
- **Format:** every generated PSD/PSB sample parses without panic and preserves
  unknown blocks byte-for-byte.

Failing inputs are shrunk and written to a committed regression seed file.

### Fuzzing

`cargo-fuzz` (libFuzzer) targets, one per untrusted parser: `psd`, `psb`,
`tiff`, `png`, `jpeg`, `gif`, `icc`, `xmp`, `raw`, plus a **command/journal
replay** target for recovery (`XC-012`). Structure-aware fuzzing uses the
`arbitrary` crate so inputs are valid-enough to reach deep code paths. Oracles:
no panic/UB, bounded memory, and "parse then re-serialize is stable" where
lossless. A regression corpus of every historical crash is run on every PR.

### GUI testing

- **Qt Test** for `QObject`/widget logic: `QSignalSpy` for signal assertions,
  `QAbstractItemModelTester` for every item model, `QTest::mouseClick`/`keyClick`
  for interaction.
- **Qt Quick Test** for QML surfaces.
- The app's `--headless` flag selects the offscreen QPA plugin before
  `QApplication` and the self-test asserts the platform is `offscreen`; an
  explicit `QT_QPA_PLATFORM=offscreen` or an `xvfb-run` wrapper still works.
  `--interop-probe` is excluded — it needs a real platform Vulkan instance and
  is not offscreen-compatible.
- **Accessibility** is checked via `QAccessible`/AT-SPI smoke probes (roles,
  names, focus order), matching `02-ui-ux/accessibility.md`.
- Visual widget styling (dark theme, panel layout) is covered by a small
  widget-screenshot golden set, not full-window captures, to limit platform
  font noise.

### Performance regression tests

Budgets live in `ARCH-003`. Enforcement:

- Criterion microbenchmarks for core kernels (blur, resample, composite, ICC).
- Qt `QBENCHMARK` for UI-path latency (menu open, panel refresh).
- Statistical gating: compare against the committed baseline; fail only on a
  p50 regression beyond the threshold that is statistically significant across
  runs, to avoid flaky perf gates.
- Report `p50`/`p95`/`p99` and throughput; the frame-budget criteria in
  `ARCH-003` are checked by a scripted gesture harness, not microbenchmarks.

### PSD round-trip harness

The single highest-value structural test, because it needs no CS6 at runtime:

```text
for each doc in corpus:
    before_bytes = read(doc)
    model        = psd::parse(before_bytes)
    after_bytes  = psd::serialize(model)
    model2       = psd::parse(after_bytes)
    assert structural_eq(model, model2)          # layer tree, masks, styles, paths
    assert opaque_preserved(before_bytes, after_bytes)  # unknown blocks/resources
    assert pixel_eq(model, model2)               # channel decode == encode
    assert metadata_eq(model, model2)            # ICC, EXIF, IPTC, XMP, Version Info
```

"Opaque preserved" compares the byte spans of unknown image resources,
unknown additional-layer-info keys, and the duotone spec. PSB exercises the
8-byte length fields. The corpus is CS6-generated files plus synthetic
edge-case documents (1-px, 56 channels, CMYK/Lab, RLE/ZIP/ZIP-prediction).

### Flakiness and quarantine

Tests that pass but are non-deterministic are quarantined, not deleted; the
determinism gate (render twice) is the first line of defense. `-repeat` and
`QTEST_FATAL_FAIL` isolate Qt-side flakes. CUDA/Mesa/driver variance is pinned
by recording the driver version in the golden manifest.

## Rust module mapping

- `pictura-testkit::golden` — `GoldenCase`, `GoldenManifest`, capture and
  compare entry points; emits `CompareReport`.
- `pictura-testkit::metrics` — `ImageRef` (planar f32, premultiplied),
  `compare_exact`, `compare_lsb`, `compare_psnr`, `compare_dssim`,
  `compare_delta_e2000`; returns `Metric { max, mean, p95, outliers }`.
- `pictura-testkit::tolerance` — `Tolerance` enum per operation class plus
  `ToleranceProfile` loaded from the manifest.
- `pictura-testkit::roundtrip` — PSD/PSB/TIFF/PNG/JPEG structural round-trip
  drivers and `opaque_preserved`.
- `pictura-testkit::determinism` — double-render gate, seeded RNG injection
  (`ChaCha8Rng`), and a tile-scheduling lock for deterministic reductions.
- `pictura-testkit::corpus` — corpus discovery, checksums, provenance metadata.
- `pictura-testkit::proptest` — shared strategies (`doc_strategy`,
  `command_seq_strategy`) behind the `proptest` dev-dependency.
- `pictura-fuzz` (separate, nightly toolchain) — one `fuzz_target!` per parser.
- `xtask` — orchestrates tiers, uploads diff artifacts, checks budgets.

Types crossing crate boundaries: `ImageRef` (borrowed pixel planes),
`CompareReport`, `Tolerance`, `GoldenManifest`. No Qt types enter the Rust core;
the harness is driven from `xtask`/CI, not from the app.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `GoldenDiffViewer` | `QDialog` | Developer-only side-by-side/diff viewer for failed golden cases. |
| `TestCorpusModel` | `QAbstractListModel` | Lists round-trip corpus entries and pass/fail. |
| CTest integration | CMake/CTest | `add_test` per Qt Test executable, `LABELS` per tier. |
| `QTest` runner env | `QTEST_FUNCTION_TIMEOUT`, `QT_QPA_PLATFORM=offscreen` | Timeouts and headless execution. |
| `QBENCHMARK` harness | Qt Test | UI-path latency with `-perf`/callgrind backends. |
| `AccessibilityProbe` | `QAccessible` | Role/name/focus smoke checks. |

Widgets (not QML) for the diff viewer because it is a developer utility and
must render large images efficiently; QML surfaces are tested with Qt Quick Test
rather than through a custom tool.

## Data-model impact

- **No document data changes.** Golden manifests, tolerance profiles, seeds, and
  corpus checksums are test assets outside the document and preferences stores.
- Golden manifest schema: `{ schema, case_id, app_version, cs6_version,
  hardware, driver, color_settings, seed, steps[], tolerance_profile,
  reference_sha256 }`.
- A **`Determinism` knob** exists in the document/render context solely for
  tests (fixed RNG seed and deterministic tile order); it never serializes to
  PSD and is inert in production.
- Round-trip tests rely on the document exposing a stable **structural hash**
  and **revision counter** (`ARCH-008`, `ARCH-012`), which are already proposed
  there.

## Edge cases

- **GPU nondeterminism / driver variance** — fail the determinism gate, then
  apply the GPU-vs-CPU slack only for GPU-vs-CPU comparisons; never relax the
  CS6 comparison silently.
- **Fonts and text rendering** — glyph rasterization is system- and
  version-dependent; text golden tests require a bundled font and are excluded
  on platforms without it. Type parity is asserted structurally (layout,
  metrics), not pixel-exactly.
- **Locale** — number/date formatting and default units differ; tests pin a
  locale or run a locale matrix.
- **Endianness and alignment** — every parser test runs both byte orders; PSB
  8-byte lengths are always covered.
- **Color management absent/present** — comparison space is explicit; CI without
  lcms2 is a hard failure, not a skip.
- **1-px / empty selection / 0-area ROI** — first-class corpus entries.
- **Huge (PSB) documents** — the full corpus is nightly; PR runs a memory-bounded
  subset.
- **Scratch-disk pressure** — round-trip tests run with a small scratch budget to
  exercise spill paths.
- **Float comparison near zero** — use ULP budget, not relative error alone.
- **Cross-platform reference drift** — goldens are captured on the reference
  platform; other platforms run structural tests and only the non-render tiers.
- **Fuzz corpus growth** — cap committed corpus size; crush/merge periodically.
- **Auto-Recovery / Background Save** — crash injection and save-under-load are
  tested here but specified in `XC-012`.

## Parity acceptance criteria

1. Given a CS6-generated PSD with layers, masks, styles, paths, ICC, EXIF, XMP,
   and IPTC, the round-trip harness reports structural equality, byte-identical
   unknown blocks, and pixel equality at the document bit depth.
2. Given any generated document, `parse(serialize(parse(bytes)))` is equal to
   `parse(bytes)` and no property test in the corpus fails with the committed
   seed.
3. Given a golden case in the lossless class, the comparison reports an exact
   match (zero differing codes).
4. Given a golden case in the integer-arithmetic class, max-abs ≤ 1 LSB, fewer
   than 0.5% of pixels exceed 1 LSB, and no pixel exceeds 2 LSB.
5. Given a golden case in the filter class, PSNR ≥ 40 dB and DSSIM ≤ 0.001
   against the CS6 reference.
6. Given a color-conversion golden case, mean ΔE2000 ≤ 1.0 and max ΔE2000 ≤ 3.0.
7. Given a 32-bit float golden case, relative error ≤ 1e-5 and ULP ≤ 2 near zero.
8. Given the same golden case rendered twice in one job, the two outputs are
   bit-identical; otherwise the case fails as `FLAKY`.
9. Given a GPU path, GPU output passes the class metric vs the CS6 reference,
   and GPU-vs-CPU differs by no more than the GPU-vs-CPU slack.
10. Given a Qt Test executable run under `-platform offscreen`, all model checks
    (`QAbstractItemModelTester`) pass and accessibility probes report the
    expected roles/names in the documented order.
11. Given the Criterion baseline, a nightly run whose p50 worsens past the
    threshold is flagged as a regression; a run within noise is not.
12. Given a fuzz target and its regression corpus, every known crash input
    completes without panic and the parser stays within its memory budget.
13. Given a simulated crash during Auto-Recovery, the recovered document matches
    the last checkpoint within the same tolerances as a normal open (see
    `XC-012`).

## Sources

- `https://doc.rust-lang.org/cargo/commands/cargo-test.html` — `cargo test`
  unit/integration/doc targets, `--no-fail-fast`, `CARGO_BIN_EXE_<name>`,
  test working directory.
- `https://proptest-rs.github.io/proptest/` — property testing model
  (Hypothesis-style), automatic shrinking, minimal failing case.
- `https://rust-fuzz.github.io/book/cargo-fuzz.html` and
  `https://rust-fuzz.github.io/book/cargo-fuzz/setup.html` — `cargo-fuzz` wraps
  libFuzzer via `libfuzzer-sys`; nightly toolchain and sanitizer requirements.
- `https://docs.rs/criterion/latest/criterion/` — statistics-driven
  micro-benchmarking; p-value/confidence and baseline comparison model.
- `https://docs.rs/dssim/latest/dssim/` — multi-scale SSIM comparison, LAB
  conversion, `Val` result on 0..1 scale; the perceptual metric proposed above.
- `https://doc.qt.io/qt-6/qtest-overview.html` — Qt Test data-driven tests,
  `QTEST_FUNCTION_TIMEOUT`, `-platform offscreen`, `-repeat`, benchmarking
  back-ends (walltime, tickcounter, callgrind, Linux perf).
- `https://doc.qt.io/qt-6/qttest-index.html` — Qt Test vs Qt Quick Test modules,
  `QSignalSpy` and `QAbstractItemModelTester`.
- `https://docs.qt.io/qt-6/qsavefile.html` — used by the recovery harness to
  observe atomic writes (`commit`, `cancelWriting`, direct-write fallback).
- CS6 behavior for Background Save / Auto-Recovery:
  `https://www.photoshopessentials.com/basics/background-auto-save-cs6/` and
  `https://photoshopguides.github.io/File%20Handling`.
- `https://web.archive.org/web/20240303004655/https://helpx.adobe.com/photoshop/kb/file-recovery-photoshop.html`
  — Adobe KB: recovery information stored at user-specified intervals and
  recovered on restart; default 10 minutes, can be set to 5.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD/PSB structure, compression codes, image resources, CS6 Auto Save
  resources 1086/1087, Version Info 1057.
- Internal: `docs/01-architecture/performance-targets.md`,
  `docs/01-architecture/file-formats.md`,
  `docs/01-architecture/undo-history.md`.

No Adobe source code, binaries, or assets were used; reference renderings are
captured outputs and are governed by
`00-overview/licensing-and-provenance.md`.

## Open questions

- **Which operations are actually deterministic in CS6?** The determinism
  assumption is *(inferred)*. Resolve by rendering the same case twice on one
  CS6 install; if any operation is nondeterministic, it moves to a
  distribution-based tolerance.
- **Tolerance values.** All thresholds above are engineering guesses. Resolve by
  capturing a CS6 reference corpus and measuring the spread of our implementation
  against it; publish the calibration table with the corpus.
- **Reference hardware.** CS6 pixel output may vary by GPU/driver (Mercury) and
  by color settings. Resolve by documenting the exact CS6 environment and
  capturing on CPU (Mercury off) as the canonical reference.
- **Legal status of captured reference renderings.** Are CS6-produced images
  redistributable as test fixtures? Resolve with
  `00-overview/licensing-and-provenance.md` before committing any.
- **Where the reference corpus lives.** In-repo (size), Git LFS, or a separate
  artifact store. Resolve with a size/access decision.
- **GPU in CI.** `lavapipe`/`llvmpipe` coverage vs a real-GPU runner, and how to
  keep golden GPU tests non-flaky. Resolve with a CI prototype.
- **Qt Quick Test coverage.** How much UI is QML vs Widgets is undecided
  (`01-architecture/qt6-ui-design.md`); the GUI tier follows that decision.
- **Text rendering tolerances.** Whether to bundle fonts and pixel-test text at
  all. Resolve with a typography spike.
- **Fuzz target availability on `aarch64`.** libFuzzer/sanitizer support varies;
  Resolve with an architecture/CI decision.
- **Flake quarantine policy.** Who may quarantine and for how long. Resolve with
  a written CI policy.
