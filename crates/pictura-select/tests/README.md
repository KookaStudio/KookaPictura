# M5-B — ImageMagick selection-mask oracle

These tests diff the `pictura_select` modify ops against **ImageMagick**
grayscale morphology and Gaussian blur. This is a *sanity* oracle, not a parity
oracle: Adobe does not publish the structuring element (square vs. Euclidean
disk), the operation ordering, or the feather radius→sigma mapping.

`pictura_select` (task M5-A) landed while this oracle was being written, so the
mapping below is calibrated to its **actual** implementation. The differential
and property tests still carry `#[ignore = "enable once M5-A lands"]` per the
M5-B brief; run them with `--ignored`.

- Oracle script: `scripts/select_oracle.py`
- Tests: `crates/pictura-select/tests/oracle.rs`
- Verified against: ImageMagick **7.1.2-29 Q16-HDRI** (2026-07-27).
- Spec: `docs/08-selection/grow-similar-and-modify.md` (`SEL-013`),
  `docs/08-selection/selection-model.md` (`SEL-001`).

## Mask layout

Raw 8-bit grayscale, one byte per pixel (`gray:`), matching the `Selection.data`
coverage layout (0 = outside, 255 = inside).

```bash
magick -size 8x8 -depth 8 gray:IN.gray <OPERATOR ARGS> -depth 8 gray:OUT.gray
```

## Selection op → ImageMagick mapping

| `Selection` op | ImageMagick | Equivalent? | Tolerance | Notes |
|---|---|---|---|---|
| `expand(r)` | `-morphology Dilate Square:r` | yes (M5-A) | 0 | M5-A uses a separable box element `(2r+1)²`; IM `Square:r` is the same. |
| `contract(r)` | `-morphology Erode Square:r` | yes (M5-A) | 0 | As `expand`. IM clamps outside the canvas to the edge, matching PS's canvas-edge exemption and M5-A's clamp. |
| `feather(r)` | `-gaussian-blur 0x(r/2)` | yes (M5-A) | 0 | M5-A uses a separable Gaussian with `σ = r/2` and radius `⌈3σ⌉`; matches IM exactly on the 8×8 mask. The differential pins `r = 2`; `σ >= 2` (radius >= 4) uses the variance-matched three-box approximation instead, which is not byte-compared to IM. |
| `smooth(r)` | `-morphology Smooth Square:r` | **no** | 32 | IM `Smooth` is a mean filter; PS `Smooth` is a majority vote over a square window of sample radius `r`. Shape/magnitude only; not exercised. |
| `invert` | `-negate` | yes | 0 | Identical `255 - v`. |
| (threshold helper) | `-threshold T%` | yes | 0 | Binarizes a coverage mask at `T`. |

Tolerances are absolute per-8-bit-sample allowances. `0` means the output is
identical up to Q16 rounding (none observed on the 8×8 mask). The `smooth` row
is the only approximate one; the feather row is exact only at the narrow-kernel
radius the differential pins.

## Why `Square:r` and not `Disk:r`

The spec (`grow-similar-and-modify.md`) proposes a threshold-independent
**distance transform** = Euclidean dilation/erosion, i.e. IM `Disk:r`; it leaves
the shape an open question ("square vs. Euclidean disk"). M5-A instead
implements a **separable box** min/max over a `(2r+1)²` window, which is
`Square:r`. On the 8×8 test mask the two diverge at exactly the 4 diagonal
corner pixels (max delta 255), so the differential tests pass
`--kernel Square:r` explicitly. `--radius r` still selects the Euclidean
`Disk:r` default for manual inspection.

## Exact ImageMagick flags

| Operation | Flags |
|---|---|
| Dilate / Expand | `-morphology Dilate Square:r` (`--op dilate --kernel Square:r`) |
| Erode / Contract | `-morphology Erode Square:r` (`--op erode --kernel Square:r`) |
| Smooth | `-morphology Smooth Square:r` (`--op smooth --radius r`) |
| Open / Close / Gradient | `-morphology Open\|Close\|Gradient <kernel>` |
| Feather | `-gaussian-blur 0xSIGMA` (`--op gaussian --sigma S`) |
| Blur (`-blur` variant) | `-blur 0xSIGMA` (`--op blur --sigma S`) |
| Invert | `-negate` |
| Threshold | `-threshold T%` (`--op threshold --level T`, `T` in 0..255) |

