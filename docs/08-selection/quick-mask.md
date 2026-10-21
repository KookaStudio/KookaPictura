# Quick Mask (Selection ⇄ Mask Semantics)

- **Spec ID:** `SEL-004`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Quick Mask is a long-standing selection-editing mode carried into CS6. CS6 adds no documented Quick Mask behavior change. (CS6's mask-related change was moving *layer-mask* editing from the CS5 Masks panel into the Properties panel — that is `LAY-004`, not Quick Mask.)
- **Depends on:** `SEL-001` selection-model, `SEL-002` selection-tools-overview, `TOOL-044` quick-mask-tool, `07-color-painting/brush-engine.md`, `05-layers/layer-masks.md` (`LAY-004`), `08-selection/channel-based-masking.md`, `08-selection/save-and-load-selections.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `02-ui-ux/panels/channels-panel.md`.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help corpus.
> This spec covers the **selection/masking semantics**; the painter tool UI, brush
> settings, and overlay compositing details live in `TOOL-044` and are
> cross-referenced, not repeated.

## CS6 behavior

Quick Mask mode is the bridge between the active selection and an editable mask. CS6 defines it as a way to convert a selection into a temporary mask for easier editing: the Quick Mask appears as a colored overlay with adjustable opacity, can be edited with any painting tool or modified with a filter, and is converted back into a selection on the image when the mode is exited.

Semantics that matter to the selection model:

- **It is a temporary alpha channel.** A temporary Quick Mask channel appears in the Channels panel for as long as Quick Mask mode is active; all mask editing still happens in the image window. Entering the mode creates the channel; leaving it converts the mask back to a selection and removes the temporary channel.
- **Selected vs masked is polarity, not two different masks.** The overlay shows the *protected* (unselected) area by default; the selected area is unprotected. A rubylith-like color overlay covers and protects everything outside the selection, leaving the selected area unprotected; by default the protected area is tinted red at 50% opacity.
- **Painting changes coverage.** With the default **Masked Areas** display, painting with white selects more of the image (the overlay is removed where white is painted); painting with black deselects (the overlay covers where black is painted); painting with gray or another color creates a semitransparent area useful for feathering or anti-aliased effects.
- **Partial coverage survives the round trip.** Semitransparent areas may not look selected after exiting Quick Mask mode, but they are — partial values are retained in the resulting selection.
- **The boundary is the 50% line.** When a feathered mask is converted to a selection, the boundary line runs halfway between the black and white pixels of the mask gradient; it marks the transition between pixels that are less than 50% selected and those that are more than 50% selected.
- **Two display conventions flip the paint semantics.** The Quick Mask Options dialog offers:

  - **Masked Areas** (default): masked areas are black (opaque) and selected areas white (transparent); painting with black enlarges the masked area, painting with white enlarges the selected area.
  - **Selected Areas**: masked areas are white (transparent) and selected areas black (opaque); painting with white enlarges the masked area, painting with black enlarges the selected area.

  The toolbox button glyph reflects the choice (white-on-gray for Masked Areas, gray-on-white for Selected Areas), and `Alt`/`Option`-click toggles the two.
- **Color and opacity are display-only.** The color and opacity settings affect only how the mask looks and never change how underlying areas are protected. Opacity accepts 0–100%; default red at 50%.
- **Persistence.** Leaving Quick Mask produces a selection; it can then be saved permanently with `Select > Save Selection` or the Channels panel, which is how a temporary mask becomes a real alpha channel. A Quick Mask is *not* itself saved.
- **Filters apply to the mask.** CS6 explicitly allows modifying the Quick Mask with a filter, so a mask can be blurred (feather), sharpened, etc., before conversion back to a selection — this is a selection-mask operation, not a pixel edit.

The painter tool details (toolbox button, `Q`, brush/opacity/flow, swatch auto-swap to black/white) are specified in `TOOL-044`.

## UI surface

Semantics-defining surface only; the full tool surface is in `TOOL-044`.

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox | Mode toggle | `Q` | Standard ⇄ Quick Mask (`TOOL-044`) |
| Toolbox button | `Alt`/`Option`-click | — | Toggle Masked Areas / Selected Areas |
| Quick Mask Options | Dialog | — | Display option, mask color, opacity 0–100% |
| Channels panel | Temporary channel | — | Appears only while in Quick Mask mode |
| `Select > Save Selection` | Menu | — | Persist the round-tripped selection as an alpha channel |
| `Select > Color Range` | Dialog | — | Offers a **Quick Mask** selection-preview mode (`SEL-005`) |
| `~` (tilde) | Key | — | Toggle composite / grayscale mask view while in Quick Mask (CS6 Channels key table) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Display option | enum | Masked Areas | Masked Areas / Selected Areas | Controls paint polarity and glyph |
| Mask opacity | percent | 50 | 0–100 | Display only |
| Mask color | RGBA | red | any | Display only |
| Coverage value | u8 | from selection | 0–255 | White=fully selected, black=fully masked (Masked Areas) |
| Brush opacity/flow | percent | tool default | 1–100 | Produces partial coverage (feather/AA) |
| Temporary channel | — | created on entry | removed on exit | Not serialized |

## Algorithms & pipeline

### Selection ⇄ mask conversion

The conversion is an identity on coverage; only the *interpretation* and the optional display overlay differ.

```text
enter_quickmask(selection):
    channel = temporary_alpha(selection.coverage)   # 1:1 copy
    mode = QuickMask { display, color, opacity }
    paint_edits(channel)                            # painting / filters mutate coverage
leave_quickmask(channel):
    selection = Selection::from_coverage(channel)   # 1:1 copy, no quantization
    remove temporary channel
```

- **No lossy step.** Semitransparent values survive; the ants drawn on exit are the 50% contour (`SEL-001`).
- **Filters** operate on the temporary channel like any grayscale image, so feather = blur, and threshold filters can harden a mask.
- **Painting** reproduces paint-engine alpha compositing against the mask with the brush's opacity/flow; this reuses `07-color-painting/brush-engine.md`.

### Overlay composite (display only)

```text
shown(p) = lerp(image(p), mask_color, opacity * (1 - coverage_norm(p)))   # Masked Areas
shown(p) = lerp(image(p), mask_color, opacity * coverage_norm(p))         # Selected Areas
```

Where `coverage_norm = coverage/255`. Color/opacity never enter the mask. This matches the CS6 statement that changing them may make the mask more easily visible against the image colors but does not change protection.

### Round trip to a persistent mask

A Quick Mask becomes durable only through an explicit save:

```text
leave_quickmask -> selection
Select > Save Selection -> new/selected alpha channel (ChannelKind::Alpha)
```

The alpha channel and a Quick Mask use the same grayscale-mask convention, so a saved Quick Mask can be reloaded as a selection or used as a layer mask source (`LAY-004`).

## Rust module mapping

- `pictura_selection::quickmask::QuickMaskSession` — `{ temp: Mask, display: MaskedAreas | SelectedAreas, color, opacity, active: bool }`.
- `QuickMaskSession::enter(&Selection)` — copies coverage into `temp`; `leave(&self) -> Selection`.
- `pictura_selection::quickmask::cover(&Mask, image) -> OverlayFrame` — display-only compositor (or a GPU shader, `ARCH-006`).
- `pictura_selection::mask::apply_filter(&mut Mask, filter)` — restrict filters to mask-editing operations.
- `pictura_selection::alpha::save(&Selection)` — persist to a real channel (`SEL-001`).
- Shared: `Mask` tiles, `SelectionOp`, `MaskColor`, `opacity`.

Boundary types: `Mask` tiles (borrowed/shared), `MaskColor` (RGBA), `opacity` (u8), and an `active` flag. The Qt overlay consumes a pre-composited frame, never the raw mask, while in Quick Mask mode.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `QuickMaskModeButton` | `QToolButton` | `Q` toggle; `Alt`-click switches Masked/Selected Areas; glyph matches option (`TOOL-044`) |
| `QuickMaskOptionsDialog` | `QDialog` | Display option radio, color picker, opacity spin |
| `QuickMaskChannelEntry` | model row | Temporary Channels-panel entry present only while active |
| `MaskEditOverlay` | `QGraphicsItem` | Rubylith composite; display-only |
| `MaskFilterMenu` | submenu of Filters | Applies allowed filters to the temporary mask |

Widgets over QML for the same reasons as `TOOL-044`: the mode is a dense, keyboard-centric toolbox state; the overlay is a single vector/raster canvas item.

## Data-model impact

- A Quick Mask is **ephemeral**: the temporary channel lives only inside the active session and is never serialized by `ARCH-011`.
- On exit, exactly one **selection change** is committed. Undo model options (resolve in `ARCH-009`):
  - one `SelectionChange` for the entire Quick Mask session (coarse), or
  - per-paint-edit mask snapshots while active (fine, matching Photoshop's editing history behavior).
  The fetched CS6 Help does not state Quick Mask's history granularity — see Open questions.
- Saving via `Select > Save Selection` creates/updates an alpha channel (`ChannelKind::Alpha`), which *is* serialized and undoable.
- Display option, color, and opacity are **preferences/session state** (and influence the channel options dialog), never document pixels.
- Filters applied to the mask are mask operations; they must not alter layer pixels.

## Edge cases

- **Entering with no selection:** define the result — CS6 does not document it. Proposal: enter with an all-masked (empty-selection) channel so painting with white adds a fresh selection; verify against CS6.
- **Partial coverage:** keep it; do not threshold on exit. Only ants use the 50% line.
- **Empty/1-px documents:** overlay and paint must handle degenerate sizes; a 1-px mask toggles between fully selected and fully masked.
- **Polarity switch while editing:** `Alt`-click flips Masked/Selected Areas; the underlying coverage must not change, only paint interpretation and glyph. A pending brush stroke must not re-interpret mid-stroke.
- **Color/opacity change:** never changes coverage; verify by saving to a channel before and after.
- **Channel budget:** saving a Quick Mask consumes a channel; respect the 56-channel limit and surface the same failure as any save-selection operation.
- **16/32-bit:** coverage semantics unchanged; whether the temporary channel respects document depth is unspecified. Painting partial coverage is independent of pixel depth.
- **CMYK/Lab:** the overlay tint is a UI color, not a document color; no conversion.
- **Huge/PSB:** temporary channel is tiled; entering/leaving must not copy the whole document in one flat buffer.
- **GPU unavailable:** overlay falls back to CPU `QPainter`; mask data is CPU regardless.
- **Undo:** cancelling/leaving must not leave a dangling temporary channel; history after leaving must match the selection model's rules.

## Parity acceptance criteria

- Given any selection, `Q` enters Quick Mask with a red, 50%-opaque overlay over the **masked** (unselected) area; `Q` again returns a selection whose coverage equals the mask exactly (including partial values).
- Given Masked Areas, painting white adds coverage and black removes it; given Selected Areas, the polarity is reversed.
- Given a gray brush at 50% opacity, the resulting selection contains partial (intermediate) coverage values.
- Given a feathered Quick Mask, the post-exit marching-ants boundary sits at the 50% coverage contour.
- Given a color or opacity change, the overlay appearance changes but a saved selection before/after is byte-identical.
- Given `Alt`-click on the Quick Mask button, the glyph and paint polarity switch without changing coverage.
- Given a filter applied in Quick Mask mode (e.g. blur), the mask changes and no layer pixel changes.
- Given `Select > Save Selection` after leaving Quick Mask, a new alpha channel holds the mask and can be reloaded as the same selection.
- Given the `~` key in Quick Mask mode, the view toggles between the composite overlay and the grayscale mask.
- Given entering and immediately leaving Quick Mask with no edit, the selection is unchanged.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: the Quick Mask definition (converting a selection to a temporary mask for editing, shown as a colored overlay with adjustable opacity, editable with any painting tool or a filter, and converted back to a selection on exit); the temporary Channels-panel channel; the rubylith overlay protecting the unselected area; default red 50% overlay; painting white selects / black deselects and gray creates semitransparency; the note that semitransparent areas may not look selected on exit but are; the 50% boundary rule; Masked Areas vs Selected Areas definitions and the `Alt`/`Option`-click toggle; mask color and opacity being display-only; the channel Masked/Selected Areas options; `Select > Save Selection` as the path to a permanent alpha channel; Quick Mask as a Color Range selection-preview mode; the `~` toggle between composite and grayscale mask in the Channels key table.

Cross-referenced (not duplicated here):

- `03-tools/quick-mask-tool.md` (`TOOL-044`) — owns the toolbox button, `Q` behavior, painting tool options, and the overlay compositing proposal; it cites the same CS6 PDF.

## Open questions

- **Entering Quick Mask with no selection.** Undocumented. Resolve with a CS6 capture (does it produce an all-masked or all-selected temporary channel?).
- **History granularity for mask painting.** Whether each brush stroke is a history state or the whole Quick Mask session is one state is unstated. Resolve in `ARCH-009` with a CS6 test.
- **Which filters are allowed on a mask.** CS6 says the mask can be modified with a filter without listing which filters (filters requiring color data or producing color are presumably disabled). Resolve with a CS6 test.
- **Temporary channel bit depth.** Whether the temporary channel follows document bit depth (8/16/32) is unknown. Resolve by inspecting conversion on a 16/32-bit document.
- **Interaction with layer masks.** Whether entering Quick Mask while a layer mask is selected edits the selection or the layer mask in CS6 is not documented in the fetched text. Resolve with a CS6 capture.
- **`~` toggle scope.** Whether the composite/grayscale toggle is Quick-Mask-specific or a general Channels-panel behavior needs confirmation from a CS6 capture.
