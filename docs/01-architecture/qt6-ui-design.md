# Qt6 UI Design

- **Spec ID:** `ARCH-003` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` (this document covers the UI technology choice, not a CS6 feature)
- **Depends on:** `ARCH-001` system-architecture, `ARCH-004` rust-qt-interop, `ARCH-006` gpu-rendering-pipeline, `02-ui-ux/application-frame.md`, `02-ui-ux/workspace-and-docks.md`

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the
> behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not.

## CS6 behavior

Photoshop CS6 presents a top-level application frame with a menu bar, an options
bar that changes with the active tool, a document tab strip when more than one
document is open, a tool panel on the left, and a set of dockable panels on the
right (Layers, Channels, Paths, History, Color, Swatches, Adjustments, and
others). Panels can be collapsed, tabified, floated, and rearranged, and the
arrangement can be saved as a workspace.

CS6 introduced a darker application interface than CS5 and exposed a small set of
interface brightness choices. The exact panel geometry, tab behavior, gridding,
and theme values are owned by the `02-ui-ux/` specs and are **not re-sourced in
this document**; see `## Open questions`. What constrains this architecture
document is the shape of the frame: many long-lived, rearrangable, stateful
panels around a large central canvas, with per-panel scrollable and virtualized
content.

The canvas must support interactive pans, zooms, and rotation with a ruler and
guide overlay, and it must show GPU-composited document content at interactive
latency. That blends a dense widget chrome problem with a high-throughput
rendering surface in the same window.

## UI surface

The architecture decision applies to every visible surface. The table lists the
frame elements and the panel families that the choice must carry.

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Application frame | `QMainWindow` | n/a | Menu bar, options bar, status bar, dock areas |
| Canvas / document view | Central widget | `Ctrl+0`, `Space` drag | GPU-composited; rulers, guides, transform handles |
| Tool panel | Dock widget, single/double column | `Tab`, tool letters | Long-lived, fixed width, icon grid |
| Layers, Channels, Paths, History | Dock widgets, tabbed | `F7`, etc. | Virtualized lists/trees, drag-and-drop |
| Color, Swatches, Adjustments | Dock widgets | `F6`, etc. | Palette grids, sliders, live thumbnails |
| Options bar | Tool bar | n/a | Rebuilt per active tool |
| Floating / collapsed panels | Top-level `QDockWidget` | double-click title | Must survive window moves and monitor changes |

## Parameters & ranges

Values the UI layer consumes from configuration. Ranges marked "provisional"
must be fixed by the `02-ui-ux/` specs.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Interface brightness | enum | Dark | Dark / Medium / Light | CS6-style theme; exact values TBD |
| Panel width | int (px, DIP) | provisional | e.g. 200 to 480 | Per panel, persisted |
| Dock tab position | enum | Bottom | Bottom / Top / Left / Right | Maps to `QMainWindow::setTabPosition` |
| Dock nesting | bool | false | on / off | `AllowNestedDocks`; off keeps drag behavior simple |
| Dock animation | bool | true | on / off | `AnimatedDocks` |
| Icon size | `QSize` (px) | style default | e.g. 16, 24, 32 | High-DPI assets selected by `QIcon` |
| Device pixel ratio | float | platform | 1.0, 1.25, 1.5, 2.0, ... | Read from `QWindow::devicePixelRatio()` |
| Canvas zoom | float | fit | ~0.1% to 3200% | Transform on the view, not the widget |
| Workspace | string | "Essentials" | named set | Persisted via `saveState()` |

## Algorithms & pipeline

### Decision: Qt Widgets shell, not Qt Quick/QML

The application frame, docks, menus, and panels are built with **Qt Widgets**.
Qt Quick/QML is used only for isolated, self-contained surfaces if a later spec
demonstrates a concrete need.

Rationale, grounded in the Qt documentation fetched for this document:

- `QMainWindow` provides the dock/tab/toolbar/central-widget framework directly.
  It has four dock areas, tabified docking, corner ownership, nested docks, and
  `saveState()`/`restoreState()` persistence keyed by `objectName`. Rebuilding
  this in a QML scene would mean reimplementing docking, floating, tab
  drag-and-drop, and workspace persistence, which is the densest part of a
  professional app (`QMainWindow`, `QDockWidget` docs).
- `QDockWidget` already covers floating, closable, movable panels with custom
  title bars via `setTitleBarWidget()`, and `toggleViewAction()` for the
  Window menu. These are exactly the CS6 panel behaviors.
- Widgets are dense, keyboard-centric, and DPI-aware through `QPainter` and the
  device-independent coordinate system. A panel-heavy editing UI benefits from
  the mature `QAbstractItemView` delegate machinery for Layers/Channels.
