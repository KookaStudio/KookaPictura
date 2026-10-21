# Channel-Based Masking

- **Spec ID:** `SEL-013`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 continues the CS5 move of mask controls into the **Properties** panel and adds the `Delete Mask` / `Disable/Enable Mask` controls there; the Channels panel, `Apply Image`, and `Calculations` dialogs are otherwise the CS5 behaviour. CS6 What's New lists no new Calculations/Apply Image options.
- **Depends on:** `08-selection/save-and-load-selections.md` (`SEL-012`), `08-selection/quick-mask.md`, `01-architecture/document-model.md`, `01-architecture/file-formats.md`, `04-image-ops/adjustments-overview.md`, `05-layers/layer-masks.md`, `05-layers/blend-modes.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

### Channels as masks

The Channels panel lists all channels of the document: the composite channel first (RGB/CMYK/Lab), then colour channels, then **alpha channels** and **spot colour channels**. Alpha channels store selections as grayscale masks. Masks are grayscale images and are editable like any image — with painting tools, editing tools, and filters. Under the default **`Masked Areas`** sense, areas painted **black are protected** and areas painted **white are editable**; `Selected Areas` inverts that reading.

Channel options (New Channel / Channel Options dialogs):

| Option | Meaning |
|---|---|
| `Masked Areas` | Masked areas black (opaque), selected areas white (transparent). Painting black increases the masked area, white increases the selected area. Quick Mask button shows a white circle on grey. |
| `Selected Areas` | Masked areas white, selected areas black; painting white increases the masked area. Quick Mask button shows a grey circle on white. |
| `Spot Color` | Converts an alpha channel to a spot colour channel; only available for existing channels. |
| `Color` | Mask overlay colour and opacity — appearance only; does not affect protection. |

Editing a channel directly: select the channel, paint on the canvas. White adds the channel's selection; black removes it; intermediate values give partial coverage. `Paint on only one channel at a time`. The channel thumbnail updates live. `Ctrl`/`Cmd`-clicking a channel thumbnail loads its boundary as a selection (see `SEL-012`); the layer/mask/filter-mask thumbnails support the same load plus Add/Subtract/Intersect modifiers (documented under "Load selections from a layer or layer mask's boundaries").

Related: Quick Mask mode creates a **temporary** channel visible in the Channels panel while active; on returning to Standard mode the mask converts back to a selection, and `Select > Save Selection` can make it permanent.

### Apply Image

`Image > Apply Image` blends one image's **layer and channel** (the source) with a layer and channel of the **active image** (the destination). Controls: `Preview`, source image/layer/channel (`Merged` for all layers), `Invert`, `Blending`, `Opacity`, `Preserve Transparency`, and an optional `Mask` (choose image, layer, and channel; channel can be any colour or alpha channel, or `Transparency` for the chosen layer's boundaries, or the active selection; plus `Invert`). If the two images have different colour modes, only a single channel (not the source's composite) can be applied to the destination's composite channel.

### Calculations

`Image > Calculations` blends **two individual channels** from one or more source images. Controls: `Preview`, Source 1 (image, layer, channel, `Invert`; `Gray` channel duplicates the effect of converting to grayscale), Source 2 (same), `Blending`, `Opacity`, optional `Mask` (image/layer/channel, or Transparency, or active selection, plus `Invert`), and `Result` = `New Document`, `New Channel`, or `Selection`. **The Calculations command cannot use composite channels.** The result can therefore directly become a saved selection/mask — this is the bridge between channel maths and selection state.

### Add and Subtract blending modes

`Add` and `Subtract` are available **only** in Apply Image and Calculations (not in the Layers panel):

- `Add` — adds corresponding pixel values; divides the sum by `Scale`, then adds `Offset`. `Black + Black = 0`, `White + anything ≥ 255 → White`. Averaging two channels = Add, `Scale = 2`, `Offset = 0`. Higher `Scale` darkens.
- `Subtract` — subtracts source pixel values from the target, then divides by `Scale` and adds `Offset`.
- `Scale` is `1.000`–`2.000`; `Offset` is `−255`…`+255` (negative darkens, positive lightens).

All other blend modes available to Apply Image/Calculations are the standard layer blend modes (see `05-layers/blend-modes.md`); the Help says the maths is applied to brightness values and that both source channels must have identical pixel dimensions.

### How these compose with selections and masks

1. A selection can be saved to an alpha channel (`SEL-012`) and then edited as a grayscale mask.
2. Calculations can combine two channels (including two alpha channels) into a **new selection**, effectively a boolean/blended selection operation beyond Add/Subtract/Intersect.
3. Apply Image can composite a channel or layer into the active document through a mask, using the active selection or an alpha channel as that mask.
4. The result of either command can be written to a channel, which is then loadable as a selection or assignable as a layer mask.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Channels` | Panel | — | Lists composite, colour, alpha, spot channels |
| Channels panel menu | Menu | — | `New Channel`, `Duplicate Channel`, `Delete Channel`, `Channel Options`, `Split Channels`, `Merge Channels`, `New Spot Channel` |
| Channels panel | Buttons | — | `Load Selection`, `Save Selection`, `New Channel`, `Delete` |
| Channels panel | Modifier-click | `Ctrl`/`Cmd` (+ `Shift`/`Alt`) + thumbnail | Load / add / subtract / intersect |
| Channels panel | Double-click | — | Channel Options dialog |
| `Image > Apply Image` | Dialog | — | Source layer/channel + destination layer/channel blend |
| `Image > Calculations` | Dialog | — | Two-channel blend to new doc/channel/selection |
| `Edit > Fill` / painting tools | — | — | Edit the active channel's grayscale |
| Quick Mask toggle | Toolbox | `Q` | Temporary channel; see `quick-mask.md` |
| `Select > Save Selection` | Dialog | — | Persist selection to channel |
| `Select > Load Selection` | Dialog | — | Load channel as selection |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Channel type | enum | alpha | colour / alpha / spot / composite | Composite not usable in Calculations |
| Mask sense | enum | `Masked Areas` | Masked Areas / Selected Areas | Swaps black/white meaning |
| Spot Color | enum action | — | convert existing alpha → spot | Existing channels only |
| Mask overlay colour | colour | red (Quick Mask default) | free | Appearance only |
| Mask overlay opacity | percent | 50 % (Quick Mask default) | 0–100 % | Appearance only |
| Apply Image Source | image/layer/channel | active document/layer | any open doc, any layer, `Merged` | Colour-mode rule for composite |
| Apply Image Invert | bool | off | on / off | |
| Apply Image Blending | enum | Normal | standard layer modes + Add/Subtract | |
| Apply Image Opacity | percent | 100 % | 0–100 % | |
| Apply Image Preserve Transparency | bool | off | on / off | |
| Apply Image Mask | enum + refs | none | channel / Transparency / selection, + Invert | |
| Calculations Source 1 / 2 | image/layer/channel | active | any open doc, any layer, `Merged`; `Gray` channel option | Composite channels forbidden |
| Calculations Result | enum | New Document? | New Document / New Channel / Selection | Default not stated |
| Add / Subtract Scale | float | 1.000 | 1.000–2.000 | Only in Apply Image / Calculations |
| Add / Subtract Offset | int | 0 | −255…+255 | Negative darkens, positive lightens |
| Max channels | int | — | 56 | Colour + alpha + spot |

