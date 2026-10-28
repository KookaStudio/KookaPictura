# Application Frame

- **Spec ID:** `UI-001`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the CS5 **Application bar** was removed; workspace switching/options moved to the **Options bar**, document layout to `Window > Arrange`, and screen-mode switching to a button at the bottom of the toolbar; the draggable title bar was reduced by **over 30%**; the interface gained **four color themes** (Black, Dark Gray, Medium Gray, Light Gray), on-image displays, and a per-screen-mode canvas colour chosen in the Interface preferences.
- **Depends on:** `ARCH-003` qt6-ui-design, `UI-002` menus, `UI-003` workspace-and-docks, `UI-004` toolbox-and-options-bar, `ARCH-006` gpu-rendering-pipeline

> All module and widget names below are **design proposals** (a minimal M16
> shell now exists under `crates/pictura-app`; proposal names may differ from the
> shipped ones). Facts confirmed by a fetched CS6 source are stated plainly;
> anything reconstructed from secondary sources or inference is marked
> *(secondary)*, *(inferred)* or `(to verify)`.

## CS6 behavior

### The application frame

Photoshop CS6 groups its workspace elements into an **application frame**: a
single integrated window containing the menu bar, options bar, document
windows, Tools panel, and docks. Moving or resizing the frame moves/resizes its
contents together so panels do not overlap, and panels do not disappear when the
user switches applications or clicks outside the application. On **Windows the
frame is always on**; on **Mac** the CS6 Help says the free-form interface can be
turned off, and the CS6-for-Photographers guide confirms that for Photoshop the
toggle is `Window > Application Frame` *(secondary)*.

CS6 specifically **removed the Application bar** that CS5 had across the top.
The CS6 Help "What's New" states that "functions formerly in the application bar
have moved elsewhere" and that the draggable title bar was reduced by **over
30%**. The CS6-for-Photographers guide resolves the control-by-control relocation
for Photoshop: the Application bar is gone, **workspace options moved to the
Options bar**, **document-layout options became available solely via
`Window > Arrange`**, and the **screen-mode button moved to the bottom of the
toolbar** (the "What's New" text also names the toolbar button) *(secondary; the
generic workspace chapter still describes the pre-CS6 Application-bar layout)*.

### Menu bar (Windows) vs. application menu (Mac)

- The CS6 Help's workspace overview says the **Application bar** contained
  "menus (Windows only)". On **Windows**, the application menus (File … Help)
  live in a menu bar across the top of the frame.
- On **Mac OS**, Photoshop follows the platform convention: a **Photoshop
  application menu** (Photoshop > About, Preferences, Services, Hide, Quit) sits
  in the system menu bar, and product preferences are reached via
  `Photoshop > Preferences` instead of `Edit > Preferences`. The Help writes
  `Choose Edit > Preferences (Windows) or Photoshop > Preferences (Mac OS)`
  throughout, confirming the split.
- The user-facing spec convention for Kooka Pictura: expose menu definitions
  once, with a platform mapping layer (see `UI-002`). On Linux there is no
  application menu, so the Windows-style menu bar is the model, with
  `Edit > Preferences` (or a first-class `Preferences` entry in an app menu if
  the desktop uses a global menu bar).

### Document windows and tabs

- When more than one file is open, document windows are **tabbed** by default.
  Drag a tab to reorder; drag a tab out of the group to untab/float it.
- `Window > Arrange > Float in Window` floats one document; `Window > Arrange >
  Float All In Windows` floats all. `Window > Arrange` also has `Tile`,
  `Consolidate All to Tabs`, `Match Zoom`, `Match Location`, `Match All`, and
  `New Window For [file]`.
- Documents can be grouped and docked; CS6 also supports **stacked/tiled
  groups** of documents via drop zones along a window's top/bottom/sides. The
  generic CS6 workspace chapter still describes a Layout button on the Application
  bar, but Photoshop CS6 removed that bar and moved document-layout options to
  `Window > Arrange` *(secondary)*.
- New in CS6 on Windows: **New/Open document commands by context-clicking
  document tabs** (previously Mac-only).
- Tabbed display is governed by the Interface preference **Open Documents As
  Tabs** (default on); **Enable Floating Document Window** is a sibling Interface
  preference *(secondary)*.

### Title bar

