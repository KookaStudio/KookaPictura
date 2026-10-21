# Blend Modes

- **Spec ID:** `LAY-010`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the 27-mode set and all blend formulas are unchanged from CS5 (Subtract/Divide date from CS5 or earlier, per community sources). CS6's blend-adjacent changes are UI/workflow only: group styles, the layer-style effects reorder, and a Dither option on Gradient Overlay/Stroke. Exact CS5-vs-CS6 mode-set differences are noted in `## Open questions`.
- **Depends on:** `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/color-management.md` (`ARCH-007`), `01-architecture/gpu-rendering-pipeline.md`, `05-layers/layers-overview.md`, `05-layers/layer-styles.md` (`LAY-011`), `05-layers/adjustment-layers.md` (`LAY-012`), `05-layers/fill-layers.md` (`LAY-013`), `04-image-ops/image-modes.md`.

> All module, crate, and widget names are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract for
> the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and
> what did not. Blend math is quoted from the W3C *Compositing and Blending Level
> 1* specification where that spec defines the mode; Photoshop-only modes are
> marked *(community-documented)*; anything not sourced is marked *(inferred)*.
> Adobe's exact integer rounding and 32-bit behavior are closed and are listed in
> `## Open questions`.

## CS6 behavior

A **blend mode** determines how the pixels of a layer (the **blend** or **source**
color, `Cs`) combine with the pixels already composited underneath (the **base** or
**backdrop** color, `Cb`) to produce the **result** color. CS6 has **27 layer
blend modes**. The layers panel menu, the options bar `Mode` menu of painting
tools, each layer-style effect's own `Blend Mode`, and Smart Filter blending all
use this set (tool-only modes add `Behind` and `Clear`, which are *not* layer
modes).

The canonical enumeration is the PSD 4-byte key. Grouping used throughout this
spec:

| Group | Modes | PSD keys |
|---|---|---|
| Normal / disappear | Normal, Dissolve | `norm`, `diss` |
| Darken | Darken, Multiply, Color Burn, Linear Burn, Darker Color | `dark`, `mul `, `idiv`, `lbrn`, `dkCl` |
| Lighten | Lighten, Screen, Color Dodge, Linear Dodge (Add), Lighter Color | `lite`, `scrn`, `div `, `lddg`, `lgCl` |
| Contrast | Overlay, Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix | `over`, `sLit`, `hLit`, `vLit`, `lLit`, `pLit`, `hMix` |
| Inversion | Difference, Exclusion, Subtract, Divide | `diff`, `smud`, `fsub`, `fdiv` |
| Component | Hue, Saturation, Color, Luminosity | `hue `, `sat `, `colr`, `lum ` |

`Behind` (paints only where destination alpha is 0) and `Clear` (sets alpha to 0)
appear in the options bar for the Brush, Pencil, Paint Bucket, Shape (fill-region),
Fill, and Stroke tools; they require **Lock Transparency off** and have no layer
equivalent. `Threshold` is the name Photoshop gives to `Normal` in Bitmap and
Indexed modes.

Blend modes are used in several distinct compositing contexts, and the result
differs subtly between them:

- **Layer-to-backdrop.** The layer's pixels blend with the accumulated composite
  of all visible layers below, at the layer's position in the stack.
- **Group blend mode.** A group's default mode is `Pass Through`: children blend
  directly against the parent backdrop. Choosing any other mode first composites
  the group's children into the group's own buffer, then blends that buffer into
  the backdrop as a single unit. Consequently, no adjustment layer or child blend
  mode inside a non-passthrough group affects layers outside the group.
- **Painting-tool mode.** The tool blends into the active layer's existing pixels.
  Where the active layer has no pixels, the first dab is written as `Normal`; later
  overlapping strokes use the tool's mode. Tool blends are baked, not re-editable.
- **Layer-style / smart-filter blend mode.** Each effect or smart filter carries
  its own mode and opacity and composites as a separate pass (see `LAY-011`).

### Blend Interior Effects As Group (layer styles nuance)