## Algorithms & pipeline

### Channel compositing

All image data is reduced to per-channel brightness planes. Apply Image and Calculations evaluate, per pixel:

```text
result = Blend(dest_or_sourceA, sourceB)     # standard blend formula
result = lerp(base, result, opacity)         # opacity
result = mask ? lerp(dest, result, mask) : result   # masked application
```

`Add` and `Subtract` plug into the same blend slot with the documented arithmetic:

```text
Add:      clamp((A + B) / Scale + Offset)
Subtract: clamp((A - B) / Scale + Offset)
Add avg:  Scale = 2, Offset = 0
```

All arithmetic is per channel on the document's bit depth. The Help's values are stated on an 8-bit 0–255 scale; 16/32-bit scaling is inferred.

`Preserve Transparency` restricts the write to pixels that are opaque in the destination layer; it is the layer's alpha used as an implicit mask.

### Channel maths as a selection operation

Because Calculations can output a `Selection`, the blended channel's values become the new selection coverage. Loading a channel as a selection reads grayscale as coverage, so a high-contrast channel (from Difference, Subtract, Threshold-like combinations) yields a hard selection and a soft gradient yields a feathered one. This makes Calculations the general-purpose boolean/matte composer that sits behind many advanced masking workflows.

### Spot channels (side note)

