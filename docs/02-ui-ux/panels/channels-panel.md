# Channels Panel

- **Spec ID:** `PAN-002`
- **Status:** `Draft`
- **Parity tier:** `Core`.
- **New in CS6:** `Changed` — CS6 enabled **Split Channels** for documents with a single layer (so simple transparency can be split). The panel's channel model (color/alpha/spot channels, eye column, thumbnail options, panel menu) is otherwise CS5-era.
- **Depends on:** `SEL-004` quick-mask, `SEL-012` save-and-load-selections, `SEL-013` channel-based-masking, `IMG-004` image-modes, `IMG-007` duotone, `IMG-008` indexed-color, `CLR-003` swatches-and-libraries (spot color pickers), `LAY-004` layer-masks (masks are stored as alpha channels), `ARCH-008` document-model, `ARCH-009` undo-history, `PAN-004` history-panel, `10-workflow-io/printing.md`.

> Crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Selection semantics and mask/alpha-channel behavior
> are owned by the `08-selection/` specs; this file specifies the **panel widget**
> and the channel operations invoked from it.

## CS6 behavior

The **Channels panel** (`Window > Channels`) lists every channel in the image, with the composite channel first for RGB, CMYK, and Lab images. A thumbnail of the channel contents sits left of the name and updates automatically as the channel is edited. Channel kinds:

- **Color channels** — created automatically from the document color mode (e.g. RGB = R, G, B plus the composite channel used for editing).
- **Alpha channels** — store selections as grayscale images; used as masks.
- **Spot color channels** — additional plates for printing with spot inks.

An image can have up to **56 channels**. All channels have the same pixel dimensions as the image. Channel data is compressed by some formats; alpha channels survive only in Photoshop, PDF, TIFF, PSB and raw, and DCS 2.0 preserves only spot channels (see `10-workflow-io/` and `ARCH-008`).

**Default placement (CS6 Essentials workspace).** The panel is grouped in the **bottom-right Layers tab set**, tabbed with **Layers** and **Paths**; Layers is the default-active tab and Channels/Paths sit behind it. (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*.)

### Visibility and viewing

Clicking the eye column shows/hides a channel; dragging through the eye column toggles multiple channels. Clicking the composite channel shows all default color channels, and the composite is displayed whenever all color channels are visible. Individual channels display in grayscale; in RGB/CMYK/Lab they can be shown in color (Lab: only a and b) via `Edit > Preferences > Interface > Show Channels in Color` (Mac: `Photoshop > Preferences > Interface`). If more than one channel is active, channels always appear in color.

### Selecting and editing

One or more channels can be selected; selected/active names are highlighted. `Shift`-click selects/deselects multiple. Painting tools paint only **one channel at a time**: white adds the channel color at 100% intensity, gray at lower intensity, black fully removes it. A temporary **Quick Mask** channel appears while in Quick Mask mode, but mask editing is done in the image window (`SEL-004`).

### Channel options, alpha and spot channels

`New Channel` is invoked from the panel bottom button or the panel menu; `Alt`/`Option`-clicking the button opens the options dialog. `Channel Options` (double-click the thumbnail, or the menu) exposes: **Name**, **Color Indicates** (`Masked Areas` / `Selected Areas`, with the Quick Mask button glyph changing accordingly), **Color** (a color field and opacity for the mask overlay — display-only, it does not change protection), and **Spot Color** (converts an alpha channel to a spot color channel; only for existing channels). A new channel's overlay **defaults to red at 50 % opacity** (community/CS6-era sources; the Help describes the options but not their defaults).

A new alpha channel is the only visible channel until the composite eye is enabled, at which point the mask shows as a color overlay. Painting white removes masked areas, black adds them, and an opacity below 100% (options bar, or painting with a color) produces partial values.

