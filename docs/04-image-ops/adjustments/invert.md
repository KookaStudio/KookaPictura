# Invert

- **Spec ID:** `ADJ-020`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Invert adjustment is long-standing. CS6 changed the surrounding plumbing: the CS6 "What's new" list adds "Enable Invert and Threshold adjustments for masks in 32-bit/channel images". The CS6 properties UI is the Properties panel (CS5 used the Adjustments panel).
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-008` document-model, `ARCH-009` undo-history, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `05-layers/adjustment-layers.md`, `01-architecture/color-management.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Invert** reverses every color channel. The CS6 Help defines it on a fixed
256-step scale: "When you invert an image, the brightness value of each pixel in
the channels is converted to the inverse value on the 256-step color-values
scale. For example, a pixel in a positive image with a value of 255 is changed to
0, and a pixel with a value of 5 is changed to 250."

- **Two ways to apply** (per the Help):
  - `Image > Adjustments > Invert` — destructive: "this method makes direct
    adjustments to the image layer and discards image information."
  - Adjustments panel icon (CS5) / Properties panel (CS6) → creates a
    non-destructive **Invert adjustment layer**; also
    `Layer > New Adjustment Layer > Invert`.
- **No settings.** The Help states "Inverted adjustment layers do not have
  editable settings." The adjustment is fixed at full strength; only layer
  opacity/blend mode, mask, and clipping affect it.
- **Canonical use.** The Help names it as a step in edge-mask sharpening: apply
  Find Edges, then `Image > Adjustments > Invert`, then Maximum, etc.
- **Color-negative caveat.** "Because color print film contains an orange mask in
  its base, the Invert adjustment cannot make accurate positive images from
  scanned color negatives."
- **Not a selection inversion.** `Select > Inverse` (`Shift+Ctrl+I`) is a
  different command; the same shortcut inverts a layer mask. Keep the two
  distinct in the menu model.

### Per-mode channel rule

The Help asserts per-channel inversion on the 256-step scale but does not
enumerate color modes. The rule and its per-mode consequences *(inferred where
not quoted)*:

