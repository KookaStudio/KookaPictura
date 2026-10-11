# Design: add-match-color

## Context

`docs/04-image-ops/adjustments/match-color.md` (ADJ-031) is the behavioral
contract: match a target's colours to a source's statistics with Neutralize,
Luminance, Colour Intensity, and Fade. CS6's kernel is closed; the doc proposes
the Reinhard et al. mean/std transfer as the behavioural model. This change
implements that model end to end: kernel in `pictura-adjust`, statistics and
preview plumbing in the `pictura-app` bridge, and a modal dialog replacing the
`ponytail` menu stub.

## Non-goals

- No Match Color adjustment layer (CS6 has none) and so no PSD encoding; like
  Replace Color this is dialog-only and native-depth apply refuses it.
- The Help's eyedropper/Info-panel feedback while the dialog is open is not
  implemented (modal dialog, like the other adjustment dialogs).
- Cross-profile statistics stay unconverted (working composites only); the
  doc lists this as an open question.

## Decisions

### Colour space: CIELAB, local conversion

Statistics and the transfer run in CIELAB (D65): `L` carries the Luminance
slider, `a`/`b` carry Colour Intensity, and an achromatic output is exactly
`a = b = 0`. The conversion (sRGB EOTF → linear RGB → XYZ D65 → Lab, and the
inverse) is ~40 lines of standard f64 math implemented locally in
`pictura-adjust/src/match_color.rs`; the crate depends only on `pictura-core`
and the existing `pictura-codec::rgb_to_lab` is u8-quantized planar (too coarse
for statistics). `ponytail:` marks the approximation class and the possible
consolidation into `pictura-color` (#269). Adobe's space is unpublished; the
doc lists the exact space as an open question.

### Transfer, and the four modifiers

For Lab channel `c`:

```
matched[c] = (in[c] − target.mean[c]) · scale[c] + source.mean_effective[c]
scale[c]   = source.std[c] / target.std[c]        (1.0 when target.std ≈ 0)
```

- **Neutralize** replaces the source mean with `[source.mean_L, 0, 0]`: the
  transfer keeps the source's luminance and contrast statistics but imposes no
  colour cast. With `Source = None` (source statistics = the target's own)
  this is exactly a gray-world cast removal, which is how the Help describes
  Neutralize working by itself.
- **Luminance** (1–200, default 100) multiplies the transferred `L` by
  `luminance/100`; 100 is identity and the effect is monotonic.
- **Colour Intensity** (1–200, default 100) multiplies `a`/`b` by
  `(intensity − 1)/99`, so 1 is exactly achromatic (an exact gray, not a
  1 % residue) and 100 is exactly identity.
- **Fade** blends the matched pixel toward the original by `fade/100`
  (0 = full match, default 0; 100 = identity). This resolves the doc's
  "Fade range and default" open question in favour of the Help-sourced
  sentence ("moving the slider to the right reduces the amount"): the draft
  table's inferred default 100 and its acceptance criterion 4 ("Fade = 0 …
  identity") cannot both hold with that direction, so the sourced statement
  wins and the criterion is superseded. A self-match (source statistics equal
  the target's) with default options short-circuits to an exact no-op rather
  than a ±1 LSB Lab round-trip.

Statistics are weighted: a pixel contributes `(alpha/255) · (selection
coverage/255)` when a selection plane is given, `alpha/255` otherwise, so
transparent canvas and unselected pixels do not skew the means.

### Where statistics are computed

`match_color_stats(pixels: &[[u8; 4]], width, selection: Option<&[u8]>)` lives
in `pictura-adjust` next to the Lab math and is pure. The *bridge* gathers
inputs, because statistics must cover the **whole** layer or document — never
the preview section, which is what the shared apply path crops:

- Target: the active layer as a document-sized surface
  (`pictura_paint::layer_surface`), optionally restricted to the target
  document's selection when "Use Selection In Target To Calculate Adjustment"
  is checked.
- Source: another document's merged composite
  (`current_buffer` → `surface_from_composite`) or one pixel layer
  (`layer_surface`), restricted to the *source* document's selection when
  "Use Selection In Source To Calculate Colors" is checked. `Source = None`
  reuses the target statistics.
- The dialog recomputes statistics only when the source, layer, or the two
  selection checkboxes change; sliders only re-run the pointwise preview.

With no pixel contributing weight, the statistics are zeroed and the kernel's
flat-target guard (`scale = 1`) makes the transfer an identity — the doc's
"1-px / flat-color" edge case.

### Ignore Selection When Applying Adjustment

The shared apply path (`apply_op_active_region`) always confines an op to the
active selection. This change adds an `ignore_selection: bool` parameter
(false at every existing call site) that skips the mask, so the whole layer is
adjusted; the engine already treats `mask = None` as full coverage.

### Save / Load Statistics

The dialog writes the six statistics doubles as a 56-byte binary file: the
ASCII magic `PCTMCST`, a `u8` version, then six little-endian `f64`s. Adobe's
`.sta` format is undocumented (doc open question); a lossless in-tree format
keeps Save→Load bit-for-bit reproducible, which the doc's acceptance 7 asks
for. The file dialog filter is `*.sta`.

### RGB-only enablement

The menu row is enabled only when the target is an editable pixel layer *and*
`document_mode()` reports `rgb`, per the Help's RGB-only restriction (the app
models RGB and Grayscale; Grayscale disables the row).

## Risks / trade-offs

- Lab round-trip can shift a pixel by ±1 LSB; only the exact self-match
  short-circuit avoids it by construction. Real matches are approximations
  and marked as such; no golden baseline covers this path.
- Statistics scan whole documents on the GUI thread when the source changes
  (same cost class as the histogram the other adjustment dialogs take at
  open); slider drags are section-restricted point operations, as for the
  other adjustment dialogs.
