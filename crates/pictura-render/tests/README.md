# M2-B — ImageMagick compositing oracle

These tests diff `pictura_render::composite_rgba` against **ImageMagick**, a
second, independent implementation of the blend modes. ImageMagick implements
the SVG/W3C Compositing blend operators, so for the modes it supports the two
should agree up to integer rounding; where it uses a deliberately different
algorithm the mode is not used as an oracle at all.

- Scenes and reference generation: `scripts/im_compose.py`
- Fixtures: `crates/pictura-render/tests/fixtures/*.rgba`, raw **interleaved
  RGBA8**, 8×8, row-major. `*_base.rgba` / `*_src.rgba` are the two input
  layers (base = destination, src = source); `<scene>_<Mode>.rgba` is the
  `magick -compose` result for that PSD mode.
- Tests: `crates/pictura-render/tests/oracle.rs`. The tests that call
  `composite_rgba` are `#[ignore]`d until task M2-A lands.
- Verified against: ImageMagick **7.1.2-29 Q16-HDRI**.

## PSD mode → ImageMagick compose operator

19 of the 27 modes have an ImageMagick operator that computes the same function.
The remaining 8 do not and are covered by hand-computed W3C unit tests in
`pictura-render` instead (per the M2 exit gate).

| PSD mode | PSD key | IM `-compose` operator | Supported | Tolerance | Notes |
|---|---|---|---|---|---|
| Normal | `norm` | `Over` | yes | 0 | Porter–Duff source-over |
| Dissolve | `diss` | — | **no** | — | stochastic; IM needs `-define compose:args` and uses its own dither pattern |
| Darken | `dark` | `Darken` | yes | 0 | |
| Multiply | `mul ` | `Multiply` | yes | 0 | |
| Color Burn | `idiv` | `ColorBurn` | yes | 0 | |
| Linear Burn | `lbrn` | `LinearBurn` | yes | 0 | |
| Darker Color | `dkCl` | — | **no** | — | IM `DarkenIntensity` compares luminance, Photoshop compares the channel **sum** |
| Lighten | `lite` | `Lighten` | yes | 0 | |
| Screen | `scrn` | `Screen` | yes | 0 | |
| Color Dodge | `div ` | `ColorDodge` | yes | 0 | |
| Linear Dodge (Add) | `lddg` | `LinearDodge` | yes | 0 | `min(1, Cb+Cs)`, keeps Over alpha (unlike `Plus`, which adds alpha) |
| Lighter Color | `lgCl` | — | **no** | — | IM `LightenIntensity` compares luminance, not the channel sum |
| Overlay | `over` | `Overlay` | yes | 0 | |
| Soft Light | `sLit` | — | **no** | — | this IM build's `SoftLight` is not the W3C/PS formula (grey-on-grey returns white) |
| Hard Light | `hLit` | `HardLight` | yes | 0 | |
| Vivid Light | `vLit` | `VividLight` | yes | 1 | |
| Linear Light | `lLit` | `LinearLight` | yes | 0 | |
| Pin Light | `pLit` | `PinLight` | yes | 0 | |
| Hard Mix | `hMix` | `HardMix` | yes | 0 | threshold at 0.5; the exact `Cb+Cs == 1` boundary differs, the scene avoids it |
| Difference | `diff` | `Difference` | yes | 0 | |
| Exclusion | `smud` | `Exclusion` | yes | 0 | |
| Subtract | `fsub` | `MinusSrc` | yes | 0 | PS Subtract = base − source. IM's operand order is reversed vs. its docs, so `MinusSrc` (not `MinusDst`) yields base − source |
| Divide | `fdiv` | `DivideSrc` | yes | 0 | PS Divide = base / source; same operand-order reversal, so `DivideSrc` |
| Hue | `hue ` | — | **no** | — | IM `Hue` blends in HSL space, not SVG `SetSat`/`SetLum`; measured divergence up to 70/255 |
| Saturation | `sat ` | — | **no** | — | IM `Saturate` blends in HSL space; divergence up to 94/255 |
| Color | `colr` | — | **no** | — | IM `Colorize` blends in HSL space; divergence up to 92/255 |
| Luminosity | `lum ` | — | **no** | — | IM `Luminize` blends in HSL space; divergence up to 35/255 |

`Plus` is **not** used for Linear Dodge: `Plus` also adds the alpha channels,
whereas Photoshop's Linear Dodge uses normal source-over alpha. IM's
`Subtract`/`Add` are the modulus (wrap-around) operators and are also not used.