Spot channels are extra ink plates, not selection masks, but they appear alongside alpha channels and can be edited the same way (Multichannel mode treats all channels as alpha or spot). They are serialised only in DCS 2.0 / relevant formats. Cross-ref `01-architecture/file-formats.md`.

### Mask semantics and the 50 % contour

Following the Quick Mask rule documented in the Help: when a soft mask is converted back to a selection, the boundary line runs halfway between the black and white pixels of the gradient — i.e. the 50 % coverage contour. The same rule governs loading any alpha channel.

## Rust module mapping

- `pictura_channels` — `ChannelRegistry`, `Channel { id, kind: Colour | Alpha | Spot | Composite, sense, overlay, plane }`.
- `pictura_channels::calc` — `apply_image(doc, params: ApplyImageParams)`, `calculations(docs, params: CalculationsParams, result: ResultTarget)`.
- `pictura_channels::blend` — `add(a, b, scale, offset)`, `subtract(a, b, scale, offset)`, and a dispatch to the shared blend-mode table in `pictura_layers::blend`.
- `pictura_selection::from_channel` — `channel_to_selection(channel, sense, invert) -> CoverageMask`; the shared 50 % contour rule.
- `pictura_selection::mask` — `apply_to_layer_mask(mask, layer)`.
- `pictura_psd` — alpha/spot channel records, channel order, names, sense.

