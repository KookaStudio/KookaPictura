# Color Models

- **Spec ID:** `CLR-001`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the models and the Adobe Color Picker's model set are long-standing. CS6 changes are confined to picker input details (paste `#`/`0x` hex, 3-digit shorthand; resizable Color panel) documented in `CLR-002`.
- **Depends on:** `ARCH-007` color-management, `ARCH-008` document-model, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `04-image-ops/color-profiles-and-assignment.md`, `07-color-painting/color-picker.md`, `07-color-painting/swatches-and-libraries.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

Photoshop distinguishes three related ideas that the CS6 Help separates explicitly:

- **Color model** — "a method (usually numeric) for describing color" (RGB, CMYK, HSB, Lab).
- **Color space** — "a variant of a color model" with a specific gamut (e.g. within RGB: sRGB, Adobe RGB, ProPhoto RGB).
- **Color mode** — the per-document setting that determines "which color model is used to display and print the image" and therefore the number of channels, number of colors, tools, and file formats available.

The Help classifies models as **device-dependent** (RGB, HSL, HSB, CMYK — each can map to many different color spaces) versus **device-independent** (CIE L\*a\*b\*, a fixed color space tied directly to human color perception). Color management uses Lab as the reference for translating between spaces (`ARCH-007`).

### Foreground and background colors

Photoshop uses the **foreground color** to paint, fill, and stroke selections and the **background color** to make gradient fills and fill erased areas. Defaults are **foreground black, background white**; in an alpha channel the defaults reverse, with foreground white and background black. The colors are chosen via the Eyedropper tool, the Color panel, the Swatches panel, or the Adobe Color Picker.

### The four picker models

The Adobe Color Picker offers four color models — HSB, RGB, Lab, and CMYK. Selecting a color displays the numeric values for **HSB, RGB, Lab, CMYK, and hexadecimal simultaneously**, which shows how the different models describe the same color.

| Model | Definition (CS6 Help) | Components and ranges |
|---|---|---|
| **HSB** | "Based on the human perception of color"; describes hue, saturation, brightness. | Hue = location on the color wheel, **0°–360°**; Saturation = "amount of gray in proportion to the hue", **0% (gray) – 100% (fully saturated)**; Brightness = "relative lightness or darkness", **0% (black) – 100% (white)**. |
| **RGB** | Additive primaries (light); the model monitors use. | R, G, B each **0–255** in 8-bpc terms ("0 is no color, and 255 is the pure color"); equal components = neutral gray, all 0 = black, all 255 = white. |
| **Lab (CIE L\*a\*b\*)** | Device-independent, "based on the human perception of color"; color management's reference space. | **L 0–100**; **a (green–red axis) and b (blue–yellow axis) −128 … +127**. |
| **CMYK** | Subtractive process inks used by printers. | C, M, Y, K each a **percentage** of process ink; pure white = all four at 0%. |

The Help's color-model primer also defines **additive** (RGB) and **subtractive** (CMYK) primaries and the **color wheel**, including the rule that a color can be increased/decreased either directly or by adjusting its complement (e.g. magenta is opposed by green).

### Color modes (document-level)

| Mode | Channels | Values | Notes |
|---|---|---|---|
| **RGB Color** | 3 | 0–255 per channel at 8 bpc | Default for new documents; 3×8 = 24-bit/pixel → up to **16.7 million** colors; 48-bit (16 bpc) and 96-bit (32 bpc) hold more. Varies by working space. |
| **CMYK Color** | 4 | percentage of each process ink | Every pixel gets a percentage for each process ink; bright red ≈ 2% C, 93% M, 90% Y, 0% K; pre-press oriented; varies by working space. |
| **Lab Color** | 3 | L 0–100; a, b −128 … +127 (in picker/Color panel) | Device-independent; used by the CMS as a reference. |
| **Grayscale** | 1 | 8-bit: 0 (black) – 255 (white) | Also expressible as black-ink percentage (0% = white, 100% = black); 16/32-bit have far more shades; range defined by the Gray working space. |
| **Bitmap** | 1 | black or white only | 1-bit per pixel. |
| **Indexed Color** | 1 | up to 256 colors via CLUT | 8-bit; closest color or dithering for out-of-table colors. |
| **Duotone** | 1 + inks | monotone/duotone/tritone/quadtone | One to four custom inks over a grayscale image. |
| **Multichannel** | n | 256 gray levels/channel | Specialized printing; layers flattened. |

Grayscale, RGB, CMYK, Lab, and Multichannel support 16-bpc; the Help's 16-bit support list names exactly those. Bitmap mode is 1 bpc; Indexed and 32-bit images cannot be converted to Multichannel.

### Bit-depth representation

- **8 bpc** — integer, 256 levels per channel (`2^8`). Grayscale 8-bpc = 256 grays; RGB 8-bpc = 24-bit/pixel.
- **16 bpc** — integer, higher precision; supported in Grayscale, RGB, CMYK, Lab, and Multichannel; all toolbox tools except Art History Brush, and all color/tonal adjustment commands except Variations.
- **32 bpc** — **floating-point**, HDR; HDR luminance is stored as 32-bit floating-point values; 32-bpc is the only depth that stores the entire HDR range. HDR has its **own Color Picker** (see `CLR-002`) with 32-bit float RGB fields.

### Gamut and web-safe warnings

- **Out-of-gamut / non-printable** — colors in RGB/HSB/Lab with no CMYK equivalent trigger a warning alert triangle, with a swatch below it showing the closest CMYK equivalent; clicking the triangle substitutes the closest printable color. Which colors are printable is defined by the current CMYK working space set in the Color Settings dialog box.
- **Not web-safe** — the web-safe palette is **216 colors** (a subset of the Mac OS 8-bit palettes); selecting a non-web color shows "an alert cube"; clicking it "select[ing] the closest web color."
- **Info panel** — in CMYK readout, "an exclamation point appears next to the CMYK values" when the sampled color is out of the printable gamut (see `CLR-004`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Adobe Color Picker | Dialog | — | Four models; simultaneous HSB/RGB/Lab/CMYK/hex values (`CLR-002`) |
| `Window > Color` — Color panel | Panel | — | Sliders per model; color ramp; out-of-gamut / non-web alerts |
| `Window > Swatches` | Panel | — | Stores and applies colors (`CLR-003`) |
| `Window > Info` | Panel | `F8` | Per-mode readouts and gamut warning (`CLR-004`) |
| `Image > Mode` | Menu | — | Convert document mode and bit depth |
| `Edit > Color Settings` | Dialog | `Ctrl/Cmd+Shift+K` | Working spaces define gamuts (`ARCH-007`) |
| Toolbox fg/bg boxes | Controls | `D` default, `X` swap | Model-independent color carriers |
| HDR Color Picker | Dialog | — | 32-bpc float entry; shown only for 32-bpc documents |
| `View > Gamut Warning` | Toggle | `Ctrl/Cmd+Shift+Y` | Masks out-of-gamut pixels (`ARCH-007`) |

Color panel slider models (Help: "Choose a Sliders option from the Color panel menu"): **RGB / CMYK / Grayscale / HSB / Lab / Web Color Sliders** *(the exact slider list is inferred from the CS6 Color panel menu; the Help names only RGB Spectrum, CMYK Spectrum, Grayscale Ramp, Current Colors, Make Ramp Web Safe for the ramp)*.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Model selector (picker) | radio | HSB | H / S / B, R / G / B, L / a / b, CMYK fields | Picker field shows the modeled pair (`CLR-002`) |
| H (hue) | degrees | 0 | 0 … 360 | Color-wheel angle |
| S (saturation) | percent | 100 | 0 … 100 | 0 = gray |
| B (brightness) | percent | 100 | 0 … 100 | 0 = black, 100 = white |
| R, G, B | int | 0/0/0 | 0 … 255 | 8-bpc encoding |
| L | int/float | 0 | 0 … 100 | Lab luminance |
| a, b | int | 0 | −128 … +127 | Green–red / blue–yellow |
| C, M, Y, K | percent | 0 | 0 … 100 | Device/working-space dependent |
| Hex `#` | hex string | 000000 | `00`–`ff` per RGB pair | CS6 accepts `#`/`0x` paste and `#123` shorthand |
| Bit depth | enum (document) | 8 Bits/Channel | 8 / 16 / 32 Bits/Channel | 32 = float HDR |
| Color mode | enum (document) | RGB Color | Bitmap / Grayscale / Indexed / RGB / CMYK / Multichannel / Duotone / Lab | Determines channels and available features |
| Only Web Colors | bool | off | on / off | Picker; snaps to 216-color palette |
| On-screen/web alert | action | — | click alert cube | Substitutes closest web color |
| Gamut alert | action | — | click triangle | Substitutes closest CMYK equivalent |

