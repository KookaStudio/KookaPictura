# Quick Mask Tool

- **Spec ID:** `TOOL-044`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Quick Mask is a long-standing selection-editing mode carried into CS6. CS6 adds no documented Quick Mask behaviour change.
- **Depends on:** `08-selection/selection-model.md`, `08-selection/quick-mask.md`, `08-selection/save-and-load-selections.md`, `08-selection/channel-based-masking.md`, `07-color-painting/brush-engine.md`, `01-architecture/document-model.md`, `01-architecture/undo-history.md`, `01-architecture/qt6-ui-design.md`, `03-tools/smudge-blur-sharpen.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

Quick Mask mode converts a selection into a **temporary, paintable mask**. The
CS6 Help: "Use Quick Mask mode to convert a selection to a temporary mask for
easier editing. The Quick Mask appears as a colored overlay with adjustable
opacity. You can edit the Quick Mask using any painting tool or modify it with a
filter. Once you exit Quick Mask mode the mask is converted back to a selection
on the image."

- **Toggle** — the **Quick Mask Mode** button at the bottom of the toolbox; the
  CS6 keyboard table gives it `Q` ("Toggle between Standard mode and Quick Mask
  mode"). A **Standard Mode** button returns to normal editing.
- **Overlay** — by default the mode "colors the protected area using a red, 50%
  opaque overlay," described as a rubylith. Colour and opacity are adjustable.
- **Masked vs Selected Areas** — the Quick Mask Options dialog offers
  **Masked Areas** (default: "Sets masked areas to black (opaque) and selected
  areas to white (transparent)") and **Selected Areas** ("Sets masked areas to
  white (transparent) and selected areas to black (opaque)"). The toolbox button
  glyph reflects the choice (white circle on gray for Masked Areas; gray circle
  on white for Selected Areas). `Alt`/`Option`-clicking the button toggles the
  two options.
- **Options dialog** — double-click the Quick Mask Mode button to set the display
  option, the mask **colour**, and the **opacity** (0–100%). The Help stresses
  that colour and opacity "affect only the appearance of the mask and have no
  effect on how underlying areas are protected."
- **Painting the mask** — select any painting tool; "The swatches in the toolbox
  automatically become black and white." With Masked Areas:
  - paint **white** to select more of the image (overlay is removed),
  - paint **black** to deselect (overlay covers the area),
  - paint **gray** (or another colour) for a semitransparent area useful for
    feathering/anti-aliasing. "Semitransparent areas may not appear to be
    selected when you exit Quick Mask Mode, but they are."
- **Temporary channel** — "A temporary Quick Mask channel appears in the Channels
  panel while you work in Quick Mask mode. However, you do all mask editing in
  the image window." `~` (tilde) toggles between the composite and the grayscale
  mask view.
- **Exit to selection** — clicking Standard Mode returns to the image with "a
  selection border [that] surrounds the unprotected area of the quick mask." For
  a feathered mask, the boundary runs at the 50% point: "between pixels that are
  less than 50% selected and those that are more than 50% selected."
- **Persistence** — a Quick Mask is temporary. "You can convert this temporary
  mask to a permanent alpha channel by switching to standard mode and choosing
  `Select > Save Selection`."
- **Colour Range preview** — the Color Range dialog also offers a **Quick Mask**
  selection-preview option, which "shows unselected areas as a rubylith overlay
  (or a custom color you've specified in the Quick Mask Options dialog box)."

The mask is 8-bit grayscale in normal practice ("Masks and channels are grayscale
images, so you can edit them like any other image"); the exact bit depth of the
Quick Mask channel relative to 16/32-bpc documents is not stated by the fetched
CS6 source.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox bottom | Mode button | `Q` | Toggles Standard / Quick Mask |
| Quick Mask Mode button | Double-click | — | Opens Quick Mask Options |
| Quick Mask Mode button | `Alt`/`Option`-click | — | Toggles Masked Areas / Selected Areas |
| Quick Mask Options | Dialog | — | Display option, mask colour, opacity |
| Channels panel | Temporary channel | — | Shown while in the mode; editing happens in the image window |
| `~` (tilde) | View toggle | `~` | Composite vs grayscale mask view |
| Options bar (painting tool) | Tool options | — | Brush, opacity, flow, mode apply to mask painting |
| Select > Save Selection | Menu | — | Persists the mask as an alpha channel |
| Select > Load Selection | Menu | — | Reloads a saved alpha channel as a selection |
| Select > Color Range | Dialog | — | Has a Quick Mask selection-preview mode |
| Select > Deselect | Menu | `Ctrl/Cmd+D` | Dismisses the resulting selection |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Display option | enum | Masked Areas | Masked Areas / Selected Areas | Quick Mask Options |
| Mask colour | colour | red (rubylith) | any colour | Affects display only |
| Mask opacity | percent | 50 | 0–100 | Affects display only |
| Toolbox swatches | colour | black / white | — | Auto-swapped on entering the mode |
| Painting tool | tool | current | Brush, Pencil, Eraser, gradients, filters | Any painting/editing tool; filters allowed |
| Brush opacity / flow | percent | from tool | 1–100 | Produces partial mask values (feather) |
| Mask value | 8-bit grey | — | 0–255 per pixel | 0 = unselected, 255 = selected *(inferred convention)* |

## Algorithms & pipeline

### Selection ⇄ mask representation

A selection is modelled as a document-resolution **8-bit coverage mask**:
`0` = fully unselected, `255` = fully selected, intermediate values encode
feathering/anti-aliasing. This is the same representation used for layer masks
and saved alpha channels, which is why the Help can say masks are grayscale
images editable with painting tools and filters.

```text
Selection {
    width: u32, height: u32,
    coverage: TileMap<u8>,   // document-resolution, tiled
}
```

### Entering and leaving the mode

```text
enter(selection):
    quick_mask_channel = selection.coverage          // as a mask
    mode = QuickMask { display, color, opacity }
    display_composite = overlay(image, quick_mask, display, color, opacity)

