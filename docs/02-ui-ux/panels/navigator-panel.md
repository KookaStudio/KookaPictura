# Navigator Panel

- **Spec ID:** `PAN-014`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Navigator panel is functionally unchanged. Zoom behavior changed elsewhere (Animated Zoom and Zoom-clicked-point preferences), not in the panel.
- **Depends on:** `ARCH-003` qt6-ui-design, `ARCH-006` gpu-rendering-pipeline, `03-tools/hand-and-zoom.md`, `02-ui-ux/preferences.md`, `02-ui-ux/workspace-and-docks.md`.

> This document owns the **Navigator panel UI surface**. Zoom commands, the Zoom
> tool, and zoom preferences belong to `03-tools/hand-and-zoom.md`; document/view
> separation belongs to `ARCH-008` document-model. All crate, module, widget, and
> type names are **design proposals**. The repository contains a working
> implementation; this file stays the behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not. Facts
> not confirmed by a fetched CS6 source are marked *(inferred)*.

## CS6 behavior

`Window > Navigator` opens the **Navigator panel**. The panel shows a thumbnail of the artwork so the view can be changed quickly; the colored box in the Navigator, called the **proxy view area**, corresponds to the currently viewable area in the window.

**Default placement.** The Navigator panel is not in the default Essentials workspace; it is supplied by the **Painting** workspace, where it replaces the Color panel in the top group of the main right-hand column (`02-ui-ux/workspace-and-docks.md`, `UI-003`).

### Anatomy

From the CS6 Help figure labels (A–G):

| Label | Element |
|---|---|
| A | Panel menu button |
| B | Thumbnail display of artwork |
| C | Proxy preview area (colored box) |
| D | Zoom text box |
| E | Zoom Out button |
| F | Zoom slider |
| G | Zoom In button |

### Interactions

- **Display** — `Window > Navigator`.
- **Zoom** — type a value in the text box, use the Zoom Out or Zoom In button, or drag the zoom slider.
- **Pan** — drag the proxy view area in the image thumbnail, or click the thumbnail to designate the viewable area.
- **Simultaneous size + position** — "**Control-drag** (Windows) or **Command-drag** (macOS) in the image thumbnail" sets the proxy area's size and position together.
- **Proxy color** — choose **Panel Options** from the panel menu, then select a preset from the **Color** pop-up menu or click the color box for a custom color.

### Zoom range and relationship to other zoom UI