## Algorithms & pipeline

Adobe's exact numeric transforms are closed; behavioral parity only. The following is the reference model *(inferred* where not quoted*)*.

### Model conversions

- **HSB ↔ RGB** — standard cylindrical transform (H∈[0,360), S,B∈[0,1]; B is value, so this is HSV/HSB, not HSL). Adobe's help text defines hue/saturation/brightness; the round-trip must be deterministic and monotonic. *(inferred)*.
- **RGB ↔ Lab** — through the D50 CIELAB connection space per ICC/CIE; the picker's Lab fields are the device-independent meaning of the current RGB value. Uses the same connection space as the CMS (`ARCH-007`). Adobe's exact white-point and rounding are not documented; parity is tolerance-based.
- **RGB/CMYK ↔ working space** — a color's numeric values have no absolute meaning; they are defined only inside the color space of the device that produces them. CMYK values are therefore produced by a profile transform from the RGB working space (and back), not by a fixed formula. Exposed as the out-of-gamut alert, whose threshold is the current CMYK working space.
- **Hex ↔ RGB** — hex `#rrggbb` is exactly the 8-bit RGB triple.

### Picker display model

The picker holds one authoritative color (arbitrary precision, derived from the model the user is editing) and projects it into all five readouts on every change. Editing any readout re-derives the others through the conversion graph above. **The reference value should be stored in the document/display space, with the other models computed on demand**, so repeated edits in one model do not accumulate round-trip error through another.

