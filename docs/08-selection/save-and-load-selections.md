# Save & Load Selections

- **Spec ID:** `SEL-012`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Save/Load Selection dialogs and channel shortcuts are unchanged from CS5; the CS6 What's New list touches selections only for decimal feather values and the Make Selection feather recall (see `SEL-010` / `SEL-014`).
- **Depends on:** `08-selection/selection-model.md`, `08-selection/channel-based-masking.md`, `01-architecture/file-formats.md`, `01-architecture/document-model.md`, `ARCH-007` undo-history, `05-layers/layer-masks.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

A selection can be persisted as a grayscale **alpha channel** and reloaded later, including into another image. The CS6 Help frames this as the durable counterpart to Quick Mask: any selection can be saved as a mask in a new or existing alpha channel and later reloaded from that mask.

### Saving

Two entry points:

- **Channels panel `Save Selection` button** — creates a new channel immediately, named according to the sequence in which channels are created (`Alpha 1`, `Alpha 2`, …; the default-name sequence is community-reported, not in the Help text).
- **`Select > Save Selection`** — opens a dialog with:

  | Control | Meaning |
  |---|---|
  | `Document` | Destination image. Default: the active image (saved to a channel). May be another open image **with the same pixel dimensions**, or a `New` image. |
  | `Channel` | Destination channel. Default: a new channel. May be any existing channel in the selected image, or a **layer mask** if the image has layers. |
  | `Name` | Channel name, shown when saving to a new channel. |
  | Combine operation | When saving into an existing channel: `Replace Channel`, `Add to Channel`, `Subtract From Channel`, `Intersect With Channel`. |

After saving, selecting the channel in the Channels panel displays the selection as a grayscale mask.

Saving a selection to an existing channel is a boolean combination against that channel's current contents: Replace overwrites, Add unions, Subtract removes, Intersect keeps the overlap.

### Loading

Two entry points:

- **`Select > Load Selection`** dialog:

  | Control | Meaning |
  |---|---|
  | `Document` | Source image. Default: the active image. |
  | `Channel` | Channel containing the selection to load. |
  | `Invert` | Loads the non-selected areas instead. |
  | `Operation` | `New Selection`, `Add To Selection`, `Subtract From Selection`, `Intersect With Selection` — how to combine with an existing selection. |

- **Channels panel** shortcuts (Help-sourced):

  | Action | Windows | Mac OS |
  |---|---|---|
  | Load channel as selection | `Ctrl`-click channel thumbnail | `Cmd`-click channel thumbnail |
  | Add to current selection | `Ctrl+Shift`-click thumbnail | `Cmd+Shift`-click thumbnail |
  | Subtract from current selection | `Ctrl+Alt`-click thumbnail | `Cmd+Option`-click thumbnail |
  | Intersect with current selection | `Ctrl+Shift+Alt`-click thumbnail | `Cmd+Shift+Option`-click thumbnail |

  Also: select the alpha channel, click the `Load Selection` button, then click the composite colour channel near the top of the panel; or drag the channel onto the `Load Selection` button.

To load a selection from another image, that image must be open and the destination image must be active. A selection can be dragged from one open Photoshop image into another.

### Storage formats

Alpha channels are grayscale masks with the same pixel dimensions as the document. The Help states an image may have **up to 56 channels** total (colour + alpha + spot). Alpha channels are preserved only when saving in **Photoshop (PSD/PSB), PDF, TIFF, or raw** formats; saving in other formats may discard them. **EPS does not support alpha channels** (it does support clipping paths — see `SEL-014`). In PSD/PSB, alternate/alpha channels are stored in the documented image-resource/extra-channel layer data (see `01-architecture/file-formats.md`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Select > Save Selection` | Dialog | — | Document, Channel, Name, combine operation |
| `Select > Load Selection` | Dialog | — | Document, Channel, Invert, Operation |
| Channels panel | Button | — | `Save Selection` (new channel); `Load Selection` (load from active alpha) |
| Channels panel | Modifier-click | `Ctrl`/`Cmd` + click thumbnail | Load as selection |
| Channels panel | Modifier-click | `Ctrl/Cmd+Shift`, `Ctrl/Cmd+Alt`, `Ctrl/Cmd+Shift+Alt` | Add / subtract / intersect |
| Channels panel menu | Menu | — | `New Channel`, `Duplicate Channel`, `Delete Channel`, `Channel Options` |
| Channels panel | Drag | — | Drag channel to `Load Selection`; drag selection between images |
| `Select > Deselect` | Menu | `Ctrl+D` | |
| `Select > Reselect` | Menu | `Ctrl+Shift+D` | Restores the most recent selection when no mask is saved |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Save Document | enum | active image | active image / any open same-size image / New | New-image option creates a single-channel multichannel image |
| Save Channel | enum | new channel | new / existing channel / layer mask | Layer mask only if the image has layers |
| Channel Name | string | `Alpha 1`, `Alpha 2`, … (community) | free text | Sequence-based default |
| Save operation | enum | Replace existing channel contents | New (implicit) / Replace / Add / Subtract / Intersect | Only when an existing channel is targeted |
| Load Document | enum | active image | active image / any open same-size image | |
| Load Channel | enum | first alpha? | document's alpha channels | Exact default not stated |
| Load Invert | bool | off | on / off | |
| Load Operation | enum | New Selection | New / Add / Subtract / Intersect | |
| Max channels | int | — | 56 (colour + alpha + spot) | Sourced |
| Channel options (mask sense) | enum | Masked Areas | Masked Areas / Selected Areas / Spot Color | See `SEL-013` |
| Mask overlay colour / opacity | colour + percent | red, 50 % (Quick Mask default) | free | Affects appearance only (SEL-013) |

