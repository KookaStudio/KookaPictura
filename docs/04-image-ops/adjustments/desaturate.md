# Desaturate

- **Spec ID:** `ADJ-030`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Desaturate command predates CS6 and is carried forward unchanged in the CS6 Help.
- **Depends on:** `04-image-ops/adjustments-overview.md`, `04-image-ops/adjustments/hue-saturation.md`, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `05-layers/adjustment-layers.md`.

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

`Image > Adjustments > Desaturate` converts a color image to grayscale values
**while leaving the document in the same color mode**. Source: CS6 reference,
"Desaturate colors".

- In an RGB image it assigns equal red, green, and blue values to each pixel;
  the pixel's lightness is preserved.
- It is **destructive**: it permanently alters the original image information in
  the affected layer. The Help states it "has the same effect as setting
  Saturation to -100 in the Hue/Saturation adjustment".
- For non-destructive work the Help directs the user to a **Hue/Saturation
  adjustment layer** instead.
- In a multi-layer image, **only the selected layer** is converted.
- Desaturate is grouped in the Help under "Applying special color effects to
  images" alongside Invert, Threshold, Posterize, and Gradient Map.

The Help does not document the numeric mapping per color mode beyond the RGB
statement. The RGB result is widely publicly documented as
`v = (min(R,G,B) + max(R,G,B)) / 2` per pixel (community source; *(inferred)*),
which is exactly "set S = 0 in HSL, keep L". It is **not** a luminance-weighted
average (0.299/0.587/0.114); see `## Open questions`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Desaturate` | Menu command | `Ctrl+Shift+U` / `Cmd+Shift+U` *(inferred; not in the CS6 shortcut tables)* | Applies immediately; no dialog. |
| Adjustments / Properties panel | — | — | Desaturate has **no** adjustment-layer icon; the Help points to Hue/Saturation for that. |
| Document canvas | — | — | Applies to the active layer, or to the active selection if one exists (general adjustment rule). |

## Parameters & ranges

`None.` Desaturate is the only color adjustment with no dialog and no
user-visible parameters. The effective behaviour is a fixed S = −100.

## Algorithms & pipeline

Documented contract:

- Equalise the channel values so the pixel becomes neutral, preserving the
  Help-stated "lightness value".
- Equivalent to `Hue/Saturation` with `Saturation = -100`.

Per-mode mapping (the Help is explicit only for RGB):

| Mode | Proposed / observed behaviour | Confidence |
|---|---|---|
| RGB | `v = (min + max) / 2` written to R, G, B (HSL L with S = 0) | Community analysis *(inferred)* |
| Grayscale | No chroma present; effectively a no-op | *(inferred)* |
| Lab | `L` unchanged, chroma channels `a = b = 0` | *(inferred)* |
| CMYK | Not documented. Community reports a tinted (brownish) neutral when desaturating CMYK, which argues the operation does **not** simply zero C/M/Y. | Unverified — `## Open questions` |
| Bitmap | 1-bit, already two-valued; command unavailable or no-op | *(inferred)* |
| Indexed / Duotone / Multichannel | Image adjustments are restricted in these modes; likely dimmed/unavailable | *(inferred)* |

Bit depth: the 32-bpc feature matrix (see `04-image-ops/32-bit-hdr.md`) lists
only `Levels, Exposure, Hue/Saturation, Channel Mixer, Photo Filter` as
supported adjustments, so Desaturate is **not available in 32-bit/channel
documents**. *(inferred from omission; verify.)*

Adobe's exact CMYK/Lab numerics are closed: **behavioural parity only, algorithm
TBD** for non-RGB modes.

## Rust module mapping

Proposed:

- `pictura-core::adjust::desaturate` — `fn desaturate(buffer: &mut PixelBuffer)`
  dispatching on the buffer's `ColorMode`; the RGB arm computes
  `(min+max)/2` in the working space.
- `pictura-core::adjust::desaturate::DesaturateCommand` — implements
  `EditCommand`; targets one layer or the active selection.
- Boundary types: `PixelBuffer` (`U8 | U16 | F32`), `ColorMode`, `Selection`.

## Qt6 component mapping