The Zoom tool magnifies or reduces the image to the next preset percentage and centers the view on the clicked point. When the image reaches its **maximum magnification level of 3200%** or **minimum size of 1 pixel**, the magnifying glass appears empty. The zoom level can also be set from the lower-left corner of the document window or in the Navigator panel, so the Navigator's zoom box, the status-bar zoom box, and `View > Zoom In` / `View > Zoom Out` must stay synchronized. Panel zoom is **view state**, not an image edit, and is therefore not undoable.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Navigator` | Panel | — | Opens/toggles the Navigator panel |
| Panel thumbnail | 2-D control | click / drag | Click sets viewable area; drag pans the proxy |
| Panel thumbnail | Gesture | `Ctrl`/`Cmd`-drag | Sets proxy size + position simultaneously |
| Proxy view area (colored box) | Overlay | drag | Pans the view |
| Zoom text box | Numeric entry | — | Type a magnification |
| Zoom Out button | Button | — | Steps magnification down |
| Zoom slider | Slider | — | Continuous magnification |
| Zoom In button | Button | — | Steps magnification up |
| Panel menu → Panel Options | Menu → dialog | — | Proxy color preset or custom color |
| Document status bar zoom box | Numeric entry | — | Must mirror the Navigator zoom level |
| `View > Zoom In` / `Zoom Out` | Menu | `Ctrl/Cmd` `+` / `-` *(inferred)* | Same view state as the panel |
| Zoom tool | Tool | `Z` | Preset-step zoom centered on click (`03-tools/hand-and-zoom.md`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Zoom | percent | 100% *(inferred)* | 1 pixel effective minimum … **3200%** maximum | Text box, slider, buttons, status bar share one value |
| Zoom slider | slider | — | maps over the zoom range | Continuous |
| Zoom step (buttons) | action | — | preset percentages | Next/previous preset, matching the Zoom tool |
| Proxy color | color | a preset *(inferred)* | preset palette or custom color | Panel Options |
| Thumbnail fit | derived | fit-to-panel | — | Aspect-preserving downscale of the document |
| Proxy size | derived | view ÷ document | ≥ 1 px | Unless set by `Ctrl`/`Cmd`-drag |
| Max magnification | const | 3200% | — | Help-sourced |
| Min view size | const | 1 px | — | Help-sourced; below/at this the magnifier appears empty |

## Algorithms & pipeline

The panel is a **view controller over per-window view state**; it performs no pixel edits.

- **Thumbnail rendering** — downscale the document composite (or its cached pyramid, `ARCH-006`) to the thumbnail widget; redraw on document change, zoom, or panel resize. On very large PSB documents this must sample from the existing image pyramid rather than a fresh full-canvas reduction.
- **Proxy rectangle** — `proxy = viewport_rect / document_rect`, mapped into thumbnail coordinates; the inverse maps a thumbnail drag back to a scroll position. Clamp so the proxy stays within the thumbnail.
- **Zoom mapping** — slider position ↔ percent is a monotonic mapping over `[min, 3200%]`; the text box accepts typed values and clamps; buttons step to the next zoom preset (`03-tools/hand-and-zoom.md`).
- **Control/Command-drag** — one gesture updates both the proxy size and position (a zoom-about-point plus recenter), then writes a single view-state update.
- **Synchronization** — all zoom surfaces (Navigator, status bar, `View` menu, Zoom tool) read/write one authoritative `ViewState`; updates are broadcast, never looped.

## Rust module mapping

- `pictura_core::view` — `ViewState { zoom: f32, scroll: Point, viewport: Rect, doc_rect: Rect }`, `zoom_to(percent)`, `pan_to(point)`, `set_proxy(rect)`; clamped constants `MIN_VIEW_PX = 1`, `MAX_ZOOM = 32.0` (3200%).
- `pictura_core::view::thumbnail` — `thumbnail_for(doc, max_size) -> ImageHandle`, reusing the image pyramid (`ARCH-006`).
- `pictura_ui_bridge::ViewportModel` — marshals `ViewState` between windows and the Navigator widget.
- `pictura_core::prefs` — proxy color, default zoom, zoom preferences (Animated Zoom, Zoom Resizes Windows, Zoom With Scroll Wheel, Zoom Clicked Point To Center) are `03-tools/hand-and-zoom.md`'s; the panel reads proxy color only.

Crossing types: `ViewState`, `Rect`, `Point`, `ImageHandle`, `DocumentId`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `NavigatorPanel` | `QDockWidget` | Host; thumbnail, zoom controls, panel menu |
| `NavigatorThumbnail` | custom `QWidget` | Paints the downscaled document + proxy rect; pan/click/ctrl-drag |
| `ProxyViewRect` | overlay item | Draggable/resizable proxy box drawn over the thumbnail |
| `ZoomSlider` | `QSlider` | Continuous zoom control |
| `ZoomSpinBox` | `QDoubleSpinBox` (or `QLineEdit`) | Typed zoom percentage with clamp |
| `ZoomButtons` | `QToolButton` pair | Zoom Out / Zoom In preset steps |
| `NavigatorMenu` | `QMenu` + `QColorDialog` | Panel Options → proxy color |
| `NavigatorController` | `QObject` | Binds `ViewportModel`; coalesces thumbnail repaints |

Widgets, not QML: a small, always-docked view instrument, consistent with `ARCH-003`. The thumbnail is a custom-painted `QWidget` fed by a Rust `ImageHandle`; it is intentionally cheap to repaint on pan/zoom.

## Data-model impact

- **View state is per document window**, never per-image pixel data and never serialized into the PSD (`ARCH-008`).
- **Workspace persistence:** a saved workspace may remember panel visibility/dock position, but the document's zoom/scroll is a live view property *(inferred)*.
- **Preferences:** proxy color; zoom behavior preferences are owned by `03-tools/hand-and-zoom.md`.
- **Undo:** zoom/pan are **not** History states.
- **Thumbnail/cache:** runtime-only; must share the document's image pyramid, not add a second full-resolution copy.

## Edge cases

- **No document open** — empty thumbnail; zoom controls disabled or neutral.
- **Single-pixel / tiny documents** — thumbnail and proxy must render without divide-by-zero; proxy may fill the thumbnail.
- **Huge (PSB) documents** — thumbnail downscale must use the existing pyramid/cache and stay off the UI thread for the first paint.
- **3200% maximum / 1-pixel minimum** — clamp exactly; the Zoom In/Out buttons and the slider must both stop at the bounds.
- **Proxy at the document edge** — pan clamping must not let the proxy leave the thumbnail or the viewport show out-of-bounds.
- **Aspect mismatch** — thumbnail preserves aspect; the proxy mapping must account for letterboxing.
- **Rotated canvas / Rotate View tool** — the thumbnail and proxy mapping must remain consistent with the rotated view (`03-tools/rotate-view.md`).
- **Window zoom vs panel zoom** — if `Zoom Resizes Windows` is on, window resizing must update the proxy/thumbnail state without a feedback loop.
- **Multiple windows on one document** — each window's zoom/scroll is independent; the Navigator must bind to the active window.
- **Theme/high-contrast** — the proxy color must remain distinguishable from the artwork under any custom color and theme.
- **Keyboard-only** — zoom text box and buttons reachable; the thumbnail/proxy pan should have a keyboard path or documented limitation.

## Parity acceptance criteria

1. Given a document with no zoom change, the Navigator thumbnail shows the whole image with the proxy box covering the entire view.
2. Given a zoom-in via the slider, text box, or Zoom In button, the document view and the proxy box both update and stay synchronized with the status-bar zoom box.
3. Given the proxy dragged in the thumbnail, the document view pans to the corresponding region.
4. Given a click on the thumbnail, the viewable area recenters on the clicked point.
5. Given `Ctrl`/`Cmd`-drag in the thumbnail, the proxy's size and position change together in one gesture.
6. Given the zoom driven to the maximum, it stops at 3200%; given the minimum, the view reaches 1 pixel and the magnifier shows empty.
7. Given **Panel Options**, choosing a preset proxy color or a custom color recolors the proxy box and persists.
8. Given a PSB document, opening the Navigator does not allocate a second full-resolution buffer and paints from the image pyramid.
9. Given the view panned to a document edge, the proxy is clamped and no out-of-bounds area is shown.
10. Given a document rotated with Rotate View, panning in the Navigator stays consistent with the rotated viewport.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded and text-extracted). Established: `Window > Navigator`; the thumbnail + **proxy view area** concept; the anatomy labels A–G (panel menu, thumbnail, proxy, zoom text box, Zoom Out, zoom slider, Zoom In); the interactions (type a value, Zoom Out/In buttons, drag slider; drag proxy to move, click thumbnail to designate the viewable area; `Control`/`Command`-drag to set size and position together); Panel Options proxy **Color** preset pop-up or custom color; the 3200% maximum magnification and 1-pixel minimum "empty magnifying glass"; setting the zoom level at the document window's lower-left corner or in the Navigator panel; the Zoom tool's preset-percentage, click-centered behavior. Zoom preferences (Animated Zoom, Zoom Resizes Windows, Zoom With Scroll Wheel, Zoom Clicked Point To Center) are owned by `03-tools/hand-and-zoom.md`.