`Blend Interior Effects As Group` (Advanced Blending in the Layer Style dialog)
decides whether the **layer's** blend mode is applied to effects that modify
opaque pixels — **Inner Glow, Satin, Color Overlay, Gradient Overlay, Pattern
Overlay, Stroke Emboss** — in addition to the layer's own pixels. Exterior effects
that only affect transparent pixels (**Drop Shadow, Outer Glow**) always blend
against the underlying visible layers and are not governed by this option.

- **Deselected (default):** interior effects composite onto the layer using their
  own effect blend modes; the layer's blend mode is then applied to the combined
  layer+interior result. Interior effects are effectively "inside" the layer.
- **Selected:** the layer's blend mode is applied to the interior effects as a
  group, and the whole group is then composited with the underlying layers.

The closely related `Blend Clipped Layers As Group` (default **on**) applies the
base layer's blend mode to every layer in a clipping mask; turning it off keeps
each clipped layer's own mode and appearance. Also in Advanced Blending:
`Transparency Shapes Layers` (restrict effects/knockouts to opaque pixels),
`Layer Mask Hides Effects`, `Vector Mask Hides Effects`, and per-channel
`Include` checkboxes that exclude a color channel from blending.

### Mode availability restrictions

- **32-bit (HDR) documents:** only **Normal, Dissolve, Darken, Multiply, Lighten,
  Darker Color, Linear Dodge (Add), Lighter Color, Difference, Subtract, Divide,
  Hue, Saturation, Color, Luminosity** are offered (14 modes).
- **Lab documents:** **Color Dodge, Color Burn, Darken, Lighten, Difference,
  Exclusion, Subtract, Divide** are unavailable.
- **Bitmap / Indexed:** only `Normal` (`Threshold` for bitmap) is meaningful;
  per-pixel blending of color does not apply.
- **CMYK:** the same mode list applies; `Hard Mix` resolves to the subtractive
  primaries and the maximum channel value is 100 (not 255).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers panel | Drop-down (upper-left of panel) | `Shift + +` / `Shift + -` cycles | Blend Mode menu for the selected layer/group; `Shift+Alt+<letter>` jumps to a mode (see below) |
| Options bar | Drop-down `Mode` | same keys | Painting/editing tools; includes tool-only `Behind`/`Clear` |
| Layer Style dialog | Per-effect `Blend Mode` drop-down | n/a | Drop Shadow, Inner Shadow, glows, Bevel, Satin, Overlays, Stroke |
| Layer Style dialog > Advanced Blending | Checkboxes + `Blend If` sliders | n/a | `Blend Interior Effects As Group`, `Blend Clipped Layers As Group`, channel `Include` |
| Layer Style dialog > Advanced Blending > Blend If | Menu + slider pair | `Alt`-drag to split slider | `Gray` or a single channel; `This Layer` / `Underlying Layer` ranges |
| Filter > *(smart filter name)* > Blending Options | Dialog | double-click blending icon | Mode + opacity for a smart filter |
| Layer > Layer Style > Blending Options | Menu | n/a | Opens Advanced Blending for the layer |

Blend-mode jump shortcuts (CS6 Help, "Keys for blending modes"): `Shift+Alt+N`
Normal, `I` Dissolve, `Q` Behind, `R` Clear, `K` Darken, `M` Multiply, `B` Color
Burn, `A` Linear Burn, `G` Lighten, `S` Screen, `D` Color Dodge, `W` Linear Dodge
(Add), `O` Overlay, `F` Soft Light, `H` Hard Light, `V` Vivid Light, `J` Linear
Light, `Z` Pin Light, `L` Hard Mix, `E` Difference, `X` Exclusion, `U` Hue, `T`
Saturation, `C` Color, `Y` Luminosity. `Lighter Color`, `Darker Color`,
`Subtract`, and `Divide` have no dedicated shortcut in the CS6 table.

## Parameters & ranges