### Warning evaluation

- **Gamut** — predicate `rgb → cmyk(working space) → rgb'`, compare per-channel or ΔE; failing units are flagged. Uses the same transform machinery as `ARCH-007`.
- **Web-safe** — each channel must be one of `{0, 51, 102, 153, 204, 255}` (the 216-color cube; *inferred* encoding, the Help states only "216 colors").

### Bit-depth behavior

- At 8 bpc, model conversions quantize to 0–255.
- At 16 bpc, integer precision must be preserved (no silent 8-bit round-trips).
- At 32 bpc, all intermediates are `f32` and unclamped; HDR values may exceed 1.0, and the ordinary picker is replaced by the HDR picker.

## Rust module mapping

- `pictura_color::model` — `ColorModel { Hsb, Rgb, Lab, Cmyk }`, `PixelValue::Hsb(Hsb) | Rgb(Rgba8|Rgba16|Rgba32) | Lab(Lab) | Cmyk(Cmyk)`, `HexColor`.
- `pictura_color::convert` — `rgb_to_hsb`, `hsb_to_rgb`, `rgb_to_lab`, `lab_to_rgb`, and profile-mediated `cmyk_from_rgb` / `rgb_from_cmyk`; all `f32` internally, quantized only at the boundary.
- `pictura_color::gamut` — `is_out_of_gamut(rgb, &CmykProfile)`, `closest_cmyk(...)`; reuses `pictura_color::transform` from `ARCH-007`.
- `pictura_color::websafe` — 216-color predicate and nearest-web-color snap (shared with the picker and Color panel).
- `pictura_color::mode` — `DocumentColorMode`, `DocumentBitDepth`, channel count, and the `supports(bit_depth, feature)` capability matrix.
- `pictura_core::command` — `SetForegroundColor` / `SetBackgroundColor` / `ConvertDocumentMode` commands; the last is a destructive history record.