**Spot channels.** A spot channel is created with `Ctrl`/`Cmd`-click the New Channel button, or `New Spot Channel` from the panel menu. Its dialog adds **Solidity** (0–100 %), which simulates on-screen ink density (100 % = fully covering, e.g. metallic; 0 % = transparent, e.g. varnish); Solidity and the color choice affect only on-screen previews and composite prints, not printed separations. An existing alpha channel is converted to a spot channel via `Channel Options > Spot Color`. `Merge Spot Channel` (panel menu) merges a spot channel with the color channels, splitting the spot ink into its CMYK components.

### Selections ↔ channels

- **Save a selection**: click the **Save Selection** button at the bottom of the panel (creates a new alpha channel named by sequence), or `Select > Save Selection` to choose Document, Channel, Name and combine mode (`Replace Channel`, `Add to Channel`, `Subtract From Channel`, `Intersect With Channel`). An existing alpha/spot channel can receive the selection (`SEL-012`).
- **Load a selection**: `Ctrl`/`Cmd`-click the channel thumbnail; or `Select > Load Selection` (Document, Channel, Invert, Operation). Combine with the current selection using the channel-thumbnail modifier keys (`Control`/`Command` + `Shift`/`Alt`/both).

### Duplicate, split, merge

- **Duplicate Channel**: select the channel, choose `Duplicate Channel` from the panel menu, name it, choose a destination Document (same-pixel-size documents, or `New` for a one-channel multichannel image), optionally `Invert`. Dragging the channel onto the `Create New Channel` button duplicates within the image. Dragging a channel into another document's window, or select-all/copy/paste, duplicates across images (across-image duplication does not require equal dimensions; the pasted channel overwrites the target).
- **Split Channels** (`Channels` panel menu) works on **flattened images only** (CS6 relaxed this to single-layer documents, e.g. simple transparency). The original file closes and each channel becomes a separate grayscale image window titled with the original filename plus the channel name.
- **Merge Channels** combines multiple open, flattened, same-pixel-size grayscale images into one image. The number of open grayscale images determines the available modes (3 → RGB, 4 → CMYK, etc.); an incompatible channel count falls back to Multichannel. Choosing `Multichannel` walks per-channel via `Next`. The merged image opens untitled; the source images close unchanged. A spot-color image cannot be split and recombined — the spot channel is added as an alpha channel.

### Rearranging, renaming, deleting

Alpha and spot channels can be reordered by dragging; spot colors overprint top-to-bottom in panel order. Alpha/spot channels can move above the default color channels only in **Multichannel** mode (`IMG-004`). Rename by double-clicking the channel name. Delete via `Alt`-click the Delete icon, drag the name to the Delete icon, or the panel menu (confirm). Deleting a **color** channel from a layered file flattens visible layers and discards hidden ones (removal converts the image to Multichannel, which has no layers); deleting an alpha, spot or quick-mask channel does not flatten.

## UI surface

