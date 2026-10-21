# Hand and Zoom Tools

- **Spec ID:** `TOOL-042`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — both options bars renamed the actual-size button to **100%** and dropped the **Print Size** button; `View > Actual Pixels` became `View > 100%`; and CS6 added high-DPI (Retina-class) viewing: `View > 200%`, `Ctrl`/`Cmd`-double-click the Zoom tool icon for 200%, `Shift+Ctrl`/`Cmd`-double-click to view all open documents at 200%.
- **Depends on:** `02-ui-ux/application-frame.md`, `02-ui-ux/keyboard-shortcuts.md`, `02-ui-ux/panels/navigator-panel.md`, `02-ui-ux/workspace-and-docks.md`, `01-architecture/qt6-ui-design.md`, `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/rust-qt-interop.md`, `TOOL-043` rotate-view

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

### Hand tool

- Moves the image within its window: select the Hand tool and drag to pan. To
  use Hand while another tool is selected, **hold the spacebar** and drag.
- **Flick Panning** — with OpenGL present, releasing mid-drag "flick[s] the
  image in the direction you want to view"; it continues moving as if still
  dragging. Enabled by `Edit > Preferences > General > Enable Flick Panning`
  (Mac: `Photoshop > Preferences > General`).
- **Scroll All Windows** — an options-bar option; with it on, dragging in one
  image scrolls all visible images. Holding `Shift` while dragging temporarily
  enables it.
- **Bird's Eye View** — hold the `H` key, click and hold in the image; the view
  zooms out to fit and a rectangle is dragged to the destination; releasing
  returns to the previous zoom on the new area. (Spring-loaded; works only
  through the `H` key, not the spacebar form.)
- **Fit On Screen** — double-click the Hand tool in the toolbox (or
  `Ctrl/Cmd+0`, or the options-bar button).
- The Hand options bar also carries the **100%** (formerly Actual Pixels)
  button; the CS6 release notes state both Hand and Zoom options bars dropped
  **Print Size**.

### Zoom tool

- Each click magnifies or reduces to the next **preset percentage** and centres
  the display around the clicked point. At the maximum magnification of
  **3200%** or the minimum size of **1 pixel**, the magnifying glass "appears
  empty."
- `Alt`/`Option` temporarily switches to zoom-out.
- **Drag a marquee** (with Scrubby Zoom off) to display that area at the highest
  possible magnification; hold spacebar mid-marquee to reposition it.
- **Scrubby Zoom** — options-bar option; drag left to zoom out, right to zoom in.
- **Continuous / Animated Zoom** — "To zoom continuously, your video card must
  support OpenGL, and Animated Zoom must be selected in the General
  preferences." Then click and hold to zoom in (`Alt`/`Option` to zoom out).
- **Resize Windows To Fit** — options-bar option; when on, the window is resized
  as the view magnifies or reduces. Off (the default), the window keeps constant
  size. `Preferences > General > Zoom Resizes Windows` extends the behaviour to
  keyboard-shortcut zooming.
- **Zoom All Windows** — options-bar option; clicking one image zooms the others
  the same relative amount (`Window > Arrange > Match Zoom` is the menu
  equivalent; `Shift`+click applies the same magnification).
- **100%** — double-click the Zoom tool, `View > 100%`, `Ctrl/Cmd+1`, or the
  options-bar button. At 100%, each image pixel is displayed by one *device*
  pixel (the most accurate view).
- `View > Zoom In` / `Zoom Out` (plus their shortcuts) become unavailable at the
  limits.

### Preferences that gate behaviour

`Edit > Preferences > General` exposes **Animated Zoom**, **Zoom Resizes
Windows**, **Zoom With Scroll Wheel**, and **Zoom Clicked Point To Center**.
`Edit > Preferences > Performance > GPU Settings` exposes **Enable OpenGL
Drawing**; the Help warns "Some Zoom tool preferences require OpenGL. If Enable
OpenGL Drawing is unavailable, your video card does not support this
technology." `Edit > Preferences > General` also has **Enable Flick Panning**.

### Multiple images

