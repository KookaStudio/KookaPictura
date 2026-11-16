# Design

## Context

`ImageView` already has a display-only rubylith overlay slot
(`setMaskOverlay`) used by the Layers panel's mask overlay. The Channels panel
has alpha rows with disabled eyes, and the frame's `syncMaskOverlay` runs on
every refresh for each canvas of the active document. The panel cannot include
`frame.h` (shell-seam rule), so it communicates through a signal and a
`PanelHost` hook.

## Decisions

### 1. Reuse the canvas overlay slot

When an alpha channel is shown, the frame builds an RGBA image from
`channels_alpha_thumbnail(view, index, 4096, w, h)` and passes it to
`ImageView::setMaskOverlay` over the document rect. The image is red at 50 %
opacity where the channel is black (the default Masked Areas sense), so the
composite stays visible under the rubylith without changing the document.

The layer-mask overlay and the alpha view share the one overlay slot; the alpha
view takes precedence while shown, and the layer mask resumes when it is hidden.

### 2. The panel owns the shown-channel state

`ChannelsPanel` keeps `shownAlpha_` (or -1). Alpha eyes are enabled and
checkable; toggling one updates `shownAlpha_` and emits
`alphaVisibilityChanged`. The composite and colour eyes clear `shownAlpha_`.
`setView` clears it on a document switch, and delete adjusts or clears it when
the shown channel is removed.

### 3. Refresh through PanelHost

`PanelHost` gains an `alphaVisibilityChanged` hook wired to the frame's
`refresh`. The existing `syncMaskOverlay` pass then installs or clears the
canvas overlay. No new frame method is needed.

### 4. Display-only

The overlay is built from the channel preview; the channel plane, selection,
history, and document are untouched. The overlay is rebuilt on refresh when the
shown index or canvas revision changes.

## Risks / Trade-offs

- [Large documents] -> the overlay is capped at 4096 px per side and scaled to
  the document rect; it is not full resolution.
- [Layer-mask overlay conflict] -> the alpha view wins while shown; hiding it
  restores the mask overlay on the next refresh.
- [Fixed overlay options] -> the overlay always uses the default Masked Areas
  sense at red 50 %; per-channel colour, opacity, and sense are out of scope.