## Algorithms & pipeline

### Mask representation

A saved selection is a single-channel `CoverageMask` stored as 8-bit (or 16/32-bit) grayscale. `White` means selected/editable and `black` masked/protected under the default `Masked Areas` channel option; `Selected Areas` swaps the sense. Loading a channel converts the grayscale back into selection coverage in `[0,1]`, so a feathered channel loads as a soft selection and the marching-ants contour is drawn at the 50 % level (exactly as Quick Mask does when converted back).

### Save / load operations

Saving to an existing channel is a per-pixel combine in coverage space:

```text
Replace:   dst = src
Add:       dst = max(dst, src)          # union of coverage
Subtract:  dst = dst * (1 - src)        # or max(dst - src, 0)
Intersect: dst = min(dst, src)          # product / min
```

Loading is the same combine against the live selection mask instead of a channel, plus an optional `1 - channel` inversion before combining. Adobe does not publish whether Add/Subtract use max/clamped-subtract or an arithmetic blend; the clamp-to-`[0,1]` behaviour is the natural reading and matches mask compositing. Mark inferred.

### Cross-image handling

Both dialogs require matching pixel dimensions for cross-document Save/Load (the Help states this for `Document` choices). Dragging a selection between images is a separate, dimension-flexible path (community behaviour) and should be modelled explicitly rather than folded into the dialog rule.

## Rust module mapping

