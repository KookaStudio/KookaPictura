# application-shell Specification

## Purpose
The Qt6 plus Rust application shell: the cxx-qt bridge, the CMake build, the headless window, the theme, and the status readouts.

## Requirements

### Requirement: CXX-Qt bridge exposes a Rust QObject to C++
The system SHALL define a Rust `QObject` through a `#[cxx_qt::bridge]` module and
SHALL expose it to the C++ shell using `cxx-qt` 0.10 and `cxx-qt-build`, rather
than hand-written `extern "C"` glue.

#### Scenario: C++ constructs the Rust bridge object
- **WHEN** the C++ shell constructs the Rust-defined `PictureView` object
- **THEN** the generated `QObject` is available to C++ and its methods are callable

#### Scenario: Rust owns the bridge state
- **WHEN** the bridged object is destroyed by C++
- **THEN** its Rust-side state is released without a double free

### Requirement: CMake builds the bridge and the C++ shell
The system SHALL build the application from a top-level `CMakeLists.txt` using
C++17, `find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)`, and
`cxx_qt_import_crate` (Corrosion) to import the `pictura_app` static library and
link it into the `pictura` executable.

#### Scenario: Configure and build with CMake
- **WHEN** a developer runs `cmake -S . -B build && cmake --build build` with Qt 6 present
- **THEN** CMake imports `pictura_app` and links the `pictura` executable

### Requirement: Qt 6 is discovered through qmake6
The system SHALL locate a Qt 6 `qmake` by preferring `qmake6` over `qmake`,
verifying that `qmake -query QT_VERSION` reports a major version of 6, and SHALL
fail configuration with an error that names the missing package when no Qt 6
`qmake` is found.

#### Scenario: Qt 6 qmake is found
- **WHEN** `qmake6` reports a version starting with `6`
- **THEN** CMake uses it as the Qt discovery executable

#### Scenario: No Qt 6 qmake is available
- **WHEN** neither `qmake6` nor a Qt 6 `qmake` exists on `PATH`
- **THEN** CMake configuration fails with an error naming the Qt 6 development package

### Requirement: Headless window shell
The system SHALL open a top-level Qt Widgets `QMainWindow` frame containing a
menu bar, a tabbed document area that hosts one canvas per open document, a
status bar, and dock areas, and SHALL run under a headless display server (Xvfb,
the offscreen QPA plugin, or the explicit `--headless` flag) without requiring a
GPU. A normal launch with no document argument SHALL open no document and leave
the document-requiring commands disabled; the scratch document used by the
self-test checks SHALL be created only for a `--self-test` or `--headless` run
that needs it. An explicitly opened or created document SHALL behave as before.

#### Scenario: Run under Xvfb
- **WHEN** the executable is started under `xvfb-run` or with `QT_QPA_PLATFORM=offscreen`
- **THEN** it opens its window and exits without error

#### Scenario: Run with the headless flag
- **WHEN** the executable is started with `--headless`
- **THEN** it opens its window on the offscreen platform and exits without error

#### Scenario: Frame chrome is present at startup
- **WHEN** the window is shown
- **THEN** the menu bar, tabbed document area, status bar, and dock area exist

#### Scenario: No document open
- **WHEN** the frame starts with no document loaded
- **THEN** document-requiring commands are disabled and the application does not crash

#### Scenario: A normal launch opens no document [las_no_scratch]
- **WHEN** the application is launched with no document argument and without
  `--self-test` or `--headless`
- **THEN** no tab and no document are created, and the document-requiring
  commands are disabled

#### Scenario: The self-test keeps its scratch document [las_selftest_scratch]
- **WHEN** the application is launched with `--self-test` or `--headless` and no
  document argument
- **THEN** the scratch document the bridge checks depend on is created and the
  self-test runs normally

### Requirement: Bridge self-test mode

The system SHALL provide a `--self-test` mode that exercises the Rust↔Qt bridge,
writes a summary to stderr, and exits non-zero when the bridged image is missing.
The self-test SHALL expose each later milestone as an independently addressable
check with a stable exit code, so a failure names the check. The headless
self-test SHALL assert that the application platform is `offscreen` and SHALL
reserve exit code **152** for a platform mismatch; later checks SHALL allocate
codes from **153** upward. The M44 checks SHALL occupy codes **153–166** and the
M45 checks SHALL occupy codes **167–179**. New checks SHALL take the next free
codes from **299** upward and SHALL live in a `selftest_*.cpp` translation unit
rather than growing `selftest.cpp`.