- `QQuickWidget`, the option for embedding QML in a widget window, costs an
  extra offscreen render pass and **disables the threaded render loop on all
  platforms** (Qt Quick Widgets docs). That is a poor default for the canvas.
- QML's strengths (fluid declarative animation, touch, GPU scene graph) are not
  the primary requirements of a mouse-and-keyboard editor whose canvas is a
  custom raster/GPU surface, not a scene graph of QML items.

Qt Quick remains viable for specific overlays (for example, an animated start
screen or a touch-oriented workspace selector). If adopted, it is embedded via
`QQuickWidget`, with the documented cost accepted per surface.

### Canvas rendering path

The canvas is a **custom widget** with a GPU-backed color buffer, not a scene of
`QGraphicsItem`s for the pixels. Two supported hosts exist in Qt 6.7+:

- `QRhiWidget` for a widget-based window. It manages a backing `QRhiTexture`,
  a depth/stencil buffer, and a render target, and calls the subclass's
  `initialize()`/`render()`. It is the portable, non-OpenGL-specific analogue of
  `QOpenGLWidget` and supports Vulkan, Metal, D3D11/12, and OpenGL.
- `QQuickRhiItem` if the canvas is later hosted inside Qt Quick. It enforces a
  strict item/renderer split and runs the renderer on the scene-graph thread.

`QGraphicsView` is used for **vector overlays**, not pixel compositing: rulers,
guides, selection marching ants, transform handles, pen/path previews. The view
supplies affine transforms (`scale`, `rotate`, `translate`), scene/view mapping
(`mapToScene`, `mapFromScene`), hit testing (`itemAt`, `items`), and a configurable
`viewportUpdateMode` (`MinimalViewportUpdate` by default). Its `backgroundBrush`
and `drawForeground()` can paint the document image under the vector items. This
keeps handle hit-testing and snapping in a mature 2D framework while the pixel
data comes from the GPU pipeline in `ARCH-006`.

Composition order per frame: paint the composited document texture (from the
`ARCH-006` pipeline) as the view background, then the overlay layer (guides,
handles, selection), then the tool cursor and HUD.

### Panel view models

Layers, Channels, Paths, History, and similar panels are backed by custom
`QAbstractItemModel` subclasses (tree for layer groups, list for history and
channels). The model exposes document state as roles; a `QStyledItemDelegate`
draws thumbnails, visibility toggles, blend-mode text, and lock icons. This is
the standard model/view split, and it keeps the panel widgets decoupled from the
document model.

Thread rule (important): `QAbstractItemModel` is not thread-safe and its API
must only be called from the thread the model lives in, which is the GUI thread
when a view is attached. Background work must queue changes and apply them on
the main thread, for example with queued connections (`QAbstractItemModel` docs).
This constraint shapes `ARCH-005`.

### Styling: why the built-in style is insufficient

Qt draws widgets through `QStyle`, and each platform style attempts to look like
the native platform. CS6 has a specific dark, flat, low-chrome look that does not
match Fusion, Windows, or macOS styles. The plan:

1. Set a base dark `QPalette` and select the Fusion style as a neutral,
   cross-platform base (Fusion is documented as a desktop style in the Qt Widgets
   module).
2. Subclass `QProxyStyle` (or `QStyle`) for the widgets that carry the CS6
   identity: dock title bars, tab bars, tool buttons, group headers, sliders,
   checkboxes, and combo boxes. `QDockWidget` supports a fully custom title bar
   through `setTitleBarWidget()`, including parent access via `qobject_cast` for
   docking and hiding behavior.
3. Use Qt Style Sheets for narrow, local adjustments where subclassing is
   disproportionate. Style sheets are applied sparingly because they interact
   with custom `paintEvent` code and can defeat native-style optimizations.

The visual contract (colors, metrics, states) is specified in `02-ui-ux/`, not
here. This document only fixes the mechanism: palette + `QStyle` subclass +
limited style sheets, with a single `Theme` object as the source of truth so the
three brightness levels and future themes do not fork the widget code.

### HiDPI

Qt 6 already scales application geometry in device-independent pixels, so widget
and item geometry, event geometry, and screen geometry are all DIP; only raw
image buffers and low-level graphics are in device pixels. Consequences:

- Static icons are shipped at multiple densities (`icon@2x.png` etc.) in a
  `QIcon`; Qt selects the best representation at runtime.
- `QImage`/`QPixmap` are raw pixel buffers. A document thumbnail or a rendered
  tile is sized in device pixels and assigned a device pixel ratio before drawing.