`Window > Arrange` provides `Match Zoom`, `Match Location`, and `Match All`.
With `Match All`, selecting Zoom or Hand and `Shift`+clicking/dragging one image
matches the others' zoom and location.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox navigation slot | Tool (fly-out group) | `H` / `Z` | Hand and Zoom; shortcut shared with Liquify (`H`, `Z`) |
| Options bar (Hand) | Tool options | `H` | Scroll All Windows, Fit On Screen, 100% |
| Options bar (Zoom) | Tool options | `Z` | Zoom In/Out, Scrubby Zoom, Resize Windows To Fit, Zoom All Windows, Fit On Screen, 100% |
| View menu | Menu | n/a | Zoom In, Zoom Out, Fit On Screen, 100%, 200%, Fill Screen |
| View > Actual Pixels | Menu | `Ctrl/Cmd+1` | Renamed to 100% in CS6 |
| Fit On Screen | Menu / double-click Hand | `Ctrl/Cmd+0` | |
| Zoom In / Zoom Out | Menu | `Ctrl/Cmd++` / `Ctrl/Cmd+-` | Next/previous preset level |
| Temporary Hand | Modifier | hold `Spacebar` | Works with any tool |
| Temporary Zoom | Modifier | hold `Spacebar+Ctrl/Cmd` | Community source; not in the CS6 shortcut table |
| Zoom out temporarily | Modifier | `Alt`/`Option` | With Zoom active |
| Bird's Eye View | Modifier + drag | hold `H`, click-hold, drag | Not the spacebar form |
| High-DPI 200% | Menu / modifier | `View > 200%`; `Ctrl/Cmd`-dbl-click Zoom icon | CS6 new; add `Shift` to apply to all open docs |
| Navigator panel | Dock | `Window > Navigator` | Proxy view area drag; zoom slider/text |
| Window > Arrange | Menu | n/a | Match Zoom / Match Location / Match All; Tile/Cascade/Float |
| Preferences > General | Pane | `Ctrl/Cmd+K` | Animated Zoom, Zoom Resizes Windows, Zoom With Scroll Wheel, Zoom Clicked Point To Center, Enable Flick Panning |
| Preferences > Performance | Pane | n/a | Enable OpenGL Drawing (GPU Settings) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Zoom level | percent | fit on open | 1 preset step … 3200% (max); min = 1 image pixel | Preset ladder, not continuous, unless animated |
| Zoom presets (observed) | enum | 100 | 25, 33.3, 50, 66.7, 100 % | Community source; full ladder below is inferred |
| Zoom presets (full, inferred) | enum | — | 1, 2, 3, 4, 6, 8, 12, 16, 25, 33.3, 50, 66.7, 100, 200, 300, 400, 600, 800, 1200, 1600, 2400, 3200 % | Photoshop's standard ladder; exact CS6 set unverified |
| 200% view | command | — | — | CS6 high-DPI addition |
| Scrubby Zoom | bool | On *(inferred; community source)* | on / off | Zoom options bar |
| Resize Windows To Fit | bool | Off | on / off | Zoom options bar |
| Zoom Resizes Windows | bool | Off *(inferred)* | on / off | General preference |
| Zoom All Windows | bool | Off | on / off | Zoom options bar |
| Scroll All Windows | bool | Off | on / off | Hand options bar; `Shift` for temporary |
| Animated Zoom | bool | On when OpenGL present *(inferred)* | on / off | General preference; requires OpenGL |
| Zoom With Scroll Wheel | bool | Off *(inferred)* | on / off | General preference |
| Zoom Clicked Point To Center | bool | Off *(inferred)* | on / off | General preference |
| Enable Flick Panning | bool | On *(inferred)* | on / off | General preference; OpenGL for flick |
| Enable OpenGL Drawing | bool | On when supported | on / off | Performance > GPU Settings |
| Pixel Grid visibility threshold | percent | 500 | auto above ~500% | `View > Show > Pixel Grid` to hide |

## Algorithms & pipeline

The Hand and Zoom tools do not alter document pixels. They maintain a
**viewport transform** — the mapping from document space to window (device)
space — and request a re-render at the new transform.