#### Scenario: Self-test succeeds

- **WHEN** the executable is started with `--self-test` and the bridge returns an image
- **THEN** it prints a self-test summary and exits 0

#### Scenario: Self-test fails on a null image

- **WHEN** the bridge returns no image
- **THEN** the self-test prints a failure and exits with a non-zero status

#### Scenario: The headless platform is asserted

- **WHEN** `--headless --self-test` runs while the application platform is not `offscreen`
- **THEN** the self-test prints a failure and exits **152**

#### Scenario: Milestone checks are independently addressable

- **WHEN** an M45 check fails
- **THEN** the self-test exits with that check's code in the range **167–179** and prints which check failed

#### Scenario: New checks use append-only codes [las_codes]

- **WHEN** a new UI check is added for this change
- **THEN** it takes a code at or above **299**, lives in a new `selftest_*.cpp`
  file, and the existing exit codes are unchanged

### Requirement: Interface theme with brightness levels
The system SHALL apply a dark application theme through a single `Theme` unit as
the source of truth, and SHALL offer four interface brightness levels applied at
runtime. Brightness SHALL step down and up with `Shift+F1` and `Shift+F2`, and
the selected level SHALL persist across restart.

#### Scenario: Theme applies at startup
- **WHEN** the frame is shown
- **THEN** the frame chrome uses the dark theme rather than the platform default

#### Scenario: Brightness shortcut steps the level
- **WHEN** `Shift+F2` is pressed
- **THEN** the interface brightness increases by one level and is reflected immediately

#### Scenario: Brightness persists
- **WHEN** the brightness level is changed and the application restarts
- **THEN** the selected level is restored

### Requirement: Screen modes and canvas colour

The system SHALL support three screen modes -- Standard, Full Screen With Menu
Bar, and Full Screen. The Tools panel's screen-mode button SHALL open a menu
listing the three modes with the active mode checked, and choosing an entry SHALL
switch to that mode; the button's icon SHALL reflect the active mode. `F` SHALL
cycle the modes forward and `Shift+F` backward. `Space+F` SHALL cycle the canvas
background colour.

#### Scenario: Cycle screen modes

- **WHEN** `F` is pressed from Standard mode
- **THEN** the frame enters Full Screen With Menu Bar, and pressing `F` again enters Full Screen

#### Scenario: Reverse the cycle

- **WHEN** `Shift+F` is pressed
- **THEN** the screen mode steps backward through the cycle

#### Scenario: Cycle canvas colour

- **WHEN** `Space+F` is pressed
- **THEN** the canvas background colour advances to the next value

#### Scenario: The screen-mode button opens the mode menu [las_screen_mode_menu]

- **WHEN** the Tools panel's screen-mode button is activated
- **THEN** a menu lists Standard, Full Screen With Menu Bar, and Full Screen with the active mode checked, and choosing an entry switches the frame to that mode

### Requirement: Status bar readouts

The system SHALL show the active document's magnification and file size in a
status bar, along with a tool-hint field, and SHALL expose a view-options popup
from the status bar for selecting which readout is displayed. When no document
is open the status bar SHALL show no readouts and no tool hint — only its
background — and SHALL show them again once a document opens. The status bar
SHALL use a small left padding and SHALL NOT draw the right-side size-grip
corner triangle. The status-bar magnification readout and any zoom control SHALL
show the same value in the same format. While a Free Transform, Skew, Distort,
or Perspective session is active the status bar SHALL also show an action
readout naming it (`Free Transform in progress`, `Skew in progress`, `Distort in
progress`, or `Perspective in progress`); the readout SHALL clear when the
session commits or cancels.

#### Scenario: Magnification and size shown

- **WHEN** a document is displayed
- **THEN** the status bar shows the current magnification and document size

#### Scenario: View options popup

- **WHEN** the status-bar options control is activated
- **THEN** a popup lists the selectable readouts and the chosen readout is displayed

#### Scenario: No document shows an empty status bar [las_status_empty]

- **WHEN** no document is open
- **THEN** the status bar shows no magnification, size, or tool-hint text, only its background, and restores them when a document opens

#### Scenario: No corner size grip [las_status_no_grip]

- **WHEN** the status bar is shown
- **THEN** it draws no right-side size-grip corner triangle and uses a small left padding