A layer carries a blend mode (`enum`, default `Normal`), an **Opacity**
(`0–100 %`, default 100), and a **Fill** opacity (`0–100 %`, default 100).
`Blend If` uses two per-channel 0–255 slider pairs.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Layer/group Blend Mode | enum (27 + group `Pass Through`) | Normal | table above | Group adds `Pass Through` |
| Opacity | percent | 100 | 0–100 | Scales the whole rendered layer incl. styles |
| Fill | percent | 100 | 0–100 | Scales layer pixels only, not effects |
| Blend If channel | enum | Gray | Gray + per-channel | Per-document color channels |
| This Layer / Underlying Layer | slider pair, 0–255 | 0 / 255 each | 0–255, split for partial blend | Measured on the channel's brightness |
| Neutral color fill (New Layer) | per mode | white / black / 50 % gray | `Fill With (Mode)-Neutral Color` | Unavailable for Normal, Dissolve, Hard Mix, Hue, Saturation, Color, Luminosity |

`Fill With (Mode)-Neutral Color` writes a preset color that is invisible under the
mode. The CS6 Help lists only which modes lack the option; the neutral values
below are the standard Photoshop ones *(inferred / community)*:

| Mode | Neutral | Mode | Neutral |
|---|---|---|---|
| Darken, Multiply, Color Burn, Linear Burn, Darker Color | white | Overlay, Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light | 50 % gray |
| Lighten, Screen, Color Dodge, Linear Dodge (Add), Lighter Color | black | Difference, Exclusion, Subtract | black |
| — | — | Divide | white |

## Algorithms & pipeline

### Definitions and the general compositing equation

Let a color channel be normalized to `[0, 1]` (`C = v / max` for 8/16-bit;
native for 32-bit float). `Cb` = backdrop, `Cs` = source, `αb` = backdrop alpha,
`αs` = source alpha. Per W3C *Compositing and Blending Level 1* §10–§11:

```
B(Cb, Cs)              blending function, per channel (separable) or triplet (non-separable)
Cm       = B(Cb, Cs)   mixed color
Cr       = (1 - αb) * Cs + αb * B(Cb, Cs)          # blend modulated by backdrop alpha
αo       = αs + αb * (1 - αs)                       # Porter-Duff source-over alpha
αo * Co  = αs*(1-αb)*Cs + αs*αb*B(Cb, Cs) + (1-αs)*αb*Cb
```

The equivalent PDF-model form is `Cr = (1-αb)·Cs + αb·B(Cb, Cs)`, then
source-over compositing. The W3C formulas (below) are the normative definition
used for behavioral parity; Adobe's per-pixel integer rounding is not published.

### Separable modes (W3C norm; all channels in [0,1])

| Mode | `B(Cb, Cs)` | Notes |
|---|---|---|
| Normal | `Cs` | a.k.a. `Threshold` in bitmap/indexed |
| Multiply | `Cb * Cs` | commutative |
| Screen | `1 - (1-Cb)*(1-Cs)` = `Cb + Cs - Cb*Cs` | commutative; inverse of Multiply |
| Overlay | `HardLight(Cs, Cb)` | inverse of Hard Light |
| Darken | `min(Cb, Cs)` | per channel |
| Lighten | `max(Cb, Cs)` | per channel |
| Color Dodge | `Cb==0 ? 0 : Cs==1 ? 1 : min(1, Cb/(1-Cs))` | black source = no change |
| Color Burn | `Cb==1 ? 1 : Cs==0 ? 0 : 1 - min(1, (1-Cb)/Cs)` | white source = no change |
| Hard Light | `Cs<=0.5 ? Multiply(Cb, 2*Cs) : Screen(Cb, 2*Cs-1)` | harsh spotlight |
| Soft Light | `Cs<=0.5 ? Cb - (1-2Cs)*Cb*(1-Cb) : Cb + (2Cs-1)*(D(Cb)-Cb)` where `D(x)= x<=0.25 ? ((16x-12)x+4)x : sqrt(x)` | diffused spotlight |
| Difference | `abs(Cb - Cs)` | white source inverts |
| Exclusion | `Cb + Cs - 2*Cb*Cs` | lower-contrast Difference |