```text
ViewportTransform {
    scale: f32,            // document px -> device px
    translation: Vec2,     // device px
    rotation_deg: f32,     // owned by TOOL-043
}
```

- **Zoom to a point** (`Zoom Clicked Point To Center` on): the document point
  under the cursor is the fixed point of the scale change, so it stays under the
  cursor. With the preference off, the centre of the window is the fixed point
  *(inferred from the preference naming)*.
- **Preset stepping**: `View > Zoom In/Out` and a Zoom-tool click move to the
  next/previous value in the preset ladder, not by a fixed multiplier. The
  ladder is chosen so common ratios land on exact device-pixel mappings.
- **Animated (continuous) zoom**: an interpolated scale animation (per-frame
  `scale = lerp(scale, target, t)`) driven while the button is held; requires
  OpenGL. Without a GPU, use discrete stepping (the documented fallback).
- **Scrubby Zoom**: horizontal drag distance maps to a logarithmic scale delta
  (`scale *= exp(k * dx)`); exact mapping is *(inferred)*.
- **Flick panning**: the Hand tool integrates pointer velocity on release and
  applies inertial decay to `translation` until a stop threshold; OpenGL-backed.
  Exact friction is *(inferred)*.
- **Bird's Eye View**: while active, render a fit-to-window proxy and map the
  dragged rectangle back to a viewport transform at the stored zoom.
- **Fit On Screen**: choose `scale = min(win_w/doc_w, win_h/doc_h)` (and resize
  the window if Resize Windows To Fit / Zoom Resizes Windows is on).
- **Rendering**: the transform is applied on the view, not by resampling the
  document (`ARCH-003`); the GPU compositor renders the visible tiles at the new
  scale. Above ~500% the pixel grid overlay becomes visible.

## Rust module mapping

Proposals.

- `pictura_render::viewport` — `ViewportTransform` (scale, translation,
  rotation), `ViewportState` (per-window), and pure operations: `zoom_to_point`,
  `step_preset`, `fit_on_screen`, `zoom_to_rect`, `clamp_scale`.
- `pictura_render::viewport::presets` — the zoom-level ladder and
  `next_level(current, dir, presets)`.
- `pictura_tools::navigate` — `HandTool`, `ZoomTool`, shared
  `NavigateOptions { resize_windows_to_fit, scrubby, zoom_all_windows,
  scroll_all_windows }`; `FlickState { velocity, decay }`; `BirdseyeState`.
- `pictura_tools::navigate::group` — multi-window operations: `scroll_all`,
  `zoom_all`, `match_zoom`, `match_location`, `match_all`.
- `pictura_ui::view_store` (bridge) — persists and broadcasts per-document view
  state across windows of the same document.

Crossing types: `ViewportTransform` (copy, cheap), `ZoomLevel(f32)`,
`Point2Device`. Zoom/pan must not allocate tiles or touch the document graph.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `CanvasView` | `QGraphicsView` | Pointer capture, pan/zoom gesture routing, proxy overlay, `mapFromScene` for document points |
| `CanvasWidget` | `QRhiWidget` | Renders the transformed colour buffer on the GPU; `devicePixelRatio` aware |
| `NavigationOptionsWidget` | `QWidget` (options bar) | Hand vs Zoom options (toggles/buttons above) |
| `NavigatorPanel` | `QWidget` dock | Thumbnail + proxy view rect, zoom slider/text box; already proposed in `02-ui-ux/panels/navigator-panel.md` |
| `ViewStore` | `QObject` | Per-document view state shared across windows; emits transform changes |
| `ScreenModeController` | `QObject` | `View > Screen Mode`; `F` / `Shift+F` cycling |

Qt specifics: wheel zoom/pan via `QWheelEvent` (with `Alt`/`Ctrl` modifiers);
trackpad pinch/rotate via `QNativeGestureEvent` (`beginNativeGesture`/
`nativeGesture`), which is the Qt6 analogue of the Mac trackpad gestures the CS6
Help documents. Window resizing uses `QWidget::resize`/`setFixedSize`. Because
`QGraphicsView` and `QRhiWidget` must not share a broken GL context
(`ARCH-003`), the transform math lives in Rust and both widgets consume it.