- Every document window has a title bar showing the document name and, for the
  active layer, the active layer name ("The name of the active layer appears in
  the title bar of the document"). A Digimarc-detected watermark adds a copyright
  symbol to the title bar.
- CS6 reduced the draggable title bar height by over 30% to reclaim screen space.
  The exact pixel metric is not published; `(to verify)` from a CS6 screenshot.

### Status bar

- The **status bar** sits at the bottom of every document window. It shows
  current magnification and file size of the active image, plus brief
  instructions for the active tool.
- Clicking the triangle in its bottom border opens a view-options menu:
  Version Cue, Document Sizes, Document Profile, Document Dimensions,
  Measurement Scale, Scratch Sizes, Efficiency, Timing, Current Tool, 32-bit
  Exposure (HDR only).
- Clicking the file-information area shows width, height, channels, resolution;
  Control-click (Windows) / Command-click (Mac) shows tile width and height.
- If a Digimarc watermark is detected, a copyright symbol is shown in the
  window title bar.

### Dark CS6 interface

- CS6 introduced a darker UI than CS5 and, for the first time, **four interface
  color themes** ("Select from four different brightness levels"). CS6 shows them
  as four gray swatches at the top of the Interface preferences; they are commonly
  named **Black, Dark Gray, Medium Gray, Light Gray**, and the **default is the
  second swatch (Dark Gray)** *(secondary)*. The swatches carry no printed names
  and exact colour values are not published `(to verify)`.
- Quick brightness shortcuts: Adobe's CS6 "What's New" says **Shift+F1 decreases**
  (darker) and **Shift+F2 increases** (lighter) brightness (Mac laptop users also
  press `Fn`). The generic workspace chapter separately prints "Shift + 1 /
  Shift + 2"; this is a documentation error in the shared chapter, and one Mac
  blog reverses the direction. The Photoshop binding is **Shift+F1 = darker,
  Shift+F2 = lighter** *(primary What's New; corroborated by multiple secondary
  sources)*.
- The theme drives the initial **canvas/pasteboard colour**, and the default dark
  theme's canvas is almost black; the canvas colour can then be overridden **per
  screen mode** in the Interface preferences *(secondary)*.
- Other Interface-preference knobs relevant to the frame: `UI Font Size` (Tiny /
  Small / Medium / Large; **default Small**; restart required), `Show Menu Colors`
  (default on), `Show Tool Tips` (default on), `Enable Text Drop Shadows` (CS6
  only), `Auto-Collapse Iconic Panels`, `Auto-Show Hidden Panels`, `Restore
  Default Workspaces`, and `Show Transformation Values` (default Top Right
  *(secondary)*).
- With `Auto-Show Hidden Panels` on, moving the pointer to the edge of the
  application window (Windows) or monitor (Mac OS) reveals a strip containing the
  hidden panels *(primary: CS6 Help, "Hide or show all panels")*.
- On-image displays (HUDs for selection dimensions, transform angles) and their
  placement via `Show Transformation Values` are part of the CS6 look.

### Screen modes and full-screen

CS6 has three screen modes, cycled with `F` (forward) and `Shift+F` (backward)
*(primary: CS6 Help key table)*:

| Mode | Chrome | Background |
|---|---|---|
| Standard Screen Mode | menu bar, Options bar, document tab, scroll bars, status bar | the pasteboard/canvas colour |
| Full Screen Mode With Menu Bar | menu bar, Options bar, Tools panel and panels; document tab, scroll bars, status bar and window buttons hidden; only the active document is shown | per-mode canvas colour, default ~50% gray |
| Full Screen Mode | no menu bar, title bar, tab or scroll bars | per-mode canvas colour, default black |

- Also reachable via `View > Screen Mode > …` and the **Screen Mode button**,
  which in CS6 sits at the **bottom of the toolbar**. The CS6 Help's "Change the
  screen mode" topic still says "in the Application bar" (leftover CS5 text); the
  "What's New" wording names the toolbar button for CS6.
- In the two full-screen modes the **other open documents are hidden** but remain
  reachable from the `Window` menu, and the image can be scrolled beyond the
  document bounds *(secondary)*.
- `Esc` (or `F`) returns from Full Screen Mode to Standard Screen Mode
  *(secondary)*.
- `Tab` hides/shows the Tools panel, Options bar and panels; `Shift+Tab` hides/
  shows panels only *(primary Help; secondary for CS6)*. In Standard and Full
  Screen With Menu Bar the toggle is the ordinary persisted panel state. In Full
  Screen Mode the panel columns and the menu bar start hidden and the **first
  `Tab` reveals them** (the second hides them again); the reveal is transient,
  so returning to Standard Screen Mode restores the chrome that was in effect
  before the mode change.
- Entering either full-screen mode from Standard Screen Mode shows the CS6
  **advisory message** *(issue #328 screenshot)*: a `Message` box whose body
  reads "In Full Screen Mode, panels are hidden. They can be accessed on the
  sides of the screen, or revealed by pressing Tab." and "While in Full Screen
  Mode, you can return to Standard Screen Mode by pressing 'F' or Esc.", with a
  default **Full Screen** button, a **Cancel** button that aborts the switch,
  and a **Don't show again** checkbox whose choice persists across restarts.
  Switching between the two full-screen modes does not show it.
- Canvas/pasteboard colour cycles with `Space+F` (forward) and `Space+Shift+F`
  (backward) or by right-clicking the canvas background *(primary: CS6 Help key
  table)*. The palette is Black, Dark Gray, Medium Gray, Light Gray and a Custom
  colour (default light blue) *(secondary)*. The Interface preferences
  additionally let each of the three screen modes use a different canvas colour
  *(secondary)*.

### Floating vs. docked windows

- Panels may be **docked** (in a dock, generally vertical) or **floating**
  (free-floating, optionally stacked). A document window may be tabbed, floated,
  or placed into a group. Full details live in `UI-003`.
- On Windows the whole app is one top-level frame; floated panels are child
  top-level windows. Taskbar behavior on Linux is compositor-defined; CS6's
  Windows taskbar behavior for floated documents is not documented and is left
  to `## Open questions`.

### Modal dialogs

- An application dialog (a filter's parameter dialog, New Document, File Info,
  Preferences, Layer…) opens **without a modal window hint**, so the window
  compositor does not dim or fade the parent window (KWin's "Dialog Parent"
  effect). The shell blocks input to the parent top-level window with an event
  filter and runs the dialog's own event loop; the caller receives the same
  result a modal `exec()` would return. A dialog with no parent runs with a
  plain `exec()`, and a file dialog that hands off to a platform/portal chooser
  keeps the platform's own modality.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Application frame | Main window | n/a | Single integrated top-level window |
| Menu bar | Menu bar | `Alt`/`F10` *(platform)* | Windows-style; Mac uses app menu |
| Application menu | Menu (Mac) | n/a | Photoshop > Preferences/About/Quit; Linux mapping TBD |
| Document tab strip | Tab bar | `Ctrl+Tab`, `Shift+Ctrl+Tab` | One tab per open document |
| Document window | MDI child / top-level | n/a | Tabbed, docked, or floated |
| Canvas | Central widget | see `TOOL-042` | GPU-composited |
| Title bar | Window chrome | n/a | Shows doc name + active layer name |
| Status bar | Window bottom bar | click triangle | Magnification, file size, view options |
| Screen Mode button | Toolbar bottom button | `F` / `Shift+F` | Cycles screen modes; `Esc` returns to Standard |
| Hide/show panels | Keyboard | `Tab` / `Shift+Tab` | Panels + tools; hover edge reveals hidden panels when Auto-Show is on |
| Brightness control | Preference | `Shift+F1` (darker) / `Shift+F2` (lighter) | Four Color Theme levels; Mac laptops add `Fn` |
| Canvas colour | Preference + canvas | `Space+F` / `Space+Shift+F` | Right-click canvas for the palette; per-screen-mode colour in Interface prefs |
| Place/Edit guides | Canvas overlay | `Ctrl+R`, drag rulers | `View > Rulers`, `View > New Guide` |
| Window > Arrange | Menu | n/a | Tile/Cascade/Float/Match/New Window |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Color Theme / brightness level | enum | Dark Gray (2nd of 4 swatches) | Black / Dark Gray / Medium Gray / Light Gray | Swatches unlabeled in UI; names *(secondary)*; exact values `(to verify)` |
| UI font size | enum | Small | Tiny / Small / Medium / Large | Restart required |
| Enable Text Drop Shadows | bool | Off *(per guide screenshots; default unverified)* | on/off | Interface preferences; CS6 only |
| Title bar height | px (DIP) | reduced in CS6 | `(to verify)` | >30% shorter than CS5 |
| Document tab height | px (DIP) | `(to verify)` | — | Theme token |
| Panel width | px (DIP) | theme default | 200–480 *(secondary)* | Per panel, persisted |
| Screen mode | enum | Standard | Standard / Full Screen With Menu Bar / Full Screen | Cycles with `F`; `Esc` back to Standard |
| Canvas colour | enum/picker | theme-linked; ~50% gray / black in full-screen | Black / Dark Gray / Medium Gray / Light Gray / Custom | Per screen mode in Interface prefs; also cycles with `Space+F` |
| Floating window geometry | screen rect | — | clamped to available geometry | Persist per workspace |
| Status bar view | enum | Document Sizes? `(inferred)` | 10 options | Per-document/session |
| Show Transformation Values | enum | Top Right *(secondary)* | Never / Top Left / Top Right / Bottom Left / Bottom Right | Interface preferences |
| Auto-Show Hidden Panels | bool | Off *(secondary)* | on/off | Interface preferences |
| Auto-Collapse Iconic Panels | bool | Off *(secondary)* | on/off | Interface preferences |

## Algorithms & pipeline

Frame composition and window management. All names here are design proposals.

- **Frame = one `QMainWindow`.** Menu bar, options bar (`QToolBar`), status bar
  (`QStatusBar`), central canvas stack, and four dock areas.
- **Document area = a tabbed MDI stack, not an OS-MDI.** Each document is a
  `CanvasWidget` hosted in a tab (`QTabWidget`) or, when floated, a top-level
  window. This mirrors the CS6 tab/float model and avoids platform MDI quirks.
- **Screen-mode state machine.** `ScreenMode ∈ {Standard, FullWithMenuBar,
  Full}`. Transition hides/shows menu bar, title bar, status bar, scroll bars
  and sets the canvas backdrop; `F` is forward, `Shift+F` backward. Canvas colour
  is an independent cycle (`Space+F`).
- **Application frame on Linux.** There is no application menu; use a menu bar.
  Wayland compositors own window placement, so "float in window" becomes "make
  a new top-level document window"; absolute positioning is not assumed.
- **Status bar refresh.** Magnification, document size, scratch size, efficiency
  and timing are sampled from the renderer and allocator; updates are throttled
  to avoid per-frame churn *(inferred)*.
- **Theme.** One `Theme` object is the single source of truth for the four
  brightness levels, metrics and colours (see `ARCH-003`). No widget hard-codes a
  colour.

## Rust module mapping

Proposals; the shell is largely Qt/C++ with Rust supplying state.

- `pictura_ui::shell` — frame setup, menu/dock registration, screen-mode state,
  workspace save/restore.
- `pictura_ui::frame::ScreenMode` — `enum ScreenMode`, transition table, and
  `apply(mode)` intent emitted to Qt.
- `pictura_ui::frame::window_store` — per-document/per-window view state
  (`ViewportState` from `TOOL-042`), tab order, float state.
- `pictura_ui::theme` — brightness level → token table (colours, metrics).
- `pictura_ui::status` — computed status fields (`DocumentSize`, `ScratchSize`,
  `Efficiency`, `Timing`, `CurrentTool`, `HdrExposure`).

Crossing types: `ScreenMode`, `BrightnessLevel`, `WindowId`, small value
structs only. No image buffers cross this boundary.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PicturaMainWindow` | `QMainWindow` | Frame, menu bar, options `QToolBar`, `QStatusBar`, dock areas, `saveState()`/`restoreState()` |
| `DocumentTabBar` | `QTabBar`/`QTabWidget` | Tab strip, reorder, drag-out-to-float, context menu (New/Open on Windows parity) |
| `CanvasWidget` | `QRhiWidget` (proposed) | GPU document surface, hosted in tab or float window |
| `StatusBar` | `QStatusBar` | Magnification/size/efficiency fields + view-options popup |
| `ScreenModeController` | `QObject` | Applies `ScreenMode`; binds `F`/`Shift+F`/`Space+F` |
| `BrightnessController` | `QObject` | Applies the four theme levels; binds brightness shortcuts |
| `Theme` | `QObject` (tokens) | Single source of truth for colours/metrics |

Widgets over QML for the shell, per `ARCH-003`. The frame is dense, keyboard-
centric and dock-heavy; QML's costs are not justified here.

## Data-model impact

- Frame state is **UI/session state, not document content**: window geometry,
  tab order, float state, screen mode, canvas colour, brightness, status-bar
  view. Persisted in the workspace/session store (`UI-003`,
  `11-cross-cutting/preference-storage.md`), never in PSD/XMP.
- Whether CS6 writes per-document view metadata (zoom/scroll/screen mode) into
  the PSD is *unverified* (`TOOL-042` open question).
- No undo records are produced by frame operations (resize, tab move, screen
  mode). `Ctrl+Z` must not revert them *(inferred)*.
- Status-bar values are derived from the document/renderer/allocator and are
  never serialized.

## Edge cases

- **Wayland.** No absolute window positioning; floating documents become
  compositor-managed top-levels and restore by relative layout only.
- **Multi-monitor.** Floated windows must be clamped to available screen
  geometry on restore; screen geometry is in DIP with scaling gaps.
- **High-DPI change at runtime.** Moving the frame between displays changes
  `devicePixelRatio`; the canvas buffer and icon assets must rebuild.
- **Full-screen on tiled compositors.** Full Screen Mode is compositor-
  controlled on Linux and may not be able to hide all chrome or may be refused.
- **GPU unavailable.** The shell and status bar must still start; canvas falls
  back to CPU compositing (`ARCH-006`).
- **Zero documents open.** Frame shows an empty canvas (or Home screen if
  adopted); menus that require a document are disabled.
- **Many documents.** Tab strip must scroll/overflow; float-all on a small
  screen must not produce unreachable windows.
- **Mac application menu semantics.** `Quit`, `About`, `Services`, `Hide` have
  no exact Linux equivalent; mapping needs a decision.
- **Taskbar behavior.** CS6's Windows grouping of floated documents is
  undocumented; Linux DE/taskbar behavior must be specified per environment.

## Parity acceptance criteria

- Given no document is open, then opening one adds a tab; opening a second adds
  a second tab; `Window > Arrange > Float All In Windows` re-parents every tab
  into its own top-level window.
- Given Standard mode, pressing `F` yields Full Screen With Menu Bar (menu bar,
  Options bar, Tools panel and panels retained; document tab, scroll bars and
  status bar hidden; ~50% gray canvas), pressing `F` again yields Full Screen
  (black, no chrome), and `Shift+F` reverses the sequence.
- Given Full Screen Mode, `Space+F` cycles the canvas background colour and
  `Space+Shift+F` reverses it.
- Given a document with a detected watermark, the copyright symbol appears in
  the title bar (`Filter ▸ Digimarc ▸ Read Watermark`; an in-tree approximation,
  see `06-filters/digimarc-watermark.md`).
- Given the status-bar triangle is clicked, the documented view-option list
  appears and selecting `Document Sizes` shows the two size figures.
- Given the Interface preferences, selecting each of the four brightness levels
  changes the whole frame chrome and canvas surround consistently, and the
  choice survives restart.
- Given `Shift+F1`/`Shift+F2`, interface brightness steps down/up one of the four
  Color Theme levels and is reflected immediately; `Esc` from Full Screen Mode
  returns to Standard, and `Tab`/`Shift+Tab` toggle the documented chrome.
- Given a workspace is saved with floated panels, quit and relaunch restores
  tab order, float state, dock layout and screen mode within one pixel at the
  same scale factor.
- Given a two-document tab group, `Ctrl+Tab` cycles documents and
  `Shift+Ctrl+Tab` cycles backward.
- Given the frame is resized, all docked panels reflow without overlap and the
  canvas takes the remaining area.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help reference (downloaded and text-extracted; also
  cached locally). Establishes: application frame definition and behavior; the
  Application bar historically contained a workspace switcher, "menus (Windows
  only)" and other controls; tabbed document windows and float-in/floats-all;
  `Window > Arrange` commands; title bar shows the active layer name and adds a
  copyright symbol on Digimarc detection; status bar location/contents and its
  full view-option list; the three screen modes and `F`/`Shift+F` cycling; the
  key table's `Space+F` / `Space+Shift+F` canvas-colour cycling; CS6 removal of
  the app bar and >30% title-bar reduction; four brightness levels and
  `Shift+F1`/`Shift+F2`; Interface preference items (UI Font Size, Show Tool
  Tips, Show Menu Colors, Auto-Show Hidden Panels, Auto-Collapse Icon Panels,
  Restore Default Workspaces, Show Transformation Values); the Auto-Show edge
  strip; Windows-only New/Open on the document-tab context menu; Enable Flick
  Panning and the zoom options; Additional Plug-ins Folder; Point/Pica; GPU
  Settings/Enable OpenGL Drawing; File Handling Save In Background and the
  10-minute Auto Save default; History Log options; Type options; 3D options.
- `https://www.photoshopforphotographers.com/pscs6/downloads/Photoshop-interface.pdf`
  — Martin Evening, *Adobe Photoshop CS6 for Photographers* (free sample
  chapter). Establishes: the Application bar is gone and its workspace options
  moved to the Options bar, with document layout solely under `Window > Arrange`;
  the Mac `Window > Application Frame` toggle; the Interface preferences
  (Figure 5) expose four themes, UI font size, **Enable Text Drop Shadows** and
  **per-screen-mode canvas colour** (Standard Screen / Full Screen with Menus /
  Full Screen); the canvas colour is linked to the theme and is almost black at
  the dark default; interface/UI-font changes need a relaunch.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Screen_display.html`
  — same book, "Screen display modes". Establishes: Standard vs Full Screen With
  Menu Bar vs Full Screen chrome and canvas behaviour; other documents hidden but
  reachable via the Window menu; `Tab`, `F`, and `Ctrl+Tab`/`Ctrl+Shift+Tab`.
- `https://www.photoshopessentials.com/basics/interface-cs6/` — secondary CS6
  tutorial. Establishes: four Color Theme swatches with the **default second from
  the left**, `Shift+F2` lighter / `Shift+F1` darker, and the pasteboard palette
  (Black, Dark Gray, Medium Gray, Light Gray, Custom — default light blue).
- `https://www.photoshopessentials.com/basics/photoshop-screen-modes-interface-tricks/`
  — secondary (CC, stated CS6-compatible). Establishes: the three screen modes'
  visible chrome, hiding of the tab/scroll bars/status bar in Full Screen With
  Menu Bar, `Esc` to exit Full Screen, edge-hover to reveal tools/panels, and
  `Tab`/`Shift+Tab`.
- Issue #328's screenshot of the CS6 full-screen advisory (a black-box capture
  attached to the repository issue) — establishes the advisory's `Message`
  title, its two body sentences, the `Full Screen` / `Cancel` buttons, and the
  `Don't show again` checkbox.
- `https://www.howtogeek.com/724883/how-to-switch-between-light-and-dark-themes-in-photoshop/`
  — secondary. Corroborates `Shift+F1` darker / `Shift+F2` lighter, four levels,
  persisted across restarts.
- `https://www.thegraphicmac.com/photoshop-cs6-interface-color-keyboard-shortcut/`
  — secondary. Confirms four shades and Shift+F1/F2 exist; its darker/lighter
  directions are reversed relative to Adobe and the other sources.
- `https://osxdaily.com/2012/04/20/change-photoshop-cs6-dark-interface-color-scheme-to-light/`
  — secondary. Names the range "darkest grey, dark grey, medium grey, light
  gray" and gives `Shift+Fn+F2`/`Shift+Fn+F1`.
- `https://www.properproof.com/photoshop/guides/Adobe%20Photoshop%20%20%20Default%20keyboard%20shortcuts.htm`
  — reproduction of the CS6 default key list. Confirms `F`/`Shift+F`, `Space+F`/
  `Space+Shift+F`, and that `Ctrl+1` is Magnify 100% (not a Preferences pane).
- SearXNG meta-search queries used to locate the above. Search-result snippets
  that could not be opened are noted as such below; no fact rests on a snippet
  alone.

Not parsed: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Exact theme palette values and metrics.** The four themes' names are sourced
  (Black, Dark Gray, Medium Gray, Light Gray; default Dark Gray), but the actual
  greys and the panel background, border, text, title-bar and tab heights are not
  published. Resolve with a CS6 screenshot colour sample and pixel metrics.
- **Mac application menu, beyond the frame.** `Window > Application Frame`
  explains the Mac frame toggle, but how Photoshop CS6 populated its Mac
  application menu (Photoshop > About/Preferences/Services/Hide/Quit) is only
  generically sourced. Resolve with the Mac-specific CS6 Help or a capture.
- **Taskbar behavior for floated documents on Windows.** Not documented. Resolve
  with a CS6-on-Windows capture; then define the Linux DE mapping.
- **Screen mode vs. window manager on Linux.** Full-screen chrome hiding under
  X11 vs Wayland tiling compositors needs a decision. Resolve in `UI-003` /
  platform integration.
- **Status-bar default view option.** Only the option list is sourced; the
  default field is inferred. Resolve with a first-run CS6 capture.
- **Exact CS5 → CS6 title-bar pixel metric.** The "over 30%" reduction is
  primary; the absolute height is not. Resolve from a CS6 screenshot at 100%.
- **`Show Transformation Values` default.** Top Right is asserted by a secondary
  source only. Resolve from a fresh CS6 Interface-pane capture.
