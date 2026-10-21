# Duotone

- **Spec ID:** `IMG-007`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Duotone mode and the Duotone Options dialog are carried forward from CS5 unchanged in the CS6 Help.
- **Depends on:** `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/file-formats.md` (`ARCH-011`), `07-color-painting/color-picker.md`, `10-workflow-io/printing.md`, `08-selection/channel-based-masking.md`.

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

Duotone mode is `Image > Mode > Duotone`. "Duotone" in Photoshop covers
**monotone, duotone, tritone, and quadtone** — grayscale images printed with one
to four inks. Source: CS6 reference, "Duotones" and "Duotone mode".

- Monotones use a single non-black ink; duotones/tritones/quadtones use two,
  three, or four inks. Colored inks reproduce tinted grays and increase tonal
  range (a press reproduces only about 50 gray levels per ink).
- Because different inks reproduce different gray levels, Photoshop treats a
  duotone as a **single-channel, 8-bit grayscale image**. In Duotone mode the
  individual channels are not directly accessible; they are manipulated through
  curves in the **Duotone Options** dialog.
- **Only 8-bit grayscale images can be converted to duotones.** The workflow is
  `Image > Mode > Grayscale` first, then `Image > Mode > Duotone`.
- Applying a duotone effect to only part of an image: convert to Multichannel mode
  (duotone curves become spot channels) and erase part of the spot channel.

### Duotone Options dialog

1. `Preview`.
2. `Type`: `Monotone`, `Duotone`, `Tritone`, `Quadtone`.
3. For each ink, click the color box (solid square) to open the color picker, then
   `Color Libraries` to choose an ink book and color. The Help's tip: "To produce
   fully saturated colors, specify inks in descending order—darkest at the top,
   lightest at the bottom."
4. Click the curve box next to each ink to adjust its **duotone curve**.
5. Set **overprint colors** if necessary; `OK`.
6. `Save` / `Load` store curves, ink settings, and overprint colors; Photoshop
   ships sample duotone/tritone/quadtone sets.

### Duotone curves

- Each ink has a curve mapping each **grayscale value in the original image** to
  a **specific ink percentage**.
- Default is a straight diagonal: a 50% midtone renders at a 50% tint, 100%
  shadow at 100% ink.
- The graph's horizontal axis runs highlights (left) → shadows (right); ink
  density increases upward.
- Up to **13 points** per curve; intermediate values are interpolated.
- Percentage text boxes accept the tint (e.g. `70` in the 100% box means 70%
  tint for the 100% shadow).
- `Save`/`Load` in the Duotone Curve dialog exchanges curves with the Curves
  dialog (including Arbitrary Map curves).
- The **Info panel** can show ink percentages: set the readout to `Actual Color`
  to see the ink percentages that will print.

### Overprint colors

- Overprint colors are two **unscreened** inks printed on top of each other
  (e.g. cyan over yellow → green).
- `Overprint Colors` opens a dialog showing how combined inks will look; click a
  combination's swatch and choose a color.
- This changes **only the on-screen display** of overprint colors, not the print
  result; calibrate the monitor first.

### Viewing separations, printing, export

- `Image > Mode > Multichannel` converts the image so each ink becomes a spot
  color channel; the composite preview may be less accurate than Duotone mode.
  Undo returns to Duotone mode.
- Printing: no CMYK conversion is required — choose `Separations` from the
  Profile menu in the Print dialog's Color Management section.