Not used in this pass:

- `helpx.adobe.com` Navigator pages (HTTP 403 from this environment); the archived CS6 PDF was used instead.

Fetched for this revision:

- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Navigator_palette.html` — Martin Evening CS6/CS5 support page: the Navigator preview, dragging the colored rectangle to scroll, changing the rectangle color via the panel fly-out `Panel Options`, `Command`/`Control`-drag to define a zoom area, the slider and "little mountain"/"big mountain" incremental zoom buttons, clicking the thumbnail to jump to an area at the same magnification, and typing a zoom percentage (up to two decimal places). *(Page header is CS5; controls carry to CS6.)*
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Painting workspace: Navigator replaces Color in the top group of the main column.

## Open questions

- **Default zoom on open** (100% assumed) and the default proxy color/preset. *Resolves with:* a CS6 first-run capture.
- **Exact zoom preset percentages** for the Zoom In/Out buttons (and whether the slider snaps to them). *Resolves with:* `03-tools/hand-and-zoom.md` and a CS6 capture.
- **Slider-to-percent mapping** (linear vs logarithmic). *Resolves with:* a CS6 interaction test.
- **Whether the Navigator zoom is persisted per document or per workspace.** *Resolves with:* a CS6 workspace/save test.
- **Proxy color persistence scope** (global preference vs per-panel). *Resolves with:* a CS6 restart test.
- **Interaction with Rotate View / multiple windows** on one document. *Resolves with:* `03-tools/rotate-view.md` and a CS6 multi-window capture.
- **Thumbnail refresh strategy for live edits** (throttle/coalesce). *Resolves with:* `ARCH-006` gpu-rendering-pipeline and a performance spike.