The 8 unsupported modes stay covered by M2-A's hand-computed W3C unit tests:
Dissolve (stochastic), Darker/Lighter Color (channel-sum comparison), Soft
Light (formula variant), and Hue/Saturation/Color/Luminosity (IM uses HSL-space
blending rather than the SVG non-separable model).

## Scenes

| Scene | Base | Source | Modes generated |
|---|---|---|---|
| `solid` | uniform `(64,128,192,255)` | uniform `(200,100,50,255)` | all 19 |
| `ramp` | opaque color gradient | opaque color gradient | all 19 |
| `alpha` | opaque color gradient | solid color, varying alpha | `Normal` only |

`solid` isolates the blend function per channel; `ramp` sweeps many color
pairs; `alpha` validates source-over with a partial-alpha source. Blend modes
other than Normal are not tested with a partial-alpha source because
ImageMagick's blend operators do not reproduce the W3C source-over mix there.

## Regenerating the fixtures

Requires `magick` (ImageMagick 7) on `PATH` (override with `MAGICK=...`) and
`python3`. No Python packages are needed.

```bash
python3 scripts/im_compose.py gen     # rewrite tests/fixtures/
python3 scripts/im_compose.py check   # regenerate in a temp dir and diff
python3 scripts/im_compose.py list    # `magick -list compose` (add --grep)
```

`im_compose.py compose <operator> <base> <src> <out>` composites two layers
directly; inputs/outputs may be `.png` or raw `.rgba`. Internally the script
routes through PNG: reading and writing raw RGBA directly makes ImageMagick
mangle the alpha channel of some operators (notably `Difference`), while PNG
keeps the alpha semantics correct.

The scenes are pure functions of pixel coordinates, so generation is
deterministic; `check` is exercised by `imagemagick_fixtures_reproduce` in
`oracle.rs`.

# Merge Down oracle

`tests/merge_oracle.rs` diffs `pictura_render::merge_scope` (Merge Down)
against the same ImageMagick references. The merge composites the source and
the target through the document compositor, so the **source's own blend mode**
governs the result, and the merged pixels must match ImageMagick's operator
for that mode.

- `merge_down_matches_committed_imagemagick_fixture` — the `alpha` scene
  merged with a `Normal` source, against the committed `alpha_Normal.rgba`.
- `merge_down_matches_imagemagick_for_every_supported_mode` — the `solid` and
  `ramp` scenes merged with a source carrying each of the 19 modes
  ImageMagick implements with the same formula, against the committed
  `<scene>_<Mode>.rgba` references, at the same per-mode tolerances the
  compositing oracle uses (0, except Vivid Light at 1). The eight modes with
  no usable ImageMagick operator stay covered by the hand-computed W3C unit
  tests in `pictura-render`.
- `merge_down_bakes_the_sources_blend_into_the_pixels` — the contract, with
  hand-computed pixels: a `Multiply` source of `(120, 130, 140)` over a
  `Screen` target of `(40, 60, 80)` merges to `(19, 31, 44)`, and the merged
  node keeps the *target's* blend mode and opacity. The source's blend mode is
  baked into the pixels, not carried onto the result. The target's blend is
  `Screen` so the assertion cannot pass by accident: a merged node that
  hardcoded `Normal` would fail it.
- `merge_down_matches_live_imagemagick` — regenerates the reference with a
  running `magick` (the `alpha` scene's source-over and the `ramp` scene's
  `Multiply`) and self-skips when `magick` is not on `PATH`.

The `alpha` scene is `Normal`-only here for the same reason it is in the
compositing oracle: ImageMagick's blend operators do not reproduce the W3C
source-over mix for a partial-alpha source, so the blend modes are checked
against the fully opaque `solid` and `ramp` scenes.

# Knockout psd-tools oracle

`tests/knockout_oracle.rs` diffs `pictura_render::composite_rgba` against
**psd-tools**' own compositor, an independent implementation of deep knockout
(`psd_tools/composite/composite.py`, "Verified against Photoshop"). The fixture
`../pictura-codec/tests/fixtures/knockout.psd` is a red Background, an
intermediate green layer, and a half-fill blue layer carrying `knko = Deep`;
`tests/fixtures/knockout_deep.rgba` is the raw interleaved-RGBA8 reference from
`python3 scripts/psd_knockout_reference.py gen`. The test self-skips when
`psd_tools` is missing. The tolerance is **1**: psd-tools floors the 8-bit
result while the CPU rounds, so the half-fill blue over red lands on 126 vs 127
at every sample; a larger disagreement is a behavior bug, not rounding. The test
also asserts the semantic punch-through (the intermediate green channel is
exactly 0 at a covered pixel).