#### Scenario: A transform session names its action [las_status_action]

- **WHEN** `Edit > Transform > Skew` begins a session
- **THEN** the status bar shows `Skew in progress` until the session commits or cancels

### Requirement: CS6-style chrome styling

The system SHALL style the application chrome (menu bar, options bar, Tools
panel, panel docks and their tabs and title bars, status bar, tool buttons, and
scrollbars) with a CS6-style dark stylesheet driven by the same brightness ramp
as the palette, and SHALL rebuild the stylesheet when the brightness level
changes. The bar chrome SHALL be a low-contrast dark surface distinct from the
document canvas, and panel tabs SHALL be flat with a distinctly highlighted
active tab. The Tools toolbar and the normal and compact widget panels SHALL
draw a darker grey border from the shared theme, and a widget SHALL use the same
colour scheme and style whether docked, shown in a compact flyout, or floating.
The document tab strip SHALL draw a dark grey border on its right side and SHALL
NOT draw an extra top border, because the options bar above it already draws a
dark grey bottom border. Every new rule SHALL be scoped to the panel family and
the document tab strip so the rest of the chrome is unaffected.

#### Scenario: Chrome is styled at startup

- **WHEN** the frame is shown
- **THEN** the dock tabs, dock title bars, tool buttons, menu bar, and status bar use the dark stylesheet rather than unstyled Fusion defaults

#### Scenario: Brightness restyles the chrome

- **WHEN** the brightness level changes with `Shift+F1` or `Shift+F2`
- **THEN** the stylesheet is regenerated for the new level and the chrome colours change

#### Scenario: Panels draw the darker grey border [m44_panelborder]

- **WHEN** the Tools toolbar and a normal- or compact-mode widget panel are shown
- **THEN** each draws a darker grey border from the shared theme

#### Scenario: The document tab strip has only a right border [m44_filebar]

- **WHEN** the document tab strip is inspected
- **THEN** it draws a dark grey border on its right side and no extra top border

#### Scenario: A widget looks the same in all three presentations [m44_popupstyle]

- **WHEN** the same widget is docked, shown in a compact flyout, and floating
- **THEN** all three present the same background, borders, and tab-bar styling

### Requirement: Default dock grouping and canvas colour

The system SHALL present the default workspace with the right-hand panels
hosted by the `PanelColumn` in the authentic CS6 Essentials two-column layout:
a wider main right-hand column with the groups **Color, Swatches**;
**Adjustments, Styles**; and **Layers, Channels, Paths**, and a narrower
secondary right-hand column collapsed to icons holding **History** and
**Properties**. The central area SHALL be a horizontal splitter hosting an
ordered set of `PanelColumn`s around the document tab area: zero or more columns
to the left of the document tabs and zero or more to the right, with the document
tabs keeping the stretch. A `PanelColumn` SHALL be creatable dynamically by a
drop and removed when emptied. The document canvas SHALL use the CS6 dark canvas
colour, and the document tab strip SHALL be styled to match the chrome. Panel
`objectName`s SHALL remain stable so the persisted session layout keeps working.
The Tools panel SHALL remain a left/right dock separate from the columns.

#### Scenario: Panels form the CS6 Essentials groups

- **WHEN** the frame starts with a fresh session
- **THEN** Color and Swatches share a group, Adjustments and Styles share a
  group, Layers, Channels, and Paths share a group, and a secondary iconic
  column holds History and Properties

#### Scenario: Existing panel behaviour is unchanged

- **WHEN** a panel is shown or hidden from the `Window > Panels` menu
- **THEN** its visibility toggles as before, including the grouped panels

#### Scenario: Canvas matches the chrome

- **WHEN** the frame is shown
- **THEN** the document view background is the CS6 dark canvas colour

#### Scenario: The document tabs keep the stretch [m43_newcolumn]

- **WHEN** a new panel column is created on either side
- **THEN** the document tab area keeps the stretch and the new column takes only
  its own width

### Requirement: Canvas transparency checkerboard

The document canvas SHALL draw a two-tone checkerboard behind the document image,
clipped to the document rect, so that any pixel with alpha less than 255 reveals
it. A fully transparent document SHALL show the checkerboard, and the
checkerboard SHALL NOT be drawn outside the document rect. The canvas SHALL NOT
substitute an opaque backdrop for a document whose bottom layer is a Background:
a visible Background covers the checkerboard through its opaque pixels, and
hiding the Background SHALL reveal the checkerboard.