- The canvas color buffer follows the widget size multiplied by
  `QWindow::devicePixelRatio()`, exactly as `QRhiWidget` documents for its
  managed texture. A 2x scale means the GPU renders the canvas at 2x.
- Test with `QT_SCALE_FACTOR` to simulate scale factors without hardware; the
  effective ratio is the product of the set factor and the native ratio.
- Fractional scaling (1.25, 1.5) is expected on Linux. `PassThrough` is the
  default rounding policy; a per-app policy can be set if artifacts appear.

## Rust module mapping

Proposals. The UI shell is largely C++; Rust supplies the document-backed models
and the canvas renderer through the bridge in `ARCH-004`.

- `pictura_ui::shell` — frame setup, dock registration, workspace save/restore.
- `pictura_ui::theme` — palette and `QStyle` metric tokens; no widget code.
- `pictura_ui::models::layers` — `QAbstractItemModel` implementation over the
  document layer tree; roles for name, visibility, opacity, blend mode, lock,
  thumbnail handle.
- `pictura_ui::models::channels` — list model over color and alpha channels.
- `pictura_ui::models::history` — list model over history states; immutable
  snapshots referenced by id.
- `pictura_ui::canvas` — renderer host implementing the `QRhiWidget` subclass
  contract; consumes the pipeline from `ARCH-006`.
- `pictura_ui::icons` — icon set lookup and `@2x` asset resolution.

Data crossing the bridge: `QModelIndex` row/column identifiers, opaque document
node ids (`u64`), and small value structs (name, opacity `f32`, flags). Image
buffers do **not** cross as Qt image types per tile; see `ARCH-006`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PicturaMainWindow` | `QMainWindow` | Dock areas, menu/tool/status bars, workspace state, `createPopupMenu()` |
| `PicturaDock` | `QDockWidget` | Panel host; custom title bar via `setTitleBarWidget()`; `toggleViewAction()` in Window menu |
| `LayersModel` | `QAbstractItemModel` | Tree over layers/groups; thumbnail, visibility, lock roles |
| `HistoryModel` | `QAbstractListModel` | History states and snapshots |
| `CanvasView` | `QGraphicsView` | Vector overlay, transforms, hit testing, viewport update mode |
| `CanvasWidget` | `QRhiWidget` | GPU color buffer host; `initialize()`/`render()`; API set early via `setApi()` |
| `PanelHeader` | `QWidget` | Custom title bar widget with collapse/close and drag hints |
| `PicturaStyle` | `QProxyStyle`/`QStyle` | CS6 visual metrics and control drawing |
| `ThresholdSlider`, `ColorSwatchGrid`, `HistogramView` | `QWidget` | Dense custom controls not covered by stock widgets |

Widgets-vs-QML decision: **Widgets for the shell and panels; QML only for
isolated surfaces behind a demonstrated need.** Reasons are in
`## Algorithms & pipeline`.

## Data-model impact

The UI layer never owns document data. It holds:

- `NodeId(u64)` handles into the document model, stable across undo/redo where
  possible.
- Model roles emitted on change: display name, visibility, opacity, blend mode,
  lock flags, thumbnail generation counter.
- Persisted UI state: dock layout (`saveState()` byte array), panel widths,
  active tool, zoom, scroll, workspace name, theme brightness. None of this is
  document data and none of it enters PSD/XMP.

Undo granularity is a document concern (`ARCH-007` undo-history). The UI emits
intents; the command layer (`ARCH-005`) decides undo records. `dataChanged()` is
emitted only after a command commits, so a model never shows a state that undo
would not reproduce.

## Edge cases

- **Floating panels on multi-monitor.** A floated `QDockWidget` is a top-level
  window. Geometry must be clamped to available screen geometry on restore;
  screen geometry is in DIP and may have gaps under fractional scaling.
- **Wayland.** Docking, floating, and window positioning are compositor-controlled;
  `saveState()` restores relative layout, not absolute screen position. Absolute
  positions must not be assumed.
- **High-DPI change at runtime.** `devicePixelRatio` can change when a window
  moves between displays. Icon assets and the canvas color buffer must be
  rebuilt. `QMainWindow::iconSizeChanged` and screen-change signals drive this.
- **Very many panels.** Nested docks plus tabification can produce deep layouts;
  `AllowNestedDocks` is off by default to keep drag behavior predictable.
- **QML embedded surfaces.** `QQuickWidget` disables the threaded render loop and
  costs an extra pass; a window can use only one graphics API at a time, so a
  Vulkan `QRhiWidget` and an OpenGL `QOpenGLWidget` must not share a window.
- **Software rendering.** If no GPU is available, the canvas falls back to a
  raster path; the shell must still work. See `ARCH-006`.