## Data-model impact

- **View state is not document content.** Zoom, scroll, and (per `TOOL-043`)
  rotation live in a per-window `ViewportState`, persisted in the saved workspace
  / session (`02-ui-ux/workspace-and-docks.md`), not in PSD/PSB image resources
  for anything the CS6 sources establish. Whether CS6 writes any view metadata
  into the PSD is *unverified*.
- **No history states.** View changes are not undoable in CS6 *(inferred)* and
  must not be recorded in `ARCH-009`; `Ctrl+Z` during a zoom does nothing.
- **Navigator proxy** is derived from the rendered composite; it is a cache
  (`ARCH-006`), not serialized.
- **Multi-window**: one document may be open in several windows (`Window >
  Arrange > New Window for …`), so `ViewportState` is keyed by window, with the
  document providing only content. Match Zoom/Location/All are cross-window
  operations.
- **Preferences**: the toggles above persist in the preference store
  (`11-cross-cutting/preference-storage.md`), not in documents.

## Edge cases

- **Max/min zoom** — at 3200% and at the 1-pixel minimum the Zoom tool shows an
  empty magnifier and `Zoom In/Out` disable; stepping must not overshoot or wrap.
- **1-px and huge (PSB) documents** — Fit On Screen and the preset ladder must
  handle extreme aspect ratios; huge documents must render without allocating a
  full-resolution device buffer (tile-based render).
- **High DPI (Retina-class)** — `devicePixelRatio > 1` means 100% is one image
  pixel per *device* pixel; the transform must be expressed in device pixels and
  the canvas buffer sized `widget * DPR` (`ARCH-003`). `View > 200%` and the
  Ctrl-double-click shortcuts are the CS6 high-DPI affordances.
- **GPU unavailable** — Animated Zoom, Flick Panning, and OpenGL-gated
  preferences turn off; stepping and panning still work on the CPU path. The UI
  should reflect that rather than silently ignoring the option.
- **Trackpad gestures** — on Linux these arrive via `QNativeGestureEvent`
  (X11/Wayland support varies); CS6's `Enable Gestures` preference (Mac-only) has
  no exact Linux equivalent and needs a documented mapping.
- **Multiple windows of one document** — per-window state; closing one window
  must not delete the document if another window is open.
- **Window resize** — with Resize Windows To Fit on, zooming resizes the
  window; do not fight the window manager on tiled Wayland compositors
  *(inferred limitation)*.
- **Screen modes** — Fit On Screen interacts with Full Screen Mode; the
  `F`/`Shift+F` cycle and canvas colour (`Space+F`) are view-only.
- **Mid-animation input** — a new zoom/pan during an animated zoom or flick
  should interrupt cleanly, not queue.
- **Undo/redo** — view operations must never enter history, even when performed
  during a history-sensitive operation.

## Parity acceptance criteria

- Given a document and the Zoom tool, a click steps to the next preset level in
  the ladder and centres on the clicked point; `Zoom In/Out` step in the opposite
  direction and disable at 3200% / the minimum.
- Given `View > Fit On Screen` (`Ctrl/Cmd+0`) or a double-click of the Hand tool,
  the whole document is visible and the window scales appropriately when Resize
  Windows To Fit is on.
- Given `View > 100%` (`Ctrl/Cmd+1`), one image pixel maps to one device pixel,
  verified against `devicePixelRatio`.
- Given `Scrubby Zoom` on, a horizontal drag changes the zoom continuously and
  the direction matches (right = in, left = out).
- Given Animated Zoom on and an OpenGL-capable GPU, click-and-hold produces
  continuous zoom; given no GPU, the option is disabled/unavailable and discrete
  stepping still works.
- Given `Scroll All Windows` on and two tiled documents, dragging in one pans
  both; holding `Shift` with the option off has the same effect.
- Given `Zoom All Windows` on, clicking one image changes the others'
  magnification by the same relative amount.
- Given `Window > Arrange > Match All` and a `Shift`+click, all images match both
  zoom and location.