exit() -> Selection:
    coverage = quick_mask_channel
    selection = threshold_and_preserve(coverage, 0.5) // boundary at 50%
    // partial coverage below/above 50% is preserved in the selection's alpha
```

- **Overlay composite** (Masked Areas): for each pixel,
  `shown = lerp(image, color, opacity * (1 - coverage_norm))`, i.e. the *unselected*
  area is tinted. **Selected Areas** inverts the coverage term. Colour/opacity
  are strictly presentation and must never be written into coverage.
- **Boundary at 50%**: the marching-ants outline is where coverage crosses 0.5;
  the underlying selection keeps fractional coverage so feathered edges survive
  the round trip.
- **Filters on the mask**: because the mask is an image channel, applying e.g.
  Gaussian Blur to it feathers the selection — consistent with the Help.
- **`~` view**: toggles rendering the coverage itself (grayscale) instead of the
  composited overlay.
- **Save/Load round trip**: `Save Selection` writes coverage into an alpha
  channel (by convention white = selected); `Load Selection` reads it back and
  can combine (New/Add/Subtract/Intersect) per `08-selection/save-and-load-selections.md`.
  Alpha channels mask or replace each other, so a Quick Mask ⇄ alpha-channel
  round trip must be lossless for 8-bit masks.

### Performance

The mask is a full-canvas 8-bit buffer (or tiled equivalent). Only tiles touched
by a stroke, filter, or overlay recompute; the overlay is a shader pass over the
visible region (`ARCH-006`).

## Rust module mapping

Proposals.

- `pictura_core::selection` — `Selection` as a `Mask` over the document tile
  grid; `from_channel`, `to_channel`, `threshold`, `feather`, `combine`
  (`New | Add | Subtract | Intersect`).
- `pictura_core::channel` — `Channel`, `ChannelKind::Alpha`, `ChannelOptions
  { display: MaskedAreas | SelectedAreas, color, opacity }`.
- `pictura_tools::quickmask` — `QuickMaskSession { active, display, color,
  opacity, mask: Selection }`; `enter(&Selection)`, `paint(dab)`,
  `filter(&Filter)`, `toggle_view()`, `exit() -> Selection`.
- `pictura_tools::quickmask::overlay` — the presentation shader input (colour +
  opacity + coverage); never mutates the mask.
- `pictura-script::command` — `EnterQuickMask` / `ExitQuickMask` /
  `SaveSelection` commands so scripting and the Actions panel can drive the mode.

Crossing types: `Selection`/`Mask` (tiled `u8`), `MaskColor` (RGBA), `opacity`.
The temporary channel is owned by the session, not the serialized document.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `QuickMaskModeButton` | `QToolButton` | Toolbox toggle; `Alt`-click switches Masked/Selected Areas; glyph matches the option |
| `QuickMaskOptionsDialog` | `QDialog` | Display option radio, colour picker, opacity spin |
| `ChannelsModel` | `QAbstractItemModel` | Shows the temporary Quick Mask channel while active (`02-ui-ux/panels/channels-panel.md`) |
| `CanvasWidget` | `QRhiWidget` | Renders the overlay pass; `~` toggles grayscale mask display |
| `SelectionModel` | `QObject` | Publishes selection changes; Save/Load call into the Rust selection module |

The mode is a presentation + tool state on top of the existing canvas and
channels model; no new top-level window. The overlay is a shader, not a bitmap
composited in the document.

## Data-model impact

- **Temporary channel is not serialized.** The Quick Mask channel exists only
  while the mode is active and is discarded on exit (its content becomes the
  selection). It must not be written to PSD/PSB.
- **Selection is not serialized either.** Only `Select > Save Selection` creates
  a durable artifact: an **alpha channel** in the document, preserved in the PSD
  (image resources / channel records). Alpha channels count against the 56-channel
  limit including colour channels.
- **Undo**: painting, erasing, and filtering the mask are destructive edits to the
  mask channel and each completed stroke/filter is a history state
  (`ARCH-009`). Whether entering/exiting Quick Mask itself creates a history
  state is unverified; the safe model is that it does not, and only mask edits do.
- **Preferences**: the Quick Mask display option, colour, and opacity are
  application-level (Quick Mask Options) and persist in the preference store
  (`11-cross-cutting/preference-storage.md`), not per document.
- **Channel options**: the Masked/Selected Areas display and channel colour are
  channel metadata for presentation; whether CS6 serializes per-channel display
  options into the PSD is unverified.
- **Bit depth**: coverage is normally 8-bit; for 16/32-bpc documents the mask's
  bit depth and its behaviour on save are unverified.

## Edge cases

- **Entering with no selection** — the resulting mask coverage (all-selected
  versus all-masked) determines the initial overlay; the CS6 Help does not state
  it. Must be verified so `Q` with no selection shows no unintended rubylith.
- **Feathering / anti-aliasing** — fractional coverage must survive the
  selection → mask → selection round trip; the 50% threshold only governs the
  marching-ants outline, not stored coverage.
- **`Alt`-click and the Options dialog** must reflect the same state in both
  directions (button glyph, radio, overlay).
- **Colour/opacity changes** must never alter coverage; only the display.
- **Bit depth 8/16/32** — confirm the mask channel depth and any conversion.
- **Colour modes** — RGB/CMYK/Lab/Grayscale share the selection model; Indexed
  and Bitmap mode availability is unverified.
- **Empty / 1-px / huge (PSB) documents** — the mask is document-resolution;
  PSB masks are large, so tiled storage and lazy allocation matter.
- **GPU unavailable** — the overlay falls back to a CPU-composited preview with
  the same semantics.
- **Undo/redo** — mask edits must undo bit-exactly; entering/exiting the mode
  must not corrupt history.
- **Filters on the mask** — a modal filter during Quick Mask must apply to the
  mask, not the image, and be undoable.
- **Channels-panel interaction** — deleting the temporary channel or switching
  channels mid-mode must be prevented or must exit the mode cleanly.
- **Save Selection naming** — new alpha channels get default names (`Alpha 1`, …)
  with overwrite/invert options; duplicate names must be handled.

## Parity acceptance criteria

- Given an arbitrary selection, pressing `Q` enters Quick Mask with a red, 50%
  opaque overlay over the unselected area; pressing `Q` again returns a selection
  whose outline matches the pre-toggle selection within tolerance.
- Given the mask overlay, painting with white expands the selection and painting
  with black contracts it (Masked Areas); the toolbox swatches are black and
  white while in the mode.
- Given **Selected Areas** (set via the dialog or `Alt`-click), the overlay
  covers the selected area instead, and white/black painting invert their effect
  accordingly; the toolbox glyph matches.
- Given a feathered selection, exiting Quick Mask preserves partial coverage
  (re-entering reproduces the same overlay) and the marching-ants boundary sits
  near the 50% coverage line.
- Given a colour or opacity change, the overlay appearance changes and the
  resulting selection is bit-identical to before the change.
- Given `~`, the view toggles between the composite and the grayscale mask.
- Given `Select > Save Selection`, a new alpha channel appears in the Channels
  panel containing the mask; `Select > Load Selection` restores the selection
  (with New/Add/Subtract/Intersect honoured).
- Given a mask-painting stroke, the History panel records one state and undo
  restores the mask bit-exactly; toggling the mode adds no spurious state.
- Given a modal filter while in Quick Mask, the filter affects the mask and can
  be undone.
- Given the document is saved and reopened, the Quick Mask mode is off and no
  temporary channel exists, while any saved alpha channels persist.
- Given `devicePixelRatio = 2`, the overlay is drawn at device resolution
  without affecting coverage.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  Quick Mask definition ("convert a selection to a temporary mask … colored
  overlay with adjustable opacity … exit … converted back to a selection");
  masks stored in alpha channels and black/white semantics; the temporary Quick
  Mask channel in the Channels panel; Masked Areas vs Selected Areas and their
  toolbox glyphs; `Alt`/`Option`-click toggling; the Options dialog colour and
  opacity and that they affect appearance only; painting white/black/gray and
  semitransparent areas; the toolbox swatches becoming black/white; the exit
  boundary at the 50% point; converting the mask via `Select > Save Selection`;
  the `Q` keyboard shortcut; the `~` composite/grayscale toggle in Quick Mask
  mode; Color Range's Quick Mask preview; and the Channel Options Masked/
  Selected Areas definitions.
- `https://glensmith.co.uk/photoshop/quick-mask` — community tutorial:
  double-click the Quick Mask button for Options, choose masked vs selected
  areas, change colour and opacity, and the semi-opaque red default. Secondary
  source; reached directly from that site's tool index.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Initial state with no selection.** Whether `Q` with no selection yields an
  all-selected or all-masked mask is not documented. *Resolves with:* a CS6 test.
- **Does entering/exiting Quick Mask create a history state?** Unverified.
  *Resolves with:* a CS6 History-panel observation.
- **Quick Mask channel bit depth at 16/32 bpc.** Whether the mask is always 8-bit
  or follows the document depth is unstated. *Resolves with:* a CS6 test and
  `04-image-ops/bit-depth-and-conversion.md`.
- **Channel display options serialization.** Whether Masked/Selected Areas and
  channel colour are stored in the PSD for alpha channels is unverified.
  *Resolves with:* a PSD inspection (`ARCH-011` file-formats).
- **Default alpha-channel naming and overwrite/invert semantics** of
  `Save Selection` need a CS6 reference. *Resolves with:*
  `08-selection/save-and-load-selections.md`.
- **Availability in Indexed and Bitmap modes.** *Resolves with:* a CS6 mode test.
- **Precise feather round-trip tolerance.** The error introduced by selection →
  mask → selection at a given feather radius must be quantified. *Resolves with:*
  a numeric CS6 experiment; feeds `08-selection/quick-mask.md`.
- **Channel-panel integrity.** Deleting the temporary channel mid-mode and other
  channels-panel operations need defined behaviour. *Resolves with:*
  `02-ui-ux/panels/channels-panel.md`.