Data crossing the boundary: `ChannelId`, `DocumentId`, `BlendMode`, `ResultTarget { NewDocument, NewChannel, Selection }`. Channel planes remain in the core; Qt gets thumbnails and previews only.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ChannelsPanel` / `ChannelsModel` | `QDockWidget` / `QAbstractItemModel` | Shared with `SEL-012`; channel list, visibility, thumbnails, modifier-click |
| `ChannelOptionsDialog` | `QDialog` | Masked Areas / Selected Areas / Spot Color / colour+opacity |
| `ApplyImageDialog` | `QDialog` | Source image/layer/channel, Invert, Blending, Opacity, Preserve Transparency, Mask group |
| `CalculationsDialog` | `QDialog` | Two sources, Blending, Opacity, Mask, Result target |
| `MaskPreviewWidget` | `QWidget` | Live preview of the result; wired to Preview checkboxes |
| `ChannelThumbnailDelegate` | `QStyledItemDelegate` | Live grayscale thumbnail; drag source |
| `PropertiesPanel` (mask controls) | `QDockWidget` | CS6 mask density/feather/invert/delete/enable (cross-ref `05-layers/layer-masks.md`) |

Widgets over QML: dialogs and the channels list are dense, precise, keyboard-driven controls; widgets match `ARCH-003`. The `Properties` panel may later move to QML, but not before the channel/model contracts stabilise.

## Data-model impact

- **Channel node:** `Channel { id, kind: Colour | Alpha | Spot | Composite, sense: MaskedAreas | SelectedAreas, name, overlay: {color, opacity}, plane: Plane }`. Documents hold colour channels (implicit from colour mode) plus a growable alpha/spot list capped at 56.
- **PSD/PSB:** alpha and spot channels serialise in the alternate-channel data; channel name, sense, and order must round-trip. DCS 2.0 preserves spot channels. See `01-architecture/file-formats.md`.
- **Undo granularity:** editing a channel, Apply Image, and Calculations each produce one history state. Channel-pixel edits require tile snapshots; `Result = Selection` touches only selection state.
- **Layer masks / filter masks:** stored as alpha channels per the Help; loading their boundaries is a selection source. Cross-ref `05-layers/layer-masks.md`, `05-layers/vector-masks-and-clipping-masks.md`.
- **Blend modes:** Add/Subtract are *not* layer blend modes; they live in the calculations namespace so they never appear in the Layers panel.
- **No XMP:** channel data is document pixels, not metadata.

## Edge cases

- **Composite channels in Calculations:** explicitly forbidden; the UI must exclude them.
- **Dimension mismatch:** Apply Image requires matching pixel dimensions for the source image to appear; Calculations sources must match. Cross-mode images may only share a single channel, not a composite.
- **No mask on Apply Image / Calculations:** the operation applies everywhere.
- **`Result = Selection` with no existing selection:** the result becomes the selection; with an existing selection, clarify combine behaviour — the Help states the result is placed in a selection but not how it combines. Open question.
- **Bitmap/Indexed:** alpha channels and calculations may be unavailable; no colour channels in the usual sense for Indexed. Verify per mode.
- **CMYK / Lab / Multichannel:** channel names and counts differ; in Multichannel all channels are alpha or spot. Calculations and Apply Image follow the same per-channel maths.
- **16/32-bit:** Add/Subtract scale/offset documented on 0–255 must be rescaled; 32-bit float must not clamp to `[0,1]` unless the mode says so.
- **`Preserve Transparency`:** only meaningful when the destination layer has transparency.
- **Spot channel conversion:** converting alpha → spot is one-way via Channel Options; splitting/merging with spot channels is prohibited (spot becomes an alpha on merge).
- **56-channel cap:** New Channel / Duplicate must refuse.
- **PSB / huge docs:** channel planes multiply memory; tiling and a budget required.
- **GPU unavailable:** previews fall back to CPU; thumbnails must still render.
- **Undo/redo:** channel pixel edits must be losslessly reversible.

## Parity acceptance criteria

- Given an alpha channel with `Masked Areas` sense, painting white increases the loaded selection and painting black decreases it.
- Given the same channel loaded as a selection, the marching-ants contour sits at 50 % channel brightness within 0.5 px.
- Given two channels and `Calculations` with `Result = New Channel`, the new channel equals the blended result; with `Result = Selection`, the same values load as selection coverage.
- Given `Add` with `Scale = 2`, `Offset = 0`, the output is the per-pixel average of the two channels within 1/255 (8-bit).
- Given `Subtract`, higher `Scale` reduces the output magnitude and a positive `Offset` lightens, matching the Help.
- Given a source image of different pixel dimensions, it is unavailable in the Apply Image / Calculations source list.
- Given `Calculations` with a composite channel as a source, the UI forbids the selection.
- Given `Preserve Transparency` on a layer with transparent regions, those regions are unchanged.
- Given `Mask` set to the active selection, the blend is applied only where the selection covers, with coverage as strength.
- Given a `Selected Areas` channel, loading it produces the inverse selection of an equivalent `Masked Areas` channel.
- Given an image saved to PSD with alpha and spot channels, reopening preserves their kind, name, sense, and order.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (curl → `/tmp`, `pdftotext`). Established: about channels (colour/alpha/spot, up to 56, preserved formats); Channels panel overview, show/hide, colour channels in colour, select/edit a channel, rearrange/rename, delete; channel options (Masked Areas, Selected Areas, Spot Color, Color); create/edit alpha channel masks; painting rules; "Channel calculations" chapter — Blending layers and channels; `Apply Image` full control list; `Calculations` full control list including `Result`; Add/Subtract definitions with Scale 1.000–2.000 and Offset ±255; "Add and Subtract… available only for Apply Image and Calculations"; load selections from layer/layer-mask/filter-mask boundaries with modifier combinations; Quick Mask temporary channel and 50 % boundary rule; duplicate/split/merge channels; mask controls in the CS5 Masks / CS6 Properties panel.
- `https://search.brave.com/search?q=Photoshop+%22Grow%22+%22Similar%22+Magic+Wand+tolerance+default+32` — search results page (context for tolerance-based selection, used in SEL-010).

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).
- Adobe's PSD file-format spec (not yet fetched; required for the channel-serialisation details).

## Open questions

- **`Result = Selection` combine behaviour.** The Help says the blended result is placed in a selection but not whether it replaces, adds, subtracts, or intersects an existing selection. Resolve with a CS6 test.
- **Calculations `Result` default.** The default target (New Document vs New Channel vs Selection) is not stated. Resolve from a CS6 UI capture.
- **High-bit-depth Scale/Offset scaling.** The documented 0–255 arithmetic must be mapped to 16/32-bit; Adobe does not state the exact scaling. Resolve with a high-bit-depth reference.
- **Apply Image colour-mode rule.** "Single channel but not composite" across differing modes is stated but the exact validation matrix is not. Resolve per colour-mode pair in `01-architecture/color-management.md`.
- **Spot channel serialisation.** DCS 2.0 and PSD spot-channel encoding details must come from the Adobe file-format spec, not the Help PDF.
- **Mask sense on PSD round-trip.** Whether the `Masked Areas` / `Selected Areas` flag is stored independently or derived must be confirmed against the file-format spec.
- **CS6 Properties-panel parity.** The exact CS6 mask controls (Density, Feather, Delete Mask, Disable/Enable) need a dedicated treatment in `05-layers/layer-masks.md`; confirm the CS5→CS6 UI move before asserting it as CS6 behaviour.