#### Scenario: A transparent document reveals both checker tones

- **WHEN** a fully transparent document is displayed on the canvas
- **THEN** pixels inside the document rect show the two checkerboard tones and
  pixels outside the document rect show the canvas colour

#### Scenario: Opaque content covers the checkerboard

- **WHEN** a document region is fully opaque
- **THEN** that region shows the document pixels and the checkerboard is not
  visible through it

#### Scenario: Hiding the Background reveals the checkerboard [app_shell_hidden_background]

- **WHEN** a document with a visible Background layer has that Background's visibility toggled off
- **THEN** the document rect shows the transparency checkerboard instead of an opaque canvas backdrop

### Requirement: Canvas clips content to the document bounds

The canvas SHALL clip the composited document and the live move preview to the
document rect, and content outside the canvas SHALL NOT be drawn.

#### Scenario: A layer preview dragged outside the document is cropped

- **WHEN** the move preview draws a layer whose position puts part of it outside
  the document rect
- **THEN** the part inside the document rect shows the layer and the part outside
  shows the canvas colour

### Requirement: Checkerboard is screen-space and document-anchored

The checkerboard cell size SHALL be constant in screen space, independent of
zoom, and the checkerboard SHALL be anchored to the document origin so that
panning does not move the pattern relative to the document.

#### Scenario: Zoom does not change the cell size

- **WHEN** the canvas zoom changes
- **THEN** the checkerboard cell size in screen pixels stays the same

#### Scenario: Panning does not shift the pattern

- **WHEN** the canvas is panned
- **THEN** the checkerboard stays aligned to the document origin rather than to
  the viewport

### Requirement: Toolbox catalogue and flyout groups

The system SHALL define a frozen toolbox catalogue of the CS6 tools in
single-column flyout slots, and SHALL show every catalogue tool, implemented or
not, in its slot's flyout. The catalogue MAY retain the Object (3D) and Camera
tool entries — the enum and table stay stable — but the Tools panel SHALL NOT
present their slots, so the panel shows 21 slots. Each catalogue entry
SHALL carry a stable asset id, a display label, a shortcut (or none), its flyout
group and slot, whether it is implemented, a `Qt::CursorShape` fallback, and a
cursor hotspot. The toolbox SHALL present one button per slot, SHALL mark a slot
with more than one member with a corner triangle, and SHALL reveal the slot's
members on hold (with `Alt`-click or the slot shortcut cycling the enabled
members). The slot's visible tool SHALL be its last-used member. A tool that is
not implemented SHALL be shown disabled with the tooltip `<label> — not
implemented yet`. The 10 implemented tools SHALL keep their existing shortcut,
checked state, cursor, and canvas behaviour.

#### Scenario: The catalogue is complete

- **WHEN** the toolbox catalogue is enumerated
- **THEN** it retains every CS6 tool including Object (3D) and Camera, while the
  Tools panel presents 21 slots with exactly 10 marked implemented

#### Scenario: The 3D and Camera slots are absent [las_no_3d_camera]

- **WHEN** the toolbox's presented slots are inspected
- **THEN** no Object (3D) or Camera slot or button is present, although the
  catalogue retains their entries

#### Scenario: A slot with hidden tools reveals them

- **WHEN** the user holds the mouse on a slot whose group has more than one member
- **THEN** a flyout lists every member of that group

#### Scenario: An unimplemented tool is visible but disabled

- **WHEN** the flyout of a group containing a tool with no engine is opened
- **THEN** that tool's entry is disabled and its tooltip is `<label> — not implemented yet`

#### Scenario: An implemented tool is selected from a flyout

- **WHEN** the user picks an implemented tool from a flyout
- **THEN** that tool becomes active and the slot shows its icon as the last-used member

#### Scenario: Cycling a slot's enabled members

- **WHEN** the user `Alt`-clicks a slot, or presses `Shift` plus the slot's shortcut
- **THEN** the active tool advances to the next enabled member of that group, wrapping deterministically

#### Scenario: Implemented tools behave as before

- **WHEN** an implemented tool is activated by shortcut or by clicking its button
- **THEN** it is marked active, its cursor is applied, and its options bar is shown, exactly as before this change

### Requirement: Tools panel is a standalone dock