Crossing types: `ColorModel`, `PixelValue`, `DocumentColorMode`, `DocumentBitDepth`, `ColorProfileId` (`ARCH-007`).

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ColorModelSwitcher` | `QButtonGroup` (radio) | H/S/B, R/G/B, L/a/b, CMYK model selection |
| `ColorNumericGrid` | `QGridLayout` of `QSpinBox`/`QDoubleSpinBox` | Per-model numeric fields with clamping and unit suffixes |
| `ColorRampWidget` | custom `QWidget` | Color ramp; spectrum options; web-safe ticks; alert icons |
| `GamutAlertLabel` | `QLabel` | Out-of-gamut triangle + closest-CMYK swatch; click to apply |
| `WebSafeAlertLabel` | `QLabel` | Non-web alert cube; click to snap |
| `ColorValueModel` | `QObject` | Authoritative color; converts and broadcasts all readouts atomically |

Widgets over QML: dense, keyboard-numeric, docked controls consistent with the shell decision in `ARCH-003` (`01-architecture/qt6-ui-design.md`).

## Data-model impact

- `Document` carries `color_mode: DocumentColorMode` and `bit_depth: DocumentBitDepth` (already owned by `ARCH-008`/`ARCH-007`). The models here do not add document fields; they describe conversions over channel data.
- Channel buffers are stored in the document mode/depth; per-model values are UI-state projections, not persisted per pixel.
- **Undo:** `Image > Mode` conversions are destructive and store the pre-conversion image (as in `ARCH-007`). Foreground/background color changes are session/UI state, not document undo.
- **Serialization:** PSD stores the mode and depth in the file header; the picker's current model choice is a preference, not document data. Named swatches and spot books are covered by `CLR-003`.
- **Spot colors** are not a model above; they live in spot channels and are printed as CMYK except in Duotone mode (same rule as `CLR-002`).

## Edge cases

- **8/16/32-bit** — 32-bpc is float; must not clamp to [0,1] or quantize. 16-bpc must not route through an 8-bit conversion. HDR uses the HDR picker.
- **CMYK in non-CMYK modes** — CMYK readouts in the picker/Info panel are working-space simulations; changing the CMYK working space changes the mapping without changing RGB pixels.
- **Lab round-trip** — repeated RGB→Lab→RGB edits must not drift; store one authoritative value.
- **Out-of-gamut** — clamped only on explicit user action (clicking the alert) or on a real mode conversion; never silently.
- **Web-safe** — only enforced when `Only Web Colors` is on or the user clicks the alert; otherwise non-web colors are legal.
- **Bitmap / Indexed** — no continuous model; the picker and most adjustments are restricted, and 8-bit Grayscale must precede Bitmap conversion.
- **Grayscale** — saturation/hue are not representable; a Grayscale document's picker works in gray levels (inferred from the Color panel's Grayscale Ramp).
- **Multichannel** — layers flatten; color model is per-channel grayscale.
- **Missing/untagged profile** — conversions fall back to the working space but must not tag the file (`ARCH-007`).
- **GPU unavailable** — CPU conversion is the reference; GPU LUT sampling may differ within the parity tolerance.
- **1-px / empty / PSB** — no model-specific issue; conversions stream by tile and must not allocate full-canvas float buffers.

## Parity acceptance criteria

1. Given an RGB document, entering an RGB triple in the picker displays matching CSS-legal hex, and re-entering that hex restores the same RGB within one 8-bit step.
2. Given HSB `(0°, 100%, 100%)`, the picker reports RGB `(255, 0, 0)`.
3. Given a saturated RGB color outside the current CMYK working space, the gamut alert appears and clicking it substitutes the same closest CMYK equivalent as the Color panel's alert.
4. Given a color whose channels are not all in `{0,51,102,153,204,255}`, `Only Web Colors` off shows the alert cube; with it on, every selectable color is one of the 216 web-safe colors.
5. Given an 8-bpc RGB document, the picker's RGB range is 0–255; given 16-bpc, the same color round-trips without an 8-bit intermediate (no banding on a fine ramp).
6. Given a 32-bpc document, the ordinary picker is replaced by the HDR picker with 32-bit float RGB fields and an Intensity slider.
7. Given a CMYK readout in the Info panel over an out-of-gamut pixel, the CMYK values carry the exclamation warning.
8. Given `Image > Mode > Lab Color` on an RGB document, Lab values match the CMS round-trip within the `ARCH-007` ΔE tolerance.
9. Given an alpha channel, the default foreground is white and background is black; in the composite document they are black and white respectively.
10. Given a mode conversion, one History state is created and undo restores the prior mode and pixel data.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded, text-extracted). Established: the foreground/background color rules and alpha-channel defaults; the four picker models HSB/RGB/Lab/CMYK and simultaneous readouts including hex; HSB component definitions and ranges; RGB 0–255; Lab L 0–100 and a/b −128…+127; CMYK percentages; the additive/subtractive primaries, color wheel and complement rule; the color model/space/mode distinction and device-dependent vs device-independent classification; the RGB/CMYK/Lab/Grayscale/Bitmap/Duotone/Indexed/Multichannel mode descriptions and channel counts; RGB 24-bit/16.7-million-colors and 48/96-bit notes; the bit-depth definitions (8/16/32 bpc, 32 = floating-point HDR); the 16-bpc support list; the out-of-gamut triangle and closest-CMYK-equivalent behavior tied to the CMYK working space; the 216 web-safe colors and alert cube; the Info panel CMYK exclamation warning.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — same file, "Productivity enhancements (JDI's) in CS6 → Color Picker": hex paste with `#`/`0x` and `#123` shorthand; also "Resizable Color panel."