- Export to page-layout apps requires **EPS or PDF**; if spot channels exist,
  convert to Multichannel and save as **DCS 2.0**. Custom colors need the correct
  suffix so the importing app recognises them.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Mode > Duotone` | Menu + dialog | none documented | Requires 8-bit Grayscale first. |
| Duotone Options dialog | Dialog | — | Type combo, ink color boxes, curve boxes, `Overprint Colors`, `Save`/`Load`, `Preview`. |
| Duotone Curve dialog | Dialog | — | Up to 13 points; percentage fields; `Save`/`Load`. |
| Overprint Colors dialog | Dialog | — | Per-combination swatches → Color Picker. |
| `Image > Mode > Color Table` / Channels | Menu / panel | — | Multichannel spot channels after conversion. |
| Info panel | Panel | — | `Actual Color` readout shows ink percentages. |
| `File > Print` | Dialog | `Ctrl+P` | `Separations` profile. |
| `File > Save As` | Dialog | — | EPS/PDF; DCS 2.0 with spot channels. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Type | combo | Monotone | Monotone / Duotone / Tritone / Quadtone | Number of active inks 1–4. |
| Ink color | color + library | black *(inferred)* | Color Picker / Color Libraries (PANTONE, TOYO, DIC, HKS, …) | Descending dark→light for saturation. |
| Duotone curve | curve editor | straight diagonal | up to 13 points; 0–100% ink | Per ink. |
| Curve percentage boxes | numeric | from curve | 0–100% | Ink tint for a given gray. |
| Overprint colors | color swatches | System-calculated *(inferred)* | Color Picker per combination | Display only. |
| Preview | bool | off *(inferred)* | on / off | |
| Info readout | enum | — | Actual Color / others | Shows ink %. |

## Algorithms & pipeline

- **Ink mapping.** For each pixel, the stored 8-bit gray value `g ∈ [0,255]`
  indexes each ink's curve: `p_i = curve_i(g)` yields ink *i*'s coverage in
  percent. The on-screen composite is a model of overprinting the ink tints.
- **Curve** is a monotone piecewise interpolation through up to 13 control
  points (default identity). The Help states values are interpolated between
  specified points.
- **Overprint display** is a lookup controlled by the Overprint Colors dialog
  (a display-time mapping, not a pixel value).
- When converted to **Multichannel**, each ink becomes a spot channel whose
  pixel values are the ink percentages; the spot-color model then composites.
- Converting to **CMYK** converts custom colors to their CMYK equivalents; the
  Help notes duotones do not need to be converted to CMYK to print separations.

The exact overprint compositing is a printing-ink model and is not documented:
**behavioural parity only, algorithm TBD** for the on-screen composite; the
**curve mapping is deterministic and testable**.

## Rust module mapping

Proposed:

- `pictura-core::color::duotone` —
  `DuotoneType { Mono, Duo, Tri, Quad }`,
  `InkSpec { color: SpotColor, curve: Curve13 }`,
  `DuotoneSpec { type_, inks: Vec<InkSpec>, overprint: OverprintTable }`.
- `fn ink_percentages(gray: u8, spec: &DuotoneSpec) -> [f32; 4]`.
- `fn duotone_to_multichannel(doc) -> MultichannelDoc`.
- `Curve13` — fixed-capacity curve type shared with `adjustments/curves.md`.
- `DuotoneCommand` implements `EditCommand`; raw duotone blob round-trips
  untouched.

## Qt6 component mapping

- `DuotoneOptionsDialog` (`QDialog`) — `Type` combo, per-ink `ColorSwatchButton`
  + `CurveButton`, `Overprint Colors…`, `Save`/`Load`, `Preview`.
- `DuotoneCurveDialog` — 13-point curve widget (shared with Curves), percentage
  spin boxes, `Save`/`Load`.
- `OverprintColorsDialog` — grid of ink-combination swatches opening the shared
  color picker.
- `InkReadout` — Info-panel mode for `Actual Color` ink percentages.

## Data-model impact

- Document color mode = **Duotone (mode 8)** with bit depth 8; single channel.
- The Duotone specification is stored in the PSD **Color Mode Data Section**;
  per the PSD spec the format is **not documented** and Photoshop recommends
  other apps treat duotone as grayscale and **preserve the blob** on round-trip.
- Related PSD image resources: `1014` Duotone halftoning, `1017` Duotone
  transfer functions, `1018` Duotone image information, `1066` Alternate Duotone
  Colors (not read/used by Photoshop).
- Undo: mode conversions are single commands; leaving Duotone mode is lossy and
  stores a pre-image (or the reverse mode conversion), per `ARCH-009`.
- Multichannel conversion creates spot-channel nodes (`ARCH-008`).

## Edge cases

- **Only 8-bit grayscale** can convert to duotone; 16/32-bit must first convert
  to 8-bit Grayscale. Converting a 16/32-bit image directly is blocked.
- **Layers**: duotone is single-channel, so conversion from a layered image
  flattens (as with Bitmap/Indexed/Multichannel per the Help's mode-conversion
  rule); keep a layered backup.
- **Grayscale with no duotone spec** opens as ordinary grayscale.
- **Overprint display only**: users may be misled if the monitor is uncalibrated;
  the Help warns about this.
- **Multichannel round-trip**: once edited in Multichannel, the original duotone
  state is not recoverable except via the History panel.
- **Export**: without EPS/PDF (or DCS 2.0 for spot channels) inks may not
  transfer; naming/suffix matters.
- **PSB**: preserve the opaque duotone blob.
- **32-bit / HDR**: not applicable (duotone is 8-bit).

## Parity acceptance criteria

1. Given an 8-bit Grayscale image, `Image > Mode > Duotone` with Type = Duotone
   and default curves produces two inks whose percentages equal the gray value
   mapped through each curve; a 50% gray yields 50% of each ink by default.
2. Given a custom 13-point curve, ink percentages at the control points match
   the entered values exactly, with interpolation between them matching the CS6
   curve within tolerance.
3. Given the default diagonal curve, converting to Multichannel produces spot
   channels whose values equal the original gray values.
4. Given a saved duotone settings set, `Load` restores inks, curves, and
   overprint colors; ink percentages match.
5. Given a 16-bit Grayscale image, the Duotone command is unavailable until the
   image is converted to 8-bit Grayscale.
6. Given a duotone PSD saved and reopened, the duotone blob round-trips
   byte-for-byte and the on-screen result is unchanged.
7. Given a duotone printed with `Separations`, each ink appears as its own
   separation (verifiable via a separation proof).
8. `Image > Mode > Multichannel` followed by `Edit > Undo` restores Duotone mode
   and the original curves.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, "Duotones": "About duotones" (mono/tri/quad, tonal
  range, single-channel 8-bit treatment), "Convert an image to duotone" (8-bit
  grayscale requirement, Type, Color Libraries, descending inks, curves,
  overprint, Multichannel partial effect), "Modify the duotone curve for a given
  ink" (identity default, up to 13 points, interpolation, percentage boxes,
  Save/Load, Info Actual Color readout), "Specifying overprint colors" /
  "Adjust the display of overprint colors", "Saving and loading duotone
  settings", "View the individual colors of a duotone image" (Multichannel spot
  channels), "Printing duotones" (Separations), "Exporting duotone images"
  (EPS/PDF/DCS 2.0); "Duotone mode" in "Color modes".
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD spec: mode enum `Duotone = 8`; Color Mode Data Section "Duotone images:
  color data contains the duotone specification (the format of which is not
  documented)"; resources 1014/1017/1018/1066.

## Open questions

- **Duotone blob format.** The PSD Color Mode Data layout is undocumented by
  Adobe. *Resolves with:* analyzing a CS6 duotone PSD, or acceptance
  of opaque round-trip only.
- **Exact curve interpolation** (linear between points vs spline). *Resolves
  with:* CS6 curve tests at intermediate gray values.
- **On-screen overprint compositing model** (multiply/transmittance/Lab).
  *Resolves with:* CS6 pixel tests on known ink combinations.
- **Default ink colors and initial curve** for a fresh Duotone dialog. *Resolves
  with:* CS6 screenshot.
- **Whether layers are always flattened** on conversion to Duotone. *Resolves
  with:* CS6 multi-layer test.
- **DCS 2.0 / EPS ink naming conventions** accepted by CS6. *Resolves with:*
  format experiments.
- **Info-panel ink percentage precision** in 16-bit-like displays. *Resolves
  with:* CS6 observation.