The Tools panel SHALL be hosted as an atomic column — a tabless, ungroupable
`PanelColumn` variant — and SHALL NOT be a `QDockWidget` in the main-window dock
areas. It SHALL support being moved and floated as a frameless `Qt::Tool`
top-level window parented to (transient for) the main window, and SHALL NOT be
grouped with other panels in a tab group. A drag or drop that would combine the
Tools panel with a widget panel or group SHALL NOT tabify or merge them. When
floated it SHALL size to the minimum its content needs, SHALL offer no resize
grip, and SHALL be a frameless `Qt::Tool` top-level window — no title bar, no
window decorations, and no taskbar entry — that may be moved outside the main
window, its movement clamped to the available screen geometry, rather than an
operating-system window with a frame. The panel's column header SHALL remain
draggable so the column can be moved and floated. The panel's content size SHALL
be fixed along the column's major axis: a fixed content width in both one- and
two-column modes. Dragging the column separator SHALL NOT resize it: the
separator handle beside the docked Tools column SHALL be disabled so no resize
drag is offered, the docked column's width range SHALL be pinned to its content
width (minimum equal to maximum) so no splitter layout pass or stray resize can
change it either, and the disabled handle state SHALL be re-applied whenever the
column is re-inserted into the splitter. Changing the tool grid between one and
two columns SHALL re-fit a floating Tools overlay to the new content size on both
axes, so the two-column grid is not clipped by a one-column-wide overlay. The
panel SHALL be placeable on either side of any widget panel or column, wherever
the columns are placed, through the same column drop grammar and the same single
insertion indicator the widget columns use, without breaking the fixed-size rule
or the no-tabification contract.

#### Scenario: The panel is an atomic column, not a dock [las_tools_column]

- **WHEN** the frame is built
- **THEN** the Tools panel is a tabless `PanelColumn` variant with no tab bar and
  no `PanelGroup`, and no Tools `QDockWidget` exists

#### Scenario: The panel floats as a frameless tool window [las_tools_inwindow_float]

- **WHEN** the Tools column is dragged out of the workspace
- **THEN** it floats as a frameless `Qt::Tool` top-level window parented to the
  main window, movable outside the main window and clamped to the screen, and can
  be re-placed

#### Scenario: The floating overlay hugs its content height [m42_tools]

- **WHEN** the Tools column floats in-window
- **THEN** its height is the minimum its content needs and it does not expand to
  fill the window

#### Scenario: The floating height cannot be dragged [m44_toolsfloat]

- **WHEN** a resize is attempted on the floating Tools column
- **THEN** it keeps its fixed content height and does not change

#### Scenario: Tabification is refused [m40_dock]

- **WHEN** the Tools column is dragged onto another panel's tab bar
- **THEN** it does not become a tab in that group and no insertion indicator is
  shown for the forbidden combination

#### Scenario: The width cannot be dragged [m43_tools]

- **WHEN** the column separator beside the Tools column is dragged
- **THEN** the Tools column's width does not change and stays at its content
  width

#### Scenario: The separator handle beside Tools is disabled [tpc_docked_separator_disabled]

- **WHEN** the frame is built or a panel session is applied
- **THEN** the splitter handle adjacent to the docked Tools column is disabled,
  so no separator drag can resize the toolbar

#### Scenario: The docked Tools width range is pinned [tpc_docked_width_pinned]

- **WHEN** the docked Tools column has been laid out
- **THEN** its minimum width equals its maximum width and equals the content
  width, so no splitter layout pass or stray resize can change it

#### Scenario: A floating Tools overlay re-fits on a mode flip [tpc_float_refit_on_mode_flip]

- **WHEN** the floating Tools column's header toggle changes the tool grid from
  one to two columns
- **THEN** the overlay grows to the two-column content width and shrinks in
  height to the new content minimum, so the grid is not clipped

#### Scenario: The panel is placed beside a widget column [m45_tools_beside_column]

- **WHEN** the floating Tools column is dragged to a side of a widget column,
  including the left of a right-hand column or between two columns
- **THEN** the single blue indicator marks that boundary and the panel is placed
  there without being tabified and without losing its fixed content width

### Requirement: Menu bar is not overlaid

The application shell SHALL keep the menu-bar row clear of every other widget: no
toolbar, dock title bar, floating overlay, or stray child widget SHALL be drawn
over the menu bar. A persisted layout that no longer matches the current chrome
SHALL be discarded rather than restoring a widget over the menu bar.