| Location | Type | Shortcut (Win / Mac) | Notes |
|---|---|---|---|
| `Window > Channels` | Menu → panel | — | Display the panel. |
| Panel menu | Menu | — | `New Channel`, `Duplicate Channel`, `Delete Channel`, `New Spot Channel`, `Merge Spot Channel`, `Split Channels`, `Merge Channels`, `Channel Options`, `Panel Options`. (Save/Load Selection live on the bottom buttons and the `Select` menu, not here.) |
| Composite channel row | Toggle | `Ctrl+2` / `Cmd+2` | Shows all color channels. |
| Color channel rows | Select | `Ctrl+3/4/5` / `Cmd+3/4/5` | R/G/B; legacy `Ctrl+1/2/3` via Keyboard Shortcuts preference. |
| Channel row → eye | Toggle | drag through eye column | Show/hide one or many. |
| Channel row → thumbnail | Load selection | `Ctrl`/`Cmd`-click | Add: `+Shift`; subtract: `+Alt`/`+Option`; intersect: `+Shift+Alt`. |
| Alpha channel row | Select + rubylith | `Shift`-click | Toggles selection and rubylith overlay. |
| Alpha/spot thumbnail | Options dialog | double-click | Name, Color Indicates, Color, Spot Color. |
| Save Selection button | Button | `Alt`/`Option`-click | Opens Save Selection options. |
| New Channel button | Button | `Ctrl`/`Cmd`-click | Creates a **spot** channel. |
| Delete icon | Button | `Alt`/`Option`-click | Delete without confirmation. |
| Quick Mask | Toolbox toggle | `~` (tilde) | Toggles composite/grayscale mask view (`SEL-004`). |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Channel thumbnail | enum | medium | None / small / medium / large | `Panel Options`; `None` improves performance. |
| Channel count | int | mode-dependent | up to 56 channels | Includes color + alpha + spot. |
| `Color Indicates` | enum | Masked Areas | Masked Areas / Selected Areas | Affects Quick Mask glyph; display only. |
| Mask overlay color | colour | red (255,0,0) | any colour | Display-only; not the protection value. |
| Mask overlay opacity | percent | 50 % | 0–100 | Display-only. CS6-era sources. |
| Spot Solidity | percent | 100 *(inferred)* | 0–100 | On-screen preview only; no effect on separations. |
| New channel name | string | `Alpha N` / sequence | — | `New Channel` dialog. |
| Duplicate destination | enum | current document | any same-size open doc / New | Cross-image duplicate relaxes size rule. |
| Split eligibility | bool | — | flattened / single-layer (CS6) | Otherwise disabled. |
| Merge mode | enum | RGB (3 images) | RGB / CMYK / Lab / Multichannel | Determined by open grayscale count. |

Panel Options default thumbnail size is not stated in the fetched Help text.

## Algorithms & pipeline

The panel is a **view over the document's channel list** plus a thin command surface; the image-processing semantics live elsewhere.

1. **Model projection.** Emit channels in document order (composite first for RGB/CMYK/Lab), each with `{ id, name, kind, thumbnail, visible, selected }`.
2. **Thumbnail generation.** Rasterise the channel to a small grayscale/colour preview, cached and invalidated on channel edits; `None` skips generation.
3. **Show/hide and colour view.** Visibility bits drive the compositing view and the "show channels in color" preference. The composite row is a derived view state, not a stored channel.
4. **Paint routing.** A painting tool addressed to the panel paints exactly one channel; the alpha value maps to the channel's grayscale (white = add, black = remove), matching `SEL-013`.
5. **Save/load selection.** Converting a selection bitmap to/from a channel is the same engine as `SEL-012`; the panel supplies the destination channel and combine mode.
6. **Split.** For a flattened/single-layer image, emit each channel as a new grayscale document and close the source. **Merge** is the inverse over grayscale inputs.
7. **Commands.** Colour-channel deletion, split and merge are document-structural commands carrying the appropriate undo record (`ARCH-009`); alpha paint records channel tiles.

## Rust module mapping

Design proposal; the channel model already belongs to `ARCH-008`.

- `pictura_core::document::channel::Channel` — `{ id, name, kind: Color | Alpha | Spot, pixels, visible, spot_color: Option<SpotSpec> }`.
- `pictura_core::document::ChannelStack` — ordered channels, composite derivation, and the up-to-56 limit.
- `pictura_core::document::channel_ops` — `NewAlpha`, `NewSpot`, `DuplicateChannel`, `DeleteChannel`, `ReorderChannels`, `RenameChannel`, `SetChannelVisible`, `SaveSelectionToChannel`, `LoadChannelAsSelection`, `SplitChannels`, `MergeChannels`.
- `pictura_core::selection` — shared selection↔mask conversion (`SEL-012`, `SEL-013`).
- `pictura_ui_bridge::ChannelsViewModel` — projected rows + thumbnail handles crossing to Qt.
- `pictura_core::color::SpotColor` — spot ink spec; library lookup via `CLR-003`.

Crossing types: `ChannelId`, `ChannelKind`, `SpotSpec`, `SelectionId`, thumbnail `TileId`. No Qt types in `pictura_core`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