- `ImageAdjustmentsMenu` — a `QAction` for Desaturate with no dialog; dispatches
  the command to the active layer/selection.
- No widget in the Adjustments/Properties panel (matching CS6).
- Undo/redo goes through the shared `HistoryController`; the action must be
  disabled when `bit_depth == 32` or the mode is unsupported, matching the
  proposed Rust guard.

## Data-model impact

- Destructive pixel edit on one layer; no new node type and no adjustment-layer
  serialization.
- Undo: one history state per invocation. Because the operation is deterministic
  and lossy, the lazy record is a tile diff (or pre-image for the affected
  region) rather than a reversible transform descriptor (`ARCH-009`).
- Selection: if a selection is active, only selected pixels change; the command
  otherwise edits the whole layer.

## Edge cases

- **32-bit/channel**: command disabled (not in the supported-adjustment list).
- **CMYK/Lab**: exact neutralisation numerics unknown; CMYK output may not be a
  true neutral gray under the active profile.
- **Grayscale/Bitmap**: no chroma; command should behave as a no-op or be
  dimmed.
- **Indexed / Duotone / Multichannel**: adjustments are restricted; disable.
- **Empty / 1-px documents**: trivially handled; no special casing.
- **PSB / huge documents**: operate per tile; never allocate a whole-document
  copy beyond the undo record.
- **Already-neutral pixels**: `min == max`, so the value is unchanged — a useful
  invariant for tests.
- **Active selection feathered/partial**: applies per pixel; no blend of the
  shift beyond the selection mask (the command is a hard substitution).

## Parity acceptance criteria

1. Given an 8-bit RGB pixel `(12, 104, 22)`, Desaturate yields `(58, 58, 58)`
   (within ±1 LSB) for every channel.
2. Given a neutral pixel `(r, r, r)`, Desaturate leaves it unchanged exactly.
3. Given pure white `(255,255,255)` and pure black `(0,0,0)`, both are
   unchanged.
4. Given a multilayer document, only the active layer's pixels change.
5. Given an active selection, pixels outside the selection are unchanged.
6. Given a 32-bit/channel document, the command is disabled.
7. Given an image edited by Desaturate and by `Hue/Saturation` with
   `Saturation = -100`, the two results are identical within ±1 LSB (the Help's
   equivalence claim).
8. Desaturate adds exactly one history state and is reversible by Undo.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, section "Desaturate colors": same-color-mode conversion,
  equal RGB assignment, lightness preservation, destructive layer edit,
  H/S −100 equivalence, Hue/Saturation adjustment-layer alternative, selected-
  layer-only behaviour; 32-bpc supported-adjustment list under "Features that
  support 32-bpc HDR images"; Desaturate also appears in the Sponge tool
  saturation description.
- `https://web.archive.org/web/20231207073531/https://stackoverflow.com/questions/9320953/what-algorithm-does-photoshop-use-to-desaturate-an-image`
  — community analysis of the RGB formula as `(min+max)/2`; explicitly
  noted as **not** the luminance-weighted average. Secondary/community source.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD color-mode enumeration and depth values (context for mode/depth gating).

## Open questions

- **Exact RGB formula.** Is `(min+max)/2` the real CS6 implementation, or an
  approximation from newer versions? *Resolves with:* a controlled CS6 test over
  a color cube.
- **CMYK behaviour.** What neutral does CS6 produce when Desaturating a CMYK
  image, and does it route through the CMYK working space? *Resolves with:* a
  CS6 CMYK test target plus profile inspection.
- **Lab behaviour.** Whether CS6 zeroes `a`/`b` and keeps `L`, or converts
  through another space. *Resolves with:* a CS6 Lab test image.
- **Availability per mode.** The exact set of color modes where the command is
  enabled (Bitmap/Indexed/Duotone/Multichannel) is not documented. *Resolves
  with:* CS6 UI inspection.
- **Keyboard shortcut.** `Ctrl+Shift+U` is not present in the CS6 Help shortcut
  tables fetched here. *Resolves with:* the CS6 keyboard-shortcut reference or a
  UI capture.
- **Selection semantics at the edges.** Whether partially selected pixels are
  substituted or blended is unverified. *Resolves with:* a feathered-selection
  test on CS6.