| Mode | Channels inverted | Result / notes |
|---|---|---|
| RGB | R, G, B | Classic photographic negative. Equal RGB stays neutral, value inverted. |
| Grayscale | 1 gray channel | Inverted gray ramp. |
| CMYK | C, M, Y, K | Per-channel inverse of ink amount. Pure white `(0,0,0,0)` → `100/100/100/100`, which is black; total-ink exceeds 300 % and must be handled by the output profile. *(inferred)* |
| Lab | L, a, b | L* inverted; a* and b* represent signed chroma and are stored offset by 128, so inverting them flips hue/chroma rather than making a useful negative. Result is *not* an RGB-style negative. *(inferred)* |
| Bitmap | 1 bit | Flips 0↔1 (black↔white). *(inferred; Bitmap has no adjustment layers.)* |
| Indexed | palette index | Invert cannot operate on palette indices; expected disabled. *(inferred)* |
| Multichannel | all channels | Per-channel inverse. *(inferred)* |

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Invert` | Menu command | `Ctrl+I` / `Cmd+I` | Destructive; acts on active layer / selection |
| Adjustments panel (CS5) / Properties panel (CS6) | Panel icon | — | Creates an Invert adjustment layer |
| `Layer > New Adjustment Layer > Invert` | Menu | — | Non-destructive; shows the New Layer dialog |
| Layer palette row | Context menu | — | Blend mode, opacity, mask, clip, duplicate, merge |
| Mask / channel target | Shortcut | `Ctrl+I` | Inverts the targeted mask/channel, not a color adjustment |
| `Select > Inverse` | Menu | `Shift+Ctrl+I` | *Different* feature (selection), listed to disambiguate |

The CS6 Invert adjustment layer exposes no editable properties, so the
Properties panel shows only the standard layer controls (blend mode, opacity,
mask, clipping).

## Parameters & ranges

Invert has **no user-visible adjustment parameters**. The only controls that
shape its result are the standard adjustment-layer controls:

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Blend mode | enum | Normal | 27 CS6 modes | Non-destructive path only |
| Opacity | percent | 100 | 0–100 | Non-destructive path only |
| Layer mask | 8/16/32-bit gray | white (all) | per-pixel | Non-destructive path only |
| Clip to layer below | bool | Off | on/off | Non-destructive path only |
| Preview (dialog) | bool | On | on/off | Destructive path |

## Algorithms & pipeline

Behavioral parity for the documented 256-step case; the integer path is exact and
the float path is *(inferred)*.

### 8-bit / 16-bit integer

For a sample `v` in a channel with maximum `M = 2^b − 1` (`M = 255` at 8 bpc,
`65535` at 16 bpc):

```
out = M − v
```

This is the canonical negative and matches the Help's "255 → 0, 5 → 250"
example. It is applied independently to every color channel.

### 32-bit floating point (HDR)

32-bit HDR samples are unclamped linear floats, so "inverse on the 256-step
scale" does not directly apply. The CS6 Help lists no 32-bit Invert adjustment
layer (the 32-bpc adjustment-layer whitelist is Levels, Vibrance, Hue/Saturation,
Channel Mixer, Photo Filter, and Exposure). A plausible float rule is
`out = white_point − v` where `white_point` is the document/preview white level,
or `out = 1 − v` for normalized `[0,1]` data. **The exact Adobe rule for negative
and >1 values is not documented**; treat as behavioral parity only. *(inferred)*

### Alpha / transparency

Invert acts on color channels only; the alpha channel (transparency) and saved
selection channels are not a photographic inverse. Photoshop's layer opacity is
handled by the compositor, not by the pixel op. *(inferred)*

### Where it sits in the pipeline

- Adjustment-layer path: the Invert node contributes
  `color → M − color` (and is itself the layer content), composited with the
  usual blend/opacity/mask rules in the working space.
- Destructive path: same op applied to the active layer/selection, recorded as a
  pixel-delta history state.
- Conversion to the document color model happens at the tile boundary
  (`pictura_color`); no gamut mapping is introduced for integer modes.

## Rust module mapping

- `pictura_adjust::invert` — `InvertOp` implementing the adjustment trait;
  `fn apply_inplace(&self, tile: &mut TilePatch, depth: BitDepth)` with the
  integer and float branches above.
- `pictura_adjust::traits` — shared `Adjustment` trait, `AdjustmentKind`,
  parameter structs, and a `has_editable_settings()` query used to hide the
  Properties UI body.
- `pictura_color::sample` — `PixelValue::{U8, U16, F32}` so one kernel serves all
  depths without per-depth specialization.
- `pictura_render::adjust` — optional wgpu compute variant; CPU is the reference.

Crossing types: `InvertParams` (empty marker), `BitDepth`, `TilePatch`,
`AdjustmentId`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `InvertPropertiesWidget` | `QWidget` | Empty body; shows only "This adjustment has no options." plus the common layer controls |
| `AdjustmentPropertiesPanel` | `QWidget` stack | Hosts per-adjustment editors; Invert contributes a no-op page |
| `AdjustmentListModel` | `QAbstractItemModel` | Supplies the adjustment-layer thumbnail and name |
| `LayerMaskEditor` | `QGraphicsView` overlay | Where `Ctrl+I` inverts a mask, kept separate from the color adjustment |

Widgets (Qt Widgets) are preferred over QML for the properties panel to match the
dockable, dense CS6 panel layout already chosen for the main shell.

## Data-model impact

- **Layer node.** Non-destructive path is a new adjustment-layer node with
  `AdjustmentKind::Invert` and an empty parameter block. No new fields beyond the
  shared adjustment-layer shape.
- **Serialization.** PSD stores adjustment layers by type key; Invert has no keys
  beyond the standard layer records (blend mode, opacity, mask, clipping). XMP
  needs no adjustment-specific fields. Verify exact PSD key names in
  `01-architecture/file-formats.md`.
- **Undo.** Destructive apply = one history state of tile deltas. Adjustment-layer
  create/edit/delete = structural history records; opacity/blend/mask edits are
  separate records. The Invert op itself cannot be "re-edited", so no live
  parameter undo records.
- **Precision.** Integer path is bit-exact and losslessly invertible twice; the
  32-bit float path is not guaranteed involutive because the white point is
  view-dependent.

## Edge cases

- **Double invert** in 8/16-bit returns the exact original samples (involution);
  this must hold bit-for-bit.
- **32-bpc** — no Invert *adjustment layer* exists (per the 32-bpc whitelist);
  CS6 does allow Invert/Threshold adjustments on a **mask** at 32 bpc. The precise
  32-bpc pixel rule is undocumented.
- **CMYK** — white→`100/100/100/100` is a valid but heavy negative; total ink can
  exceed the profile limit. Do not silently gamut-map; let the output conversion
  handle it.
- **Lab** — inverting L/a/b does not produce a photographic negative; document the
  result as CS6-consistent rather than "correcting" it.
- **Indexed / Bitmap** — expect Invert disabled for Indexed; Bitmap flips bits.
  Confirm per-mode availability (open question).
- **Alpha channels / masks** — inverting a mask via `Ctrl+I` is not the same op and
  must not touch layer color pixels.
- **Selection** — destructive Invert applies only inside the active selection;
  `Select > Inverse` is unrelated.
- **Empty / 1-px documents** — op touches no pixels for empty; 1×1 must still
  invert.
- **Huge (PSB) documents** — tile-local, no full-canvas allocation.
- **GPU unavailable** — identical CPU result at higher latency.
- **Undo/redo** — adjustment-layer toggle and destructive apply each round-trip
  exactly.

## Parity acceptance criteria

- Given any 8- or 16-bpc image, `Image > Adjustments > Invert` maps every channel
  sample `v` to `M − v` exactly; applying it twice restores the original
  bit-for-bit.
- Given an RGB document, a neutral gray patch inverts to the neutral gray with the
  complementary value (e.g. 200 → 55) and keeps R = G = B.
- Given a Grayscale document, the inverted result equals the 256-step inverse of
  the gray ramp.
- Given a CMYK document, a white patch becomes the maximal-ink patch; no channel
  is left partially inverted.
- Given a selection, only the selected pixels change; pixels outside are
  untouched.
- Given an Invert adjustment layer, the Properties panel shows no editable
  parameter controls, and toggling layer visibility shows the exact before/after.
- Given a 32-bpc document, an Invert adjustment layer is not offered, and the
  documented mask-level Invert/Threshold behavior is available.
- Given a destructive apply, History records exactly one new state and undo
  restores pixels exactly at the document bit depth.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  "inverse value on the 256-step color-values scale" rule with the 255→0 / 5→250
  example; the three application paths; "Inverted adjustment layers do not have
  editable settings"; the color-negative orange-mask caveat; the CS6 "Enable
  Invert and Threshold adjustments for masks in 32-bit/channel images" note; the
  32-bpc adjustment-layer whitelist (Levels, Vibrance, Hue/Saturation, Channel
  Mixer, Photo Filter, Exposure).
- `https://99designs.com/blog/design-tutorials/how-to-use-adobes-adjustment-layers-photoshop-cs6`
  — CS6-era secondary listing Invert/Posterize/Threshold among adjustment layers
  (referenced; fetched content was used only to confirm the CS6 adjustment-layer
  set).

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Per-mode availability.** Does CS6 offer Invert for Lab, Multichannel, Bitmap,
  and Indexed documents (adjustment layer and/or destructive command)? Resolves
  with: a mode-by-mode CS6 menu capture.
- **32-bit float rule.** The exact Invert mapping for HDR values ≤ 0 or > 1 is not
  documented. Resolves with: a CS6 32-bpc pixel probe or the 32-bit preview math.
- **CMYK inversion semantics.** Whether CS6 truly inverts K independently or
  applies a separation-aware negative is unverified. Resolves with: a CMYK test
  chart.
- **Alpha handling.** Confirm that saved alpha/spot channels are not inverted by
  the color adjustment. Resolves with: a document with alpha + Invert.
- **Mask/32-bit interaction.** The exact pixel rule for mask-level Invert in
  32-bpc documents. Resolves with: a 32-bpc layer-mask test.