### Photoshop-only / extended separable modes *(community-documented)*

These are not defined in W3C Compositing and Blending Level 1. Formulas below are
the standard community definitions (Wikipedia *Blend modes*; Pegtop blend-mode
articles); Adobe's exact clamps remain unverified.

| Mode | `B(Cb, Cs)` | Notes |
|---|---|---|
| Linear Dodge (Add) | `min(1, Cb + Cs)` | Pegtop "additive"; black = no change |
| Linear Burn | `max(0, Cb + Cs - 1)` | Pegtop "subtractive"; white = no change |
| Vivid Light | `Cs<=0.5 ? ColorBurn(Cb, 2Cs) : ColorDodge(Cb, 2Cs-1)` | contrast increase |
| Linear Light | `Cs<=0.5 ? LinearBurn(Cb, 2Cs) : LinearDodge(Cb, 2Cs-1)` = `clamp(Cb + 2*Cs - 1)` | linear contrast |
| Pin Light | `Cs<=0.5 ? Darken(Cb, 2Cs) : Lighten(Cb, 2Cs-1)` | replaces, does not mix |
| Hard Mix | `(Cb + Cs) >= 1 ? 1 : 0` | every channel becomes 0 or max; CMYK max = 100 |
| Subtract | `max(0, Cb - Cs)` | negative clipped to 0 in 8/16-bit |
| Divide | `Cs==0 ? 1 : min(1, Cb / Cs)` | Divide pseudo-code inferred from "divide base by blend" |
| Darker Color | per-triplet: choose `Cb` if `ΣCb <= ΣCs` else choose `Cs` | not per channel |
| Lighter Color | per-triplet: choose `Cb` if `ΣCb >= ΣCs` else choose `Cs` | not per channel |

`Darker Color`/`Lighter Color` compare the summed channel values of the whole
base and blend colors and copy the winning color; unlike `Darken`/`Lighten` they
never produce a third, mixed color.

### Non-separable modes (W3C norm)

W3C helper functions, applied to the (non-premultiplied) RGB triplet:

```
Lum(C) = 0.3*C.r + 0.59*C.g + 0.11*C.b

ClipColor(C):
    L = Lum(C); n = min(C.r,C.g,C.b); x = max(C.r,C.g,C.b)
    if n < 0: C = L + ((C - L) * L) / (L - n)
    if x > 1: C = L + ((C - L) * (1 - L)) / (x - L)
    return C

SetLum(C, l):  d = l - Lum(C); C += d (all channels); return ClipColor(C)

Sat(C) = max(C) - min(C)

SetSat(C, s):
    # subscripts: max = largest component, min = smallest, mid = the other
    if Cmax > Cmin:
        Cmid = ((Cmid - Cmin) * s) / (Cmax - Cmin); Cmax = s
    else:
        Cmid = Cmax = 0
    Cmin = 0
    return C
```

| Mode | `B(Cb, Cs)` |
|---|---|
| Hue | `SetLum(SetSat(Cs, Sat(Cb)), Lum(Cb))` |
| Saturation | `SetLum(SetSat(Cb, Sat(Cs)), Lum(Cb))` |
| Color | `SetLum(Cs, Lum(Cb))` |
| Luminosity | `SetLum(Cb, Lum(Cs))` |

Note the W3C luma weights `0.3/0.59/0.11`; Adobe uses the same coefficients for
these four modes. Adobe's gamut mapping for out-of-range intermediate colors is
not published, so parity for these modes is necessarily tolerant.

### Dissolve

`Dissolve` is stochastic, not a color formula: for each pixel the result is either
the base or the blend color, chosen by a per-pixel pseudo-random threshold compared
against the effective opacity. Photoshop seeds a fixed noise/dither array at
startup; lowering opacity flips pixels from opaque to transparent in that pattern
rather than averaging colors, so semi-transparent Dissolve looks grainy. Bit-exact
reproduction requires reproducing the noise pattern *(inferred)*.