| Proposal | Base | Responsibility |
|---|---|---|
| `ChannelsPanel` | `QDockWidget` | Host; list, bottom button strip, panel menu. |
| `ChannelsModel` | `QAbstractItemModel` | Channel rows; roles for kind, visibility, selection, thumbnail, spot swatch. |
| `ChannelRowDelegate` | `QStyledItemDelegate` | Eye column, thumbnail, name, composite marker, spot colour chip. |
| `ChannelThumbnailCache` | `QPixmapCache`-backed helper | Preview generation/invalidations. |
| `ChannelOptionsDialog` | `QDialog` | New/Channel Options: name, Color Indicates, colour+opacity, Spot Color. |
| `SplitChannelsAction` / `MergeChannelsDialog` | `QAction` / `QDialog` | Split; merge with mode/channel mapping and `Next` walk. |
| `SaveSelectionDialog` | `QDialog` | Document/Channel/Name/combine mode (`SEL-012`). |
| `SpotColorPicker` | `QDialog` | Spot library/colour selection (`CLR-003`). |
| `ChannelsPanelMenu` | `QMenu` | Panel-menu actions, enabled per channel kind/state. |

## Data-model impact

- **Channels already exist** in `ARCH-008`; the panel adds no field. Alpha channels double as layer masks (`LAY-004`), and the panel must present those channels without assuming they are standalone selections.
- **Spot channels** carry a spot-colour spec and a print overprint order; both persist in the PSD layer/channel records and are consumed by `10-workflow-io/printing.md` and `IMG-007`.
- **Composite channel** is a derived view, not stored.
- **Undo:** alpha paint records channel tiles; alpha/spot create/duplicate/delete/reorder are structural commands; Split/Merge are document-lifecycle commands (they create/close documents).
- **Unsaved selection**: a Quick Mask temporary channel is runtime-only and is never serialised (`SEL-004`).

## Edge cases

- **56-channel ceiling:** `New Channel`/`New Spot` must refuse at the limit.
- **Multichannel mode:** only then can alpha/spot move above colour channels; in other modes the drag must be rejected.
- **Deleting a colour channel in a layered file** flattens visible layers and discards hidden ones — this is destructive and must be warned/undoable.
- **Spot split/merge lossiness:** a spot channel in a split/recombined image becomes an alpha channel; the UI must report the downgrade rather than silently drop the ink.
- **Merge preconditions:** non-grayscale, layered, or size-mismatched inputs disable `Merge Channels`.
- **Bitmap / Indexed / 8-bit-only interactions:** alpha channels exist in Bitmap? The fetched text notes you cannot duplicate a channel *into* a Bitmap image and Indexed mode has no alpha channels; both must disable the corresponding actions.
- **Empty image / 1-px:** thumbnails and thumbnails-off must work; split of a single-channel grayscale yields one document.
- **Huge PSB / memory:** thumbnails and merge must stream per tile and respect the tile-cache cap (`ARCH-006`).
- **GPU unavailable:** thumbnails fall back to a CPU raster; panel operations unaffected.
- **Undo/redo:** undoing a delete restores channel data, name, kind, spot spec and order; Split/Merge undo re-opens/closes documents per `ARCH-009`'s document-lifecycle rules.

## Parity acceptance criteria