- **Accessibility.** Custom-painted controls must expose accessible names, roles,
  and keyboard focus; see `02-ui-ux/accessibility.md`.

## Parity acceptance criteria

- Given a two-panel dock layout and a saved workspace, restarting the app
  restores dock position, tabification, panel widths, and active tab within one
  pixel of the saved state at the same scale factor.
- Given a panel is floated and the window is moved to a second monitor with a
  different `devicePixelRatio`, the panel remains fully on-screen and its icons
  re-resolve to the correct density within one frame.
- Given a `LayersModel` change is emitted from a worker thread, no model API is
  touched off the GUI thread and the view updates exactly once per committed
  command.
- Given `QT_SCALE_FACTOR=2`, all widget geometry, dock layout persistence, and
  the canvas color buffer scale consistently; the canvas renders at 2x device
  pixels with no blur.
- Given the GPU is unavailable, the shell starts and the canvas displays a
  CPU-composited image.
- Given the CS6 visual contract from `02-ui-ux/`, a screenshot diff of the
  default workspace against a CS6 reference meets the tolerance defined there
  (tolerance TBD; see `## Open questions`).

## Sources

Fetched for this document:

- `https://doc.qt.io/qt-6/qdockwidget.html` — dock widget behavior, features,
  `setTitleBarWidget`, `toggleViewAction`, `visibilityChanged`.
- `https://doc.qt.io/qt-6/qmainwindow.html` — dock areas, `DockOptions`
  (`AllowNestedDocks`, `AllowTabbedDocks`, `VerticalTabs`), `saveState`/
  `restoreState`, `objectName` requirement, `createPopupMenu`.
- `https://doc.qt.io/qt-6/qabstractitemmodel.html` — model/view contract,
  `index`/`parent`/`rowCount`/`data`, `begin/end` insert/remove, `dataChanged`,
  thread-safety rule.
- `https://doc.qt.io/qt-6/qquickwidget.html` — offscreen pass cost, threaded
  render loop disabled, one graphics API per window, `grabFramebuffer`.
- `https://doc.qt.io/qt-6/qgraphicsview.html` — transforms, `mapToScene`/
  `mapFromScene`, `itemAt`/`items`, `ViewportUpdateMode`, `viewport()`.
- `https://doc.qt.io/qt-6/qrhiwidget.html` — backing texture, `initialize`/
  `render`, `setApi` early, device-pixel-ratio sizing, single API per window.
- `https://doc.qt.io/qt-6/highdpi.html` — device-independent pixels, device pixel
  ratio, `@2x` assets, `QT_SCALE_FACTOR`, X11/Wayland configuration, coordinate
  systems.
- `https://doc.qt.io/qt-6/qtwidgets-index.html` — widget/module overview, styles,
  model/view and Graphics View framework, CMake integration.
- `https://doc.qt.io/qt-6/topics-graphics.html` — graphics API landscape,
  `QRhiWidget`/`QQuickRhiItem` as widget/Quick hosts.

Not used as a source in this pass (see `## Open questions`):

- Adobe Photoshop CS6 Help reference PDF (intended primary source for the CS6 UI
  contract; not fetched in this pass).
- `https://www.pcworld.com/article/464868/...` — returned HTTP 403.

## Open questions

- **CS6 interface theme values.** Exact background, panel, border, and text
  colors, panel metrics, and the three brightness levels are not sourced here.
  Resolve by fetching the archived Photoshop CS6 Help PDF and a reliable CS6
  screenshot reference, then specifying them in `02-ui-ux/`.
- **CS6 dock/tab semantics.** Whether CS6 allowed nested splits and how tabified
  panels behaved on drag is not verified in this pass. Resolve from the CS6 Help
  PDF (`Window > Workspace`) or a captured UI reference.
- **QGraphicsView versus a pure custom overlay widget.** `QGraphicsProxyWidget`
  with an OpenGL/RHI viewport has documented limitations for embedded widgets.
  Resolve with a prototype comparing a `QGraphicsView` overlay against a custom
  `QWidget` overlay on `QRhiWidget`.
- **QML adoption threshold.** What surface, if any, justifies its offscreen-pass
  and threaded-render-loop costs. Resolve with a profiling spike.
- **Screenshot-diff tolerance.** The parity harness needs a numeric tolerance for
  font rendering and antialiasing differences. Resolve in
  `11-cross-cutting/testing-strategy.md`.
- **Accessibility of custom-painted panels.** Screen-reader and high-contrast
  behavior for `PicturaStyle`-drawn controls is unspecified. Resolve in
  `02-ui-ux/accessibility.md`.