### Pipeline order (proposed, behavioral parity)

Per layer, bottom-to-top; each step narrows alpha before the next blends:

1. **Source build.** Layer pixels × layer mask × vector mask.
2. **Fill opacity.** Scale source alpha by `Fill`.
3. **Layer styles.** Render effects as separate passes with their own mode/opacity;
   interior effects respect `Blend Interior Effects As Group` and knockouts
   (`LAY-011`).
4. **Blend If.** For each pixel compute a blend fraction from `This Layer`
   (source-channel brightness) and `Underlying Layer` (backdrop brightness); the
   fraction interpolates between the unblended backdrop and the blend-mode result.
5. **Blend mode.** Apply `B(Cb, Cs)` per the formulas above, weighted by the
   backdrop alpha.
6. **Overall opacity.** Scale the layer's contribution by `Opacity`.
7. **Composite** source-over into the backdrop.

Groups with `Pass Through` run steps 1–7 for their children directly against the
parent backdrop; isolated groups run them into a private buffer that is then
blended as a unit.

### 8 / 16 / 32-bit behavior

| Depth | Storage | Blend arithmetic | Notes |
|---|---|---|---|
| 8-bit | `u8`, 0–255 | Normalize to [0,1], compute, round and clamp to [0,255] | Integer quantization; per-mode rounding differs from a float reference |
| 16-bit | `u16`, 0–65535 | Same normalized formulas | More headroom; still clamped for the 27 non-HDR modes |
| 32-bit (float) | `f32`, unbounded | 14 modes only; intermediate values may exceed [0,1] and are not clamped until display | Photoshop's exact 32-bit clamps for Add/Subtract etc. are undocumented |

Integer implementations should compute in a wider type (e.g. `f32` or fixed point)
and round once at write-back. Whether Adobe rounds half-up, half-to-even, or
truncates is unknown and must be calibrated.

### Color-mode behavior

- **RGB:** all 27 modes available (subject to depth).
- **CMYK:** same menu; formulas operate on ink percentages. `Hard Mix` uses
  `max = 100`; `Multiply` darkens toward the combined ink.
- **Lab:** 8 modes unavailable (list above). `Luminosity`/`Color` operate on the
  L channel and a/b; `Difference`/`Exclusion`/`Darken`/`Lighten` are disabled.
- **Grayscale:** all modes act on the single channel.
- **Bitmap/Indexed:** `Normal` only (`Threshold` label for Bitmap).

## Rust module mapping

Design proposal. Blend kernels are required by the compositor, painting, layer
styles, and smart filters, so they live in one shared, no-allocation module.

- `pictura_blend::BlendMode` — `enum BlendMode { Normal, Dissolve, Darken, ...,
  Luminosity }` mapped 1:1 to the PSD keys; `const ALL: [BlendMode; 27]`.
- `pictura_blend::separable` — per-channel `fn blend(mode, cb, cs) -> f32` for the
  separable modes, generated/table-driven so CPU and GPU share constants.
- `pictura_blend::nonseparable` — `Lum`, `ClipColor`, `SetLum`, `SetSat` over a
  `[f32; 3]` triplet; used by Hue/Saturation/Color/Luminosity.
- `pictura_blend::dissolve` — `fn dissolve(cb, cs, opacity, noise: &[u8]) -> f32`
  plus the seeded noise-tile provider.
- `pictura_blend::composite` — `fn blend_pixel(mode, cb: &[f32], cs: &[f32],
  ab: f32, as_: f32, opacity: f32, out: &mut [f32])` implementing the general
  equation; the single entry point used by `pictura_core::composite`.
- `pictura_blend::depth` — generic `PixelScalar` (`u8`/`u16`/`f32`) normalization
  and write-back with a configurable rounding policy.
- `pictura_blend::restrict` — `fn allowed(mode, color_mode, bit_depth) -> bool`
  encoding the 32-bit/Lab availability tables for UI filtering and save validation.