- `pictura_selection::alpha` — `save_selection(doc, mask, target: ChannelRef, combine: Combine)`, `load_selection(doc, source: ChannelRef, invert: bool, combine: Combine)`.
- `pictura_selection::Combine` — enum `{ Replace, Add, Subtract, Intersect }` shared with `SEL-013` and the channel calculations result modes.
- `pictura_channels::AlphaChannel` — grayscale plane + metadata (`name`, `sense: MaskedAreas | SelectedAreas`, overlay colour/opacity).
- `pictura_channels::store` — channel registry with the 56-channel cap and creation-order naming.
- `pictura_psd` — serialise/parse alternate (alpha) channels in PSD/PSB; enforce the "preserved formats" matrix.
- Data crossing the boundary: `ChannelId`, `ChannelRef { document, id }`, and a `CoverageMask` for preview. Alpha channel pixel data stays in the core/tile store.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SaveSelectionDialog` | `QDialog` | Document/Channel/Name, operation radio group; validates same-size documents |
| `LoadSelectionDialog` | `QDialog` | Document/Channel, Invert check, Operation radio group |
| `ChannelsModel` | `QAbstractItemModel` | Channel list (composite, colour, alpha, spot), visibility, thumbnails, creation-order names |
| `ChannelsPanel` | `QDockWidget` | Panel UI; `Save Selection` / `Load Selection` / `New Channel` / `Delete` buttons; modifier-click routing |
| `ChannelThumbnailDelegate` | `QStyledItemDelegate` | Renders the live channel thumbnail; drag source for load/between-image drags |

Widgets over QML: the Channels panel is a dense list with live thumbnails and modifier-click semantics — a `QAbstractItemView` + delegate is a better fit than a QML list, consistent with `ARCH-003`.

## Data-model impact

- **New node type:** `AlphaChannel { id, name, sense, overlay, pixels: Plane }` sibling to colour and spot channels in the document's channel registry.
- **PSD/PSB:** alpha channels serialise in the extra-layer/alternate-channel data; the spec must preserve channel `name`, `sense`, and order. `Masked Areas` / `Selected Areas` semantics affect interpretation on reload. See `01-architecture/file-formats.md`.
- **Document field:** a "current selection" (`CoverageMask`, optional) distinct from any saved channel; Load replaces or combines it.
- **Undo granularity:** each Save/Load is a history state. Loading changes only document selection state, so it is cheap to undo; saving into an existing channel modifies channel pixels and must snapshot the prior channel.
- **Layer masks:** saving a selection into a layer mask converts coverage to grayscale and writes it into the mask channel (Help: `Channel` may be a layer mask).
- **Preferences:** no document state; creation-order naming and default overlay live in app preferences.

## Edge cases

- **No selection:** Save Selection is unavailable; Load Selection with `New Selection` still works if a channel is chosen.
- **No alpha channels:** Load dialog shows only colour/spot channels or is disabled; Save defaults to new channel.
- **Channel count at 56:** New Channel/Save must refuse with a clear error.
- **Cross-image size mismatch:** dialog must exclude same-dimension-incompatible documents; the drag path is the exception.
- **Layer mask target:** only valid when the image has layers; Background-only documents exclude it.
- **Sense mismatch:** loading a `Selected Areas` channel must invert relative to a `Masked Areas` channel so the visible selection is correct.
- **Soft / feathered channel:** loads as fractional coverage; contour at 50 %. A channel whose maximum coverage is below 50 % yields the Help's "No pixels are more than 50 % selected" condition.
- **Format loss:** saving to JPEG/GIF/etc. silently discards alpha channels — the UI should warn as Photoshop does.
- **16/32-bit:** alpha channels carry the document bit depth; 32-bit float masks must support fractional coverage beyond 8-bit resolution.
- **CMYK/Lab/Multichannel:** colour channel count differs; in Multichannel all channels are alpha or spot (see `SEL-013`).
- **PSB / huge docs:** channel storage doubles/quadruples memory; enforce a budget.
- **Undo/redo:** Save into existing channel must be losslessly reversible.

## Parity acceptance criteria

- Given an active selection and `Select > Save Selection` with a new channel, a new alpha channel appears whose grayscale equals the selection coverage (white = selected under the default `Masked Areas` sense) within 1/255.
- Given `Select > Load Selection` with `Invert` checked, the loaded selection is the complement of the channel within 1 px of the contour.
- Given an existing selection and `Load Selection` operation `Add` / `Subtract` / `Intersect`, the result matches the corresponding mask combine.
- Given `Ctrl`-click on an alpha channel thumbnail with no existing selection, the channel loads as a New Selection.
- Given an existing selection, `Ctrl+Shift` / `Ctrl+Alt` / `Ctrl+Shift+Alt`-click reproduce Add / Subtract / Intersect identically to the dialog operations.
- Given a document saved to PSD with two alpha channels, reopening restores both channels with their names, order, and sense.
- Given a save to JPEG, alpha channels are absent and the user is warned.
- Given a target image of different pixel dimensions, it is unavailable in the Save/Load `Document` list.
- Saving a selection to a layer mask populates the layer mask with the selection's grayscale.
- New alpha channels created by the panel button are named `Alpha 1`, `Alpha 2`, … in creation order (community-sourced default; verify against CS6).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (curl → `/tmp`, `pdftotext`). Established: "Save and load selections" procedure and dialogs (Document/Channel/Name + Replace/Add/Subtract/Intersect; Document/Channel/Invert + Operation); Channels-panel save/load buttons and all modifier-click combinations; cross-image size rules; masks stored in alpha channels, where areas painted black are protected and white editable; up to 56 channels; alpha channels preserved only in Photoshop/PDF/TIFF/raw; EPS does not support alpha channels; loading a selection into a layer mask; Quick Mask → `Save Selection` persistence; duplicate/split/merge channel procedures; channel-options definitions.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+%22Save+Selection%22+default+channel+name+Alpha+1` — search results page; surfaced the community claim of the `Alpha 1` default channel name.

Consulted as search-result snippets only (not individually fetched; community-reported):

- `http://www.sketchpad.net/channels2.htm` — states that Photoshop gives a new channel the default name `Alpha 1`.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Channels_palette.html` — Channels panel ordering and Save Selection behaviour.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).

## Open questions

- **Save combine arithmetic.** Replace/Add/Subtract/Intersect on soft coverage is not numerically specified. Resolve by comparing soft-channel saves against CS6.
- **Load default channel.** The Load dialog's default `Channel` selection is not in the fetched text. Resolve from a CS6 UI capture.
- **Exact default-name sequence.** `Alpha 1`, `Alpha 2`, … is community-reported; confirm numbering after deletions/reloads.
- **Cross-image drag semantics.** Rescaling and sense handling when dragging a selection between differently sized documents is undocumented. Resolve with a CS6 behavioural study.
- **PSD alternate-channel encoding details.** The precise PSD resource keys and sense flags must be pinned in `01-architecture/file-formats.md` against the published Adobe file-format spec, not the Help PDF.
- **Layer-mask destination bit depth.** Whether saving into a layer mask honours 16/32-bit precision is unstated. Resolve against a high-bit-depth CS6 document.
- **Save/Load Selection in 32-bpc.** The Help restricts many selection commands at 32 bpc; whether Save/Load itself is restricted is unclear. Resolve with a CS6 32-bit test.