1. Given an RGB image, the panel lists the composite channel first, then R, G, B, then alpha channels, then spot channels.
2. Given an eye-column drag across multiple rows, exactly the dragged rows toggle visibility, and the composite shows exactly when all colour channels are visible.
3. Given `Show Channels in Color`, R/G/B previews render in colour (Lab: only a and b); with the preference off, grayscale.
4. Given a selection saved via the Save Selection button, a new alpha channel appears with the selection as grayscale and loads back identically via `Ctrl`/`Cmd`-click (within selection tolerance, `SEL-012`).
5. Given `Ctrl`/`Cmd`+`Shift`/`Alt`/both on a channel thumbnail, the loaded selection adds to / subtracts from / intersects with the current selection.
6. Given a flattened or single-layer document, `Split Channels` produces one grayscale window per channel and closes the source; `Merge Channels` reconstitutes the image and closes the inputs.
7. Given a spot colour channel, split-then-merge degrades it to an alpha channel and reports the downgrade.
8. Given a delete of a colour channel in a layered document, the operation is offered with a destructive warning and is undoable to the original layer stack.
9. Given 56 channels, `New Channel` and `New Spot` are disabled.
10. Given `Panel Options > None`, thumbnails are hidden and all operations remain available.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "About channels" (color/alpha/spot; 56-channel limit; same dimensions; format preservation — PSB/TIFF/PDF/raw, DCS 2.0 spot-only); "Channels panel overview" (`Window > Channels`; composite first; auto-updating thumbnail; `Panel Options` thumbnail size/`None`); "Show or hide a channel" (eye column, drag-through, composite); "Show color channels in color" (`Edit > Preferences > Interface > Show Channels in Color`); "Select and edit channels" (multi-select, `Shift`-click, paint one channel with white/gray/black); "Rearrange and rename alpha and spot channels" (Multichannel-only ordering, spot overprint order, double-click rename); "Delete a channel" (`Alt`-click, drag, menu; colour-channel flatten/multichannel consequence); "Create an alpha channel mask…" / "Channel options" (`Masked Areas`/`Selected Areas`, Color, Spot Color; exact option prose); "Paint on a channel to mask image areas"; "Save and load selections" (Save Selection button; `Select > Save Selection` Document/Channel/Name and Replace/Add/Subtract/Intersect); "Duplicate, split, and merge channels" (Duplicate Channel options; split flattened-only; merge grayscale/flattened/same-size; Multichannel fallback; spot cannot split/recombine); "Create a new spot channel" / "Merge spot channels" (Solidity 0–100 %; Color Libraries; `New Spot Channel` and `Merge Spot Channel` in the panel menu); "Keys for the Channels panel" (`Ctrl+2`/`3/4/5`, load/combine shortcuts, `Alt`-click Save Selection, `Ctrl`-click New Channel ⇒ spot, `~` Quick Mask); "Productivity enhancements (JDI's) in CS6 > Channels" (Split Channels enabled for single-layer documents).

Consumer documentation (fetched):

- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Channels_palette.html` — CS5/CS6-era book companion: the composite channel is always at the top, colour channels occupy slots 3–6, alpha channels added via Save Selection, up to 56 channels, and Command/Ctrl-click a channel to load it as a selection.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` — CS6 Essentials default workspace: Channels is tabbed with Layers and Paths in the bottom-right group.
- `https://jkost.com/blog/2012/06/the-properties-panel-in-photoshop-cs6.html` — used only for the CS6-era assertion that the Properties panel (not this panel) now surfaces mask properties; see `PAN-006`.
- `https://www.scribd.com/document/58918086/6-Masks-and-Channels` and `https://community.adobe.com/questions-712/how-to-show-red-overlay-for-layer-mask-in-photoshop-1115428` — CS6-era corroboration that the new-channel/mask overlay default is red at 50 % opacity.

Consulted as search-result snippets only (not individually fetched): SearXNG query for "Photoshop CS6 Properties panel shape layer live properties type contextual". `helpx.adobe.com` returns 403 and was not used.

## Open questions

- **Panel Options thumbnail default** on a fresh CS6 install is not stated. *Resolves with:* a preferences dump or CS6 capture.
- **Spot Solidity default** — the Help describes the range but not the initial value (100 % assumed). *Resolves with:* a CS6 `New Spot Channel` capture.
- **`Merge Channels` mode/channel-count mapping** beyond the 3→RGB, 4→CMYK examples (e.g. Lab, Duotone) is not enumerated. *Resolves with:* a CS6 test across image modes.
- **Quick Mask channel rename/visibility persistence** is runtime-only per the Help; whether the temporary channel is listed with a stable name is not asserted. *Resolves with:* a CS6 capture.
- **Spot channel colour library linkage** and its PSD serialisation belong to `CLR-003`/`ARCH-008`; the panel only presents the picker.
- **Split/Merge undo granularity** across document close/open is not specified in the Help. *Resolves with:* a CS6 test and `ARCH-009`.