`tests/knockout_oracle.rs` also covers `knockout_group.psd`: a red Background
plus a **pass-through** group of green and the same half-fill `knko = Deep`
blue. psd-tools' own compositor applies a knockout inside a pass-through group,
so the group's green is punched through to the background; its reference is
`tests/fixtures/knockout_group.rgba`, generated by
`python3 scripts/psd_knockout_reference.py gen --out
crates/pictura-render/tests/fixtures/knockout_group.rgba`. The test asserts the
group decodes as `PassThrough` and its child as `Deep`, so a fixture that
silently becomes isolated fails rather than passing for the wrong reason.

`tests/knockout_oracle.rs` also covers `knockout_isolated_group.psd`: a red
Background, a `Yellow` layer below, and an **isolated** (explicit Normal) group
of green and the same half-fill `knko = Deep` blue. psd-tools stops a deep
knockout at an isolated group, so the group's green is punched through to the
group's own initial backdrop and the group then blends over the yellow layer; the
discriminating result is `(126,127,128)`. Its reference is
`tests/fixtures/knockout_isolated_group.rgba`, generated by
`python3 scripts/psd_knockout_reference.py gen --out
crates/pictura-render/tests/fixtures/knockout_isolated_group.rgba`. The test
asserts the group is not `PassThrough` and its child is `Deep`, and that the
layer below the group still contributes (a deep knockout reaching the document
background would zero the green channel).

# M12-B — structural document-ops oracle

`tests/document_oracle.rs` covers the M12 document operations
(`resize_document`, `resize_canvas_document`, `rotate_document`,
`flip_document`). It is a **structural** oracle, not a pixel-parity one: it
checks that an operation leaves a document that still round-trips and stays
self-consistent, not that any particular pixel landed where Photoshop would put
it. Pixel behavior of the underlying kernels is owned by `pictura-ops`' own
oracle.

## What is checked

| Class | Check | Tolerance |
|---|---|---|
| Structural | After each op, `write_psd` → `read_psd` preserves `doc.width`/`doc.height` and the top-level layer count. | Exact |
| Structural | Depth-first `(name, rect)` list is identical before write and after read (groups and children included). | Exact |
| Consistency | The engine leaves `doc.composite` untouched; the oracle materializes it with `composite_rgba(&doc)` before writing. | Exact (`PartialEq` on the `PixelBuffer`) |
| Exactness | Four `rotate_document(1)` calls equal the original document. | Exact |
| Exactness | `flip_document` twice (each axis) equals the original document. | Exact |
| Exactness | Canvas grow centered, then shrink centered back to the original size, equals the original document. | Exact |
| Exactness | Canvas grow anchored `TopLeft`, then shrink anchored `BottomRight` back to the original size restores `doc.width`/`doc.height`. | Exact |
| External | `psd-tools` (via `scripts/validate_output.py`) opens a resized and a rotated PSD and reports the expected width, height, and layer count. | Exact |

The exactness checks compare whole `Document`s, so the original must be
canonical: `sample_doc()` sets `doc.composite = composite_rgba(&doc)` before the
test, matching what the app transaction materializes after an op.

"Opposite anchor restores dimensions" is dimensions only. Opposite anchors move
the content (a `TopLeft` grow followed by a `BottomRight` shrink shifts the
stack), so the content-preserving case is the centered grow/shrink, which is the
one asserted for document equality.

## Structural vs pixel tolerances

Dimensions, layer counts, names, and rects are integers with no tolerance. The
composite check has no tolerance either: both sides come from
`composite_rgba` over the same layer stack, so they must be byte-identical. The
resample kernels themselves are not compared against an external image, so a
resize that is wrong-but-structurally-valid still passes here by design; that
gap is covered by the `pictura-ops` oracle.

## External `psd-tools` check

`psd_tools_sees_resized_document` and `psd_tools_sees_rotated_document` write a
plain two-layer document to a scratch PSD and run
`python3 scripts/validate_output.py <path>`, parsing the header line
`<path>: <W>x<H> mode=<MODE> depth=<N> layers=<K>`. When `python3` or
`psd_tools` is missing, the test prints a skip message and returns without
failing. Requires `psd-tools` 1.x (verified with 1.19.0); no `magick` needed.

`dimension_check_bites` proves the dimension check itself has teeth: it round-
trips a resized document and asserts `check_dims` accepts the right size and
rejects a deliberately corrupted width or height.

```bash
cargo test -p pictura-render --test document_oracle
```