#### Scenario: Nothing overlays the menu bar [m42_menubar]

- **WHEN** the frame is shown in its default and restored arrangements
- **THEN** no child widget's global geometry intersects the menu-bar row and only
  the menu bar is drawn there

#### Scenario: A stale layout is discarded [m42_menubar]

- **WHEN** a persisted layout that references chrome no longer present is
  restored
- **THEN** it is discarded in favour of the default arrangement and no widget is
  drawn over the menu bar

### Requirement: Explicit headless mode
The system SHALL provide a `--headless` command-line flag that selects the Qt
offscreen QPA plugin before constructing `QApplication`, so the executable runs
without an X or Wayland display. The flag SHALL take precedence over the
`--self-test` xcb override and SHALL require no external display server. When
`--headless` is given with neither a document argument nor `--self-test`, the
system SHALL run the bridge self-test. `--headless` SHALL NOT force the offscreen
plugin onto `--interop-probe`, which requires a platform Vulkan instance.

#### Scenario: Headless selects the offscreen platform
- **WHEN** the executable is started with `--headless` and `QT_QPA_PLATFORM` is unset
- **THEN** the offscreen QPA plugin is selected before `QApplication` is constructed and the process does not require `DISPLAY` or `WAYLAND_DISPLAY`

#### Scenario: Headless self-test asserts the offscreen platform
- **WHEN** `--headless --self-test` runs
- **THEN** the self-test verifies that the application platform is `offscreen`, prints the self-test summary, and exits 0

#### Scenario: A bare headless run implies the self-test
- **WHEN** `--headless` is given with no document argument and without `--self-test`
- **THEN** the system runs the bridge self-test instead of blocking in the event loop

#### Scenario: An explicitly set platform is respected
- **WHEN** `--headless` is given and `QT_QPA_PLATFORM` is already set in the environment
- **THEN** the environment value is used unchanged

#### Scenario: Wrong platform fails the headless self-test
- **WHEN** the headless self-test runs while the application platform is not `offscreen`
- **THEN** the self-test prints a failure and exits non-zero

### Requirement: Central splitter keeps every pane

The application shell SHALL keep every pane of its central splitter — the
ordered set of left widget columns, the document tab area, and right widget
columns — at or above its minimum size, and SHALL NOT allow a handle drag to
collapse a pane to zero. The splitter SHALL clamp a handle drag at the pane's
minimum size, and a widget column SHALL remain visible at its minimum floor
instead of disappearing. The per-column group splitter SHALL enforce the same
non-collapsible invariant for its panel groups.

#### Scenario: Dragging the central splitter handle cannot hide a pane [m46_center_splitter]

- **WHEN** the central splitter handle is dragged past a widget column's minimum
  width
- **THEN** the column is clamped at its minimum and stays visible rather than
  collapsing to zero

#### Scenario: The group splitter cannot collapse a group to zero [m46_no_collapse]

- **WHEN** a column's group splitter handle is dragged past a panel group's
  minimum height
- **THEN** the group is clamped at its minimum and its tab bar stays visible

### Requirement: Compact group chrome shading

The compact/iconic group container SHALL use the panel surface shade rather than
the darker base shade, and its drag-handle dots SHALL be dark gray, distinct
from the near-white window text, so the dots read as a handle.

#### Scenario: Compact group background and dots [m47_compact_shade]

- **WHEN** the compact strip is built
- **THEN** each group container uses the panel surface shade and its drag dots
  render in dark gray

### Requirement: Empty document area keeps its workspace space

When the frame is shown with no document open the document pane SHALL stay in
the central splitter with a minimum width and the splitter stretch, so the
widget columns can never absorb the workspace and the splitter keeps a grabbable
handle between the workspace and each adjacent column. Only the empty tab strip
SHALL be hidden; the pane itself SHALL remain visible and SHALL become the
workspace background. The visibility SHALL be driven from the shared
`frame.cpp::refresh` funnel so every path that changes the document set (create,
open, import, close, close all, restore) takes the same branch. When a document
exists the strip SHALL be shown and the pane SHALL host the canvas as before.

#### Scenario: No document keeps the workspace space [las_empty_pane]

