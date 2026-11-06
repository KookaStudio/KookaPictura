# Design

## Context

`docs/04-image-ops/adjustments/color-balance.md` fixes the observable contract:
three tonal bands, three complementary axes, optional luminosity preservation,
and a per-channel numeric readout that is R/G/B on an RGB document and A/B on a
Lab document. It leaves the Lab/CMYK mapping and the tonal-band curves open and
marks them as inferred.

The engine (`pictura-adjust`) applies adjustments to a planar 8-bit
`PixelBuffer` (or a native-depth `Samples` store). The render layer always hands
it the working RGB planes; a Lab or CMYK file is converted on read and its
source mode is kept in `Document::source_mode`. `document_color_mode(doc)`
returns that source mode (or `doc.mode` for a constructed RGB/Gray document).

Issue #572's decisions, confirmed with the maintainer:

1. Map in the document's channel space, not just the readout.
2. Lab: Cyan–Red and Magenta–Green both drive `a`; Yellow–Blue drives `b`.
3. CMYK: each pair lowers its right-hand colour's ink, `K` held.
4. The dialog gains a read-only mode-aware channel row.

## Goals / Non-Goals

**Goals:**

- A Lab document's Color Balance shifts `a` and `b`; a CMYK document's shifts
  read as C/M/Y ink changes; an RGB document is byte-identical to today.
- One mode source: the visible document mode, resolved in the render layer and
  passed to the kernel. The PSD `blnc` payload stays mode-independent.
- The dialog readout names the channels the shift actually moves.

**Non-Goals:**

- Profile-mediated CMYK conversion in the adjustment kernel. The working raster
  carries no retained K plate after an edit, so a per-pixel ICC transform would
  be expensive and would still not recover the file's original ink values.
- Changing the tonal-band curves or the `Preserve Luminosity` renormalization
  for RGB.
- Enforcing the systemic colour-mode availability gate (#490) or the 32-bpc
  exclusion; those remain SYS-2.
- A GPU Color Balance path. The GPU eligibility plan does not include it.

## Decisions

### Pass the mode beside the adjustment, not inside its PSD block

Chosen: `ColorBalanceMode` is a parameter to `apply_in_mode`,
`apply_native_in_mode`, and `apply_adjustment_region`, and the render layer
derives it from `document_color_mode(doc)`.

Alternative considered: add a `mode` field to `ColorBalanceParams`. Rejected:
the `blnc` block has no mode slot, so the decoder would default it to RGB and
the Properties panel's byte-patching editor would lose it on every edit. A
document's mode can also change under an existing adjustment layer, and the
layer must follow the document, not a stale saved field.

### Convert the RGB working raster to the document space

Chosen: for Lab, convert each pixel with the shared D50 sRGB↔Lab model
(`pictura_color::model`), add the band-weighted shifts to `a`/`b`, and convert
back. For CMYK, convert to C/M/Y/K, shift the three inks, and convert back.

The RGB path keeps the existing arithmetic exactly: the band weights and the
per-channel weighted sum are factored into helpers with the same operation
order, so no RGB byte changes.

`pictura-adjust` gains a dependency on `pictura-color` for the Lab model. This
is a workspace crate, not a new external dependency; the application already
links lcms2 through the colour-management stack, and `pictura-color::model`
is the documented single source of the D50 conversion.

### Lab fold

Chosen:

```
a += cyanRed − magentaGreen
b −= yellowBlue
```

`a` is the green(−)/red(+) axis; magenta is the red side of the same axis and
cyan the green side, so the two cool/warm pairs both move `a`. `b` is the
blue(−)/yellow(+) axis, and the Yellow–Blue slider is positive toward blue, so
its sign is inverted. This gives the Help's two A/B readouts while keeping all
three sliders meaningful.

Alternative considered: make Magenta–Green inert in Lab. Rejected: the CS6
dialog keeps three bars, and the spec only says the *readout* has two values,
not that a slider stops working.

### CMYK fold

Chosen: each pair lowers its right-hand colour's ink, `K` held:

```
C −= cyanRed / 255
M −= magentaGreen / 255
Y −= yellowBlue / 255
```

The working raster has no retained K plate, so the conversion uses the
no-black-generation model (`C = 1 − R`, `M = 1 − G`, `Y = 1 − B`, `K = 0`).
That is the model in which a neutral grey has C=M=Y and a single-ink decrease
lands on red, green, or blue, as the confirmed decision requires. A
K-extraction model would leave a neutral grey's C=M=Y at zero and make the
sliders inert. The shift is therefore the RGB shift expressed in ink space; the
visible CMYK difference is the C/M/Y readout, and a future profile-aware
conversion can replace the model behind the same enum.

### Preserve Luminosity

- RGB: unchanged (Rec.601 renormalization).
- Lab: `L` is not touched, so chroma shifts preserve it by construction; no
  extra renormalization.
- CMYK: after converting back to RGB, the same Rec.601 renormalization runs, so
  a large ink shift keeps the input luminance.

### Readout

Chosen: a `QLabel` named `colorBalanceReadout` above the three bars. It formats
the active tone's values with the mode's channel names:

- RGB: `R: +40  G: 0  B: -10`
- Lab: `A: +20  B: -10` (`A = cyanRed − magentaGreen`, `B = −yellowBlue`)
- CMYK: `C: -40  M: 0  Y: +10` (the ink change, not the slider value)

The mode comes from `channels_mode(view)` (the same source the Channels panel
uses). The per-bar spin boxes stay editable; the readout is display-only, as the
spec's parameter table says.

## Risks / Trade-offs

- [The RGB kernel must not drift] -> The existing `color_balance_midtone_red_and_luminosity`
  and validation tests stay; a new test asserts `apply` and
  `apply_in_mode(Rgb)` are byte-identical.
- [Lab gamut clipping] -> `lab_to_srgb_unit` clamps out-of-gamut channels; a
  large `a`/`b` shift can clip. The model is an approximation and is marked.
- [CMYK has no K plate in the working raster] -> The no-black model is a
  deliberate ceiling (`ponytail:`), documented here and in code.
- [The native-depth path must match the 8-bit path] -> Both call the same
  `band_weights` / `band_delta` helpers and the same colour-model conversions;
  a native test covers a Lab shift.
- [The dialog's mode is read once] -> Image > Mode cannot run while a modal
  adjustment dialog is open, so the mode cannot change under it.