`--kernel` accepts any IM kernel string (`Square:1`, `Disk:2`, `Diamond:1`, …);
`--radius r` (default 1) builds `Disk:r` when `--kernel` is omitted.
`--im-args="..."` passes verbatim operator arguments when no `--op` is given
(attach with `=`, or argparse reads the leading `-` as another option).

## Known divergences (ImageMagick vs. Photoshop / M5-A)

- **Structuring element (divergence found).** The spec proposes Euclidean
  distance-transform expand/contract (`Disk:r`); M5-A implements a separable
  square box (`Square:r`). They differ at the diagonal corners by a whole pixel.
  The oracle follows M5-A and exposes both.
- **Soft masks.** IM morphology is grayscale min/max on the coverage values;
  M5-A's `morph` also min/maxes, so they agree even on soft masks. The spec's
  proposed distance transform would *not* (it preserves soft edges rather than
  binarising), so the oracle does not test that interpretation.
- **Canvas edge.** IM's default `-virtual-pixel edge` clamps at the canvas
  border, so a selection touching the border is not eroded from that side. This
  matches Photoshop's canvas-edge exemption and M5-A's `clamp`. Do not pass
  `-virtual-pixel` for morphology.
- **`feather` radius→sigma (resolved).** The spec's `σ = r/2` is inferred, not
  documented; M5-A happens to use exactly `σ = r/2` with a `⌈3σ⌉` support, and
  IM agrees sample-for-sample on this mask.
- **`-blur` vs. `-gaussian-blur`.** They are *different* implementations on this
  build (`-blur 0x1` put 38 on an isolated pixel, `-gaussian-blur 0x1` put 41).
  The oracle exposes both; the feather mapping uses `-gaussian-blur`.
- **`smooth`.** PS `Smooth` is a majority-vote (median-like) filter over a
  `(2r+1)²` window; IM `Smooth` is a mean filter. Not equivalent; the tolerance
  only absorbs magnitude, not the operator difference.
- **Bit depth.** M5 is 8-bit only; IM computes in Q16-HDRI float and rounds on
  output, so a ±1 rounding difference is possible for non-binary masks.

## Active vs. ignored tests

Active (run under `cargo test -p pictura-select --test oracle`):

- `reference_script_is_present`, `imagemagick_runs` — the script is present and
  `magick` answers.
- `oracle_negate_matches_expected_bytes` — `-negate` == `255 - v`.
- `oracle_dilate_grows_a_single_pixel` — `Disk:1` dilates one pixel to its
  5-pixel cross and keeps the mask binary.
- `mapping_documents_ops_and_tolerances` — the table above is internally
  consistent.

Ignored (`#[ignore = "enable once M5-A lands"]`; all pass with `--ignored` after
M5-A landed):

- `expand_matches_imagemagick_dilate`, `contract_matches_imagemagick_erode`
  (tolerance 0, `Square:r`).
- `feather_matches_imagemagick_gaussian` (tolerance 0, `σ = r/2`).
- `invert_is_an_involution`, `boolean_identities` — no ImageMagick, pure
  `Selection` algebra.

The differential tests skip with a message when `magick` is not on `PATH`.

## Running

```bash
cargo test -p pictura-select --test oracle                # active tests
cargo test -p pictura-select --test oracle -- --ignored   # differential + property
```

Manual inspection:

```bash
python3 scripts/select_oracle.py version
python3 scripts/select_oracle.py apply --size 8x8 --op dilate --kernel Square:1 IN.gray OUT.gray
python3 scripts/select_oracle.py apply --size 8x8 --op gaussian --sigma 1.0 IN.gray OUT.gray
python3 scripts/select_oracle.py apply --size 8x8 --im-args="-negate" IN.gray OUT.gray
```

## Regeneration

There are no committed fixtures: the oracle runs ImageMagick at test time and
skips when `magick` is absent. To reproduce a reference mask by hand, write a
raw `8x8` grayscale mask (e.g. a `4x4` white block at `(2,2)`) and run the
script; `MAGICK=/path/to/magick` overrides the binary.

## Not expressed by this oracle

- `border`, `smooth` majority-vote semantics, `magic_wand` / `grow` / `similar`
  / `color_range` — no faithful ImageMagick equivalent (property tests belong to
  M5-A/M5-C).
- The spec's Euclidean-distance-transform interpretation of expand/contract.
- 16/32-bit masks; alpha-channel combinations.