Not fetched (HTTP 403 from this environment; used only as pointers): `helpx.adobe.com` color-model pages linked from the CS6 PDF.

## Open questions

- **Exact Color panel slider model list** (RGB / CMYK / Grayscale / HSB / Lab / Web Color Sliders) is *(inferred)* from the panel menu; the CS6 Help names only the ramp options. *Resolves with:* a CS6 Color panel capture or the archived help page.
- **Web-safe cube encoding** — the Help states 216 colors and "a subset of the Mac OS 8-bit color palettes" but not the channel set. The `{0,51,…}` cube is the standard web-safe mapping but is *(inferred)* here. *Resolves with:* the CS6 Color panel web-safe tick capture.
- **Whether CMYK fields are directly editable and drive the field/slider**, versus being a read-only projection. The Help states that CMYK values can be entered as percentages or set with the color slider and color field, which suggests editable. *Resolves with:* a CS6 picker capture.
- **Exact RGB↔Lab and gamut-predicate numerics** (white point, ΔE threshold, clamping) are closed. *Resolves with:* pixel-level CS6 tests and the ICC spec.
- **16-bpc picker limits** — whether the picker exposes 16-bit or only 8-bit fields. *Resolves with:* a CS6 16-bpc capture.
- **Grayscale picker behavior** — whether the ordinary picker is shown or restricted. *Resolves with:* a CS6 Grayscale-mode capture.
- **PSD spot-color semantics** beyond "prints to CMYK in every mode except Duotone" (see `CLR-003`). *Resolves with:* `01-architecture/file-formats.md` and PSD documentation.