- `pictura_blend::blendif` — `fn blend_if_factor(this, underlying, range) -> f32`
  for step 4 of the pipeline.

GPU parity: the same `BlendMode` enum is uploaded as a uniform and dispatch selects
a kernel; non-separable modes read/write a 3-tuple. Crossing types: `BlendMode`,
`ColorMode`, `BitDepth`, `&[u8]`/`&[u16]`/`&[f32]` channel planes, `TileRef`.

## Qt6 component mapping

Widgets (consistent with `01-architecture/qt6-ui-design.md`).

- `BlendModeComboBox` (`QComboBox`) — mode list filtered by `BlendMode::allowed`;
  optional separator groups and a "Pass Through" entry for groups.
- `BlendOptionsWidget` (`QWidget`) — Advanced Blending: `Blend Interior Effects As
  Group`, `Blend Clipped Layers As Group`, `Transparency Shapes Layers`, mask
  hide-effect toggles, channel `Include` checkboxes.
- `BlendIfWidget` (`QWidget`) — two `QSlider` pairs (Gray/channel) with split
  triangles; emits `(this_lo, this_hi, under_lo, under_hi)`.
- `BlendModeBadgeDelegate` (`QStyledItemDelegate`) — draws the mode name/shortcut
  on the Layers tree; `Shift+Alt+<letter>` is handled by a `QShortcut` map.
- `NeutralColorHint` — helper feeding the New Layer dialog's neutral-color option.

## Data-model impact

- `Node.blend` is an enum mirrored by the 4-byte PSD key; unknown keys on read are
  preserved as `Other([u8;4])` so a save round-trips (per `ARCH-008`).
- `Node.opacity` and `Node.fill_opacity` are `u8` percentages; PSD stores opacity
  as 0–255, so map `round(pct * 255 / 100)` on save and `round(v * 100 / 255)` on
  load.
- Advanced blending flags are new node fields: `blend_interior_effects_as_group`,
  `blend_clipped_layers_as_group`, `transparency_shapes_layers`,
  `layer_mask_hides_effects`, `vector_mask_hides_effects`, `channel_include:
  [bool; N]`. PSD stores them in the `8BIM`/`lrFX` style blocks and the layer
  flags; unknown bits are preserved.
- `Blend If` is stored per layer as `(channel, this_lo, this_hi, under_lo,
  under_hi)` sets; PSD encodes blending ranges in the layer record.
- Undo: changing blend mode/opacity/Blend If is a scalar command with an undo
  record (`ARCH-009`); no pixel backup is needed since compositing is derived.
- `Dissolve`'s noise seed is a document/view setting, not a saved layer field;
  parity requires a stable per-document seed.

## Edge cases

- **Transparent backdrop.** Where `αb = 0`, `Cr = Cs`; blend modes have no effect
  on empty areas. Where the layer itself has `αs = 0`, the base is unchanged.
- **32-bit restriction.** The mode menu must hide the 13 unavailable modes; opening
  a file that references one in a 32-bit document is a conflict (CS6 cannot create
  it) — refuse or convert, never silently blend.
- **Lab restriction.** Same for the 8 unavailable modes; loading a file that
  stores them requires conversion to RGB or refusal.
- **Hard Mix in CMYK.** Every channel resolves to 0 or the mode maximum (100 in
  CMYK, 255 in RGB); a naive 0/1 float implementation is wrong for CMYK.
- **Subtract/Divide clamping.** Negative results clip to 0 (Subtract); Divide by
  zero (source black) must not produce `inf`/`NaN` — clamp to maximum.
- **Color Dodge/Burn denominators.** Guard `1-Cs = 0` and `Cs = 0` exactly; the
  piecewise cases exist precisely for these.
- **Darker/Lighter Color.** The comparison is on the sum of channels, not
  per-channel min/max; a per-channel implementation is a different mode.
- **Dissolve + masks/Blend If.** Dissolve's stochastic threshold composes with
  mask/Blend-If alpha; the combined result must stay binary per pixel for the
  visible/invisible decision.