- Given the spacebar is held while another tool is active, the cursor becomes the
  Hand and dragging pans; releasing restores the previous tool without committing
  any document change.
- Given `H` is held and the image clicked, Bird's Eye View zooms out and a
  dragged rectangle relocates the view at the prior zoom.
- Given a zoom or pan, the History panel gains no state and `Ctrl+Z` does not
  revert it.
- Given `devicePixelRatio = 2`, the canvas renders crisply at 2× and 100% is
  pixel-accurate.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: Hand
  drag/pan and spacebar access; Flick Panning and `Enable Flick Panning`;
  Scroll All Windows and the `Shift` temporary form; Fit On Screen via
  double-click or `Ctrl/Cmd+0`; Zoom preset step with max 3200% / min 1 pixel;
  `Alt`/`Option` zoom-out; the zoom marquee and spacebar repositioning; Scrubby
  Zoom; continuous zoom requiring OpenGL + Animated Zoom; Resize Windows To Fit
  and `Zoom Resizes Windows`; the General preferences (Animated Zoom, Zoom
  Resizes Windows, Zoom With Scroll Wheel, Zoom Clicked Point To Center); Enable
  OpenGL Drawing under Performance > GPU Settings; the Navigator panel, Zoom
  All Windows, Scroll All Windows, and the Window > Arrange Match
  Zoom/Location/All commands;
  Bird's Eye View; the shortcut table (`H`, `Z`, `Ctrl+0`, `Ctrl+1`,
  `Ctrl++`/`Ctrl+-`); the CS6 "Changes in view options" (rename to 100%, Print
  Size removed, View > Actual Pixels → 100%) and high-DPI additions (View > 200%,
  Ctrl/Cmd-double-click the Zoom icon, Shift+Ctrl/Cmd for all open docs).
- `https://www.photoshopessentials.com/basics/photoshop-zoom` — community
  tutorial (CC-era, CS6-compatible in scope) corroborating the preset ladder
  (25 / 33.3 / 50 / 66.7 / 100), Fit On Screen, 100%, Scrubby/Continuous zoom,
  scroll-wheel behaviour, Hand panning, flick panning, and Bird's Eye View.
  Secondary source.
- SearXNG meta-search (query: "Photoshop Zoom tool preset zoom levels list
  3200% 100% 66.7% 50%") — used to locate the secondary source; no facts from
  snippets alone.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Exact CS6 zoom preset ladder.** Only 25 / 33.3 / 50 / 66.7 / 100 are sourced;
  the full ladder (and whether it is identical to later versions) is inferred.
  Resolves with: a CS6 `View > Zoom In` step capture or the Help PDF's
  magnification reference.
- **`Actual Pixels` vs `100%` in CS6.** The CS6 "Changes in view options" says
  the label was renamed to 100%, but the Help body still prints "(CS5 and CS6)
  Choose View > Actual Pixels". Resolves with: a CS6 menu/options-bar screenshot
  or the service-release notes.
- **Defaults for Scrubby Zoom, Zoom With Scroll Wheel, Zoom Clicked Point To
  Center, Zoom Resizes Windows, Animated Zoom, Flick Panning.** Most are inferred.
  Resolves with: a first-run CS6 preferences capture.
- **Rotation persistence across sessions.** Zoom/scroll are session UI state;
  whether CS6 persists them per document or only in the workspace is unverified.
  Resolves with: a CS6 save/reopen and restart test. Feeds `TOOL-043`.
- **Optimal GPU zoom path.** Fit vs nearest-neighbour at non-100% levels, and the
  filtering policy above 100%, need a rendering decision (`ARCH-006`). Resolves
  with: a side-by-side rendering comparison against CS6.
- **Linux trackpad/gesture mapping.** CS6 documents Mac gestures; the X11/Wayland
  `QNativeGestureEvent` mapping and any compositor limitations are unresolved.
  Resolves with: a Qt6 gesture test on both display servers.
- **Multi-window layout parity.** CS6's Tile/Cascade/Float arrangements predate
  Wayland tiling; which arrangements are supportable on Linux is a UI decision.
  Resolves with: `02-ui-ux/application-frame.md`.