- **WHEN** the frame starts with no document open
- **THEN** the document pane is visible with at least its minimum width, the
  widget columns together take less than the splitter width, an empty tab strip
  is not drawn, and the splitter exposes a handle between the workspace and each
  adjacent column

#### Scenario: The first document reveals the tab strip [las_empty_pane_show]

- **WHEN** a document is created or opened with no document previously open
- **THEN** the document tab strip is shown in the pane and hosts the new canvas

#### Scenario: Closing the last document hides the strip again [las_empty_pane_hide]

- **WHEN** the last open document is closed
- **THEN** the tab strip is hidden again through the same refresh path while the
  pane keeps its workspace space

#### Scenario: An icon column keeps its strip against the workspace [las_icon_workspace]

- **WHEN** a widget column is in icon mode beside the workspace
- **THEN** the column keeps its fixed strip width and the workspace, not the
  column, absorbs the splitter slack

### Requirement: Docked or pane-hosted Tools panel content is top-aligned

The content body of the Tools panel SHALL fill the height the dock is given and
SHALL start at the top while the panel is docked in a left or right dock area or
hosted as a central-splitter pane, so the slot column begins directly below the
custom title bar. The body SHALL expand vertically rather than taking its fixed
content height and being centred in the taller dock. Only a floating panel SHALL
keep the body at its fixed content height.

#### Scenario: The content fills and starts at the top [las_tools_top_align]

- **WHEN** the Tools panel is hosted as a central-splitter pane or docked and the
  frame is taller than the panel's content
- **THEN** the content body's height equals the space below the title bar and its
  top edge sits at the title bar's bottom, not centred in the dock

### Requirement: Tools column fills its column height

The Tools column SHALL fill its column height while it is hosted in the
workspace, and its content minimum SHALL NOT force the main window's central
workspace shorter than the window. Only the in-window floating overlay SHALL pin
its height to the content it needs; re-placing a floating column into the
workspace SHALL release the pinned height and floating it again SHALL re-pin it.
The floating pinned height SHALL NOT be drag-resizable.

#### Scenario: The column fills its column height [las_tools_free_height]

- **WHEN** the Tools column is hosted in the workspace
- **THEN** the column fills the height it is given, the central splitter keeps
  its height, and the document pane fills the splitter

#### Scenario: Floating pins and re-placing releases [las_tools_float_height]

- **WHEN** the Tools column is floated in-window and then re-placed into the
  workspace
- **THEN** the floating overlay is pinned to its content height and the
  re-placed column is free to fill its column height again

### Requirement: Menu bar uses the native macOS menu bar

On macOS the application shell SHALL present its menus through the native
system-wide menu bar rather than drawing a menu bar inside the window, and SHALL
NOT disable native menu-bar use. On platforms without a native menu bar the
shell SHALL draw the menu bar inside the window. Entering a screen mode SHALL NOT
change which of these two presentations is in effect.

#### Scenario: The native bar is used on macOS

- **WHEN** the frame is shown on macOS
- **THEN** the window's menu bar reports that it is the native menu bar and no
  menu bar is drawn inside the window frame

#### Scenario: The in-window bar is used elsewhere

- **WHEN** the frame is shown on a platform without a native menu bar
- **THEN** the window's menu bar is drawn inside the window frame

#### Scenario: Screen modes keep the platform presentation

- **WHEN** the frame enters or leaves Full Screen With Menu Bar and Full Screen
  modes on macOS
- **THEN** the menu bar continues to be presented by the system, and the shell
  does not switch to an in-window menu bar

#### Scenario: Application-menu roles are honoured on macOS

- **WHEN** the menus containing the About, Preferences, and Quit commands are
  built on macOS
- **THEN** those entries are eligible for placement in the macOS Application
  Menu through the platform's menu-role merging

### Requirement: Detected watermark indicator

A document whose watermark has been read SHALL show a copyright symbol in its
title, as `docs/02-ui-ux/application-frame.md` specifies. The indicator SHALL be
per document and transient: it is set when Read Watermark decodes a payload on
that document and is absent while no watermark has been detected.

#### Scenario: A detected watermark marks the document title [dw_title]

- **WHEN** Read Watermark decodes a payload on a document
- **THEN** that document's title (its tab, float-window, or tile label) carries a
  copyright symbol

#### Scenario: An undetected watermark leaves the title plain [dw_title_plain]

- **WHEN** Read Watermark finds no watermark, or a document has never been read
- **THEN** its title carries no copyright symbol