- **Pass-through groups.** Do not isolate group children; adjustments inside a
  passthrough group affect the parent backdrop. Switching the group to Normal
  changes the bundle of layers seen by each child's blend mode.
- **1-pixel / empty layers.** Zero-area layers contribute nothing; blend math must
  not index out of bounds.
- **PSB / huge docs.** Blend passes stream by tile; never materialize full-doc
  float buffers.
- **GPU unavailable.** CPU reference path must produce the same decisions; GPU
  dispatch selects by mode string, with a CPU fallback.
- **Opacity vs Fill.** `Opacity` scales effects and the layer; `Fill` scales only
  the layer pixels feeding the styles. Confusing the two visibly changes shadows.

## Parity acceptance criteria

1. Given a base color `Cb` and source `Cs`, Kooka Pictura's result for each W3C
   separable mode matches the W3C formula to within ±1 LSB at 8-bit and ±2 LSB at
   16-bit, before rounding-policy calibration.
2. Given Opacity `O`, the layer's contribution is `O` of the fully-opaque result;
   given Fill `F` with a Drop Shadow, the shadow's alpha is unchanged by `F` but
   changes with `O`.
3. Given a 32-bit document, the Blend Mode menu offers exactly the 14 available
   modes; the 13 unsupported modes cannot be selected.
4. Given a Lab document, Color Dodge, Color Burn, Darken, Lighten, Difference,
   Exclusion, Subtract, and Divide are unavailable.
5. Given a group with `Pass Through`, children blend against the parent backdrop;
   switching to Normal isolates the group and changes the composite per the
   formulas within tolerance `T` from `11-cross-cutting/testing-strategy.md`.
6. Given `Blend Interior Effects As Group` selected, Color Overlay/Satin/Inner
   Glow adopt the layer's blend mode; deselected, they use their own mode. The two
   composites differ as specified.
7. Given `Blend If` `This Layer` white slider at 235, source pixels brighter than
   235 do not blend; sources at the split-slider boundary blend partially.
8. Given `Darker Color`/`Lighter Color`, the output is always one of the two input
   colors (never a new mixed color).
9. Given `Hard Mix`, every output channel is exactly 0 or the channel maximum.
10. Given `Dissolve` at 50 % opacity, output pixels are binary (base or blend)
    with ~50 % coverage; the pattern is stable across re-renders with the same
    document seed.
11. Given a PSD with a blend mode, saving and reopening preserves the exact 4-byte
    key, including modes unknown to the engine.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
  (downloaded and converted with `pdftotext -layout`; 39,104 lines) — official
  Photoshop CS6 Help. Sections used: "Blending modes / Blending mode
  descriptions" (pp. 196–197) for the 27-mode definitions, base/blend/result
  color model, the 32-bit restriction list, and the Lab/CMYK restrictions
  (p. 193, "Specify a blending mode for a layer or group"); "Group blend effects"
  and "Advanced blending" (pp. 193–194) for `Blend Interior Effects As Group`,
  `Blend Clipped Layers As Group`, `Transparency Shapes Layers`, mask-hides and
  channel exclusion; "Specify a tonal range for blending layers" (p. 194) for
  `Blend If` slider semantics; "Filling new layers with a neutral color" (p. 195)
  for the neutral-color option and the modes that lack it; "Features that support
  32-bpc HDR images" (pp. 9, 146 area) for the 32-bit mode list; "Keys for
  blending modes" (p. 82) for the `Shift+Alt+<letter>` shortcuts.
- `https://www.w3.org/TR/compositing-1/` — W3C *Compositing and Blending Level 1*
  (CR Draft, 21 March 2024). §5–§6 general/compositing equations; §10.1 separable
  blend formulas (Normal, Multiply, Screen, Overlay, Darken, Lighten, Color Dodge,
  Color Burn, Hard Light, Soft Light, Difference, Exclusion); §10.2 non-separable
  helpers `Lum`, `ClipColor`, `SetLum`, `Sat`, `SetSat` and the Hue/Saturation/
  Color/Luminosity formulas. The normative basis for these 16 modes.
- `https://en.wikipedia.org/wiki/Blend_modes` — Photoshop-mode survey. Established:
  Dissolve's per-pixel noise/dither behavior; Overlay/Hard Light/Soft Light
  relationships; the Photoshop 2012 Soft Light formula; Linear Dodge, Linear Burn,
  Vivid/Linear/Pin Light and Subtract descriptions; Divide semantics; the
  Hue/Saturation/Color/Luminosity decomposition. Secondary/community source.
- `https://web.archive.org/web/2016id_/http://www.pegtop.net/delphi/articles/blendmodes/`
  and its subpages `additive.htm` ("additive" = `a+b`, "subtractive" = `a+b-1`) and
  `burn.htm`/`quadratic.htm` — independent community formulations corroborating
  Linear Dodge/Linear Burn and the dodge/burn family. Secondary source.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — Adobe Photoshop File Formats Specification. Established the canonical layer
  blend-mode 4-byte keys (`norm`, `diss`, `dark`, `mul `, `idiv`, `lbrn`, `dkCl`,
  `lite`, `scrn`, `div `, `lddg`, `lgCl`, `over`, `sLit`, `hLit`, `vLit`, `lLit`,
  `pLit`, `hMix`, `diff`, `smud`, `fsub`, `fdiv`, `hue `, `sat `, `colr`, `lum `,
  and group `pass`) and the layer opacity/clipping/flag layout. Also used by
  `ARCH-008`.

Not fetched / not used as a primary source: `helpx.adobe.com` (documented HTTP 403
in `README.md`).

## Open questions

- **Integer rounding policy.** Adobe's 8/16-bit rounding (half-up, half-even,
  truncation) and any pre/post scaling for specific modes are closed. *Resolves
  with:* pixel-diff calibration against CS6 renders of a ramp/gradient test card.
- **Extended-mode clamps.** Exact Adobe definitions/clamps for Linear Burn, Linear
  Dodge, Vivid Light, Linear Light, Pin Light, Hard Mix, Subtract, and Divide are
  not in any standards document. The formulas here are community consensus.
  *Resolves with:* CS6 test PSDs compared against candidate kernels.
- **Subtract/Divide availability and origin.** The CS6 Help describes Subtract for
  "8- and 16-bit images", implying they are layer modes, and community sources
  disagree on whether they were added in CS5 or CS6. Whether the Layers panel
  offers them in every color mode and at 8/16-bit should be confirmed. *Resolves
  with:* a CS6 UI capture or a CS6-made PSD using `fsub`/`fdiv`.
- **Soft Light formula variant.** CS6 (2012) uses the discontinuous Photoshop
  formula, which differs from the W3C formula near `a <= 0.25`. Which one CS6
  actually ships (and whether it changed after CS6) needs confirmation. *Resolves
  with:* a CS6 render of the Pegtop soft-light test image.
- **Dissolve noise array.** The seeded pseudo-random pattern is not published.
  Exact Dissolve parity is probably a non-goal. *Resolves with:* a decision in
  `11-cross-cutting/testing-strategy.md`.
- **32-bit unclamped behavior.** How far `f32` values may exceed [0,1] before
  clamping in each 32-bit mode, and how `Hard Mix` behaves there, is undocumented.
  *Resolves with:* CS6 32-bit HDR test renders.
- **Non-separable gamut mapping.** The W3C `ClipColor` path is the standard but
  Adobe's exact out-of-gamut mapping for Hue/Saturation/Color/Luminosity is
  unverified. *Resolves with:* CS6 round-trip tests on saturated color pairs.
- **Pass-through group interaction.** Exact interaction with clipping masks and
  adjustment layers inside passthrough groups needs a controlled CS6 reference
  (also open in `ARCH-008`).
- **`Lighter Color`/`Darker Color` tie-breaking** when the two sums are equal is
  unspecified. *Resolves with:* a CS6 test with equal-luminance color pairs.
