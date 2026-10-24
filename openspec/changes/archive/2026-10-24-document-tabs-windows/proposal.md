# Proposal

## Why

The document area is a single `QTabWidget` whose tab index is the document
index. Only `Tab` is bound, so a document can't be torn out into its own window,
and every `Window > Arrange` leaf is an inert stub. The parity audit (#486)
flagged `Ctrl+Tab` cycling (UI-3) and Arrange (UI-4) as missing. #317 asks for
tab tear-out/re-dock and the tab/header context menus. #917 is a tooltip that
shows the elided label, or the path, instead of the full label. All four rest
on one missing primitive: a registry of document windows that can live
somewhere other than the tab strip.

## What Changes

- The document tab tooltip shows the full, unelided label (name, zoom, mode/bits)
  for every document, saved or untitled, instead of the path (#917).
- A document is split from the windows that show it. Each open document owns one
  or more *document windows* (canvas + viewport). Each window is hosted in the
  tab strip, a floating window, or a tile pane. The active document follows the
  active window.
- `Ctrl+Tab` / `Shift+Ctrl+Tab` cycle every document window across all hosts
  (#499).
- Dragging a tab out of the tab strip floats it; dragging a floating window's
  header back over the strip re-docks it. A right-click on a tab opens the
  document-tab menu (Consolidate All to Here, Close, Close All, Close Others,
  Move to New Window, New Document…, Open Document…, Reveal in File Manager). A
  right-click on a floating window's header opens Duplicate…, Image Size…,
  Canvas Size…, File Info…, Print… (#317).
- `Window > Arrange` is implemented: Cascade, Tile, Float in Window, Float All in
  Windows, Consolidate All to Tabs, Match Zoom, Match Location, Match All, and
  New Window For [file] (#500).
- `File > Close` and every close control close one window; the document closes
  with its last window.
- Preferences > Interface gains `Open Documents As Tabs` and `Enable Floating
  Document Window Docking`, both on by default.
- Menu shortcuts, tool shortcuts, and frame keys keep working while a floating
  document window has focus.
- The full-screen modes show only the active document window; panel hiding and
  workspace switching leave document windows alone.
- The Hand options bar gains `Scroll All Windows`. The Zoom options bar gains
  `Zoom All Windows` and `Resize Windows To Fit`. Preferences > General gains
  `Zoom Resizes Windows`. `Shift` gives the all-windows form of a Hand drag or
  Zoom click.
- Floating, tiled, and multi-window layouts are not persisted. Crash-recovered
  documents reopen as tabs.

## Capabilities

### New Capabilities

- `document/document-windows`: document windows and their hosts (tab, floating,
  tile pane), float/re-dock, the floating-window header menu, keys in floating
  windows, Window > Arrange, multiple windows per document, screen modes and
  panel controls with floating windows, the two Interface preferences, the
  multi-window Hand and Zoom options, and non-persistence of the layout.

### Modified Capabilities

- `document/document-tabs`: the tabbed document area becomes the tab host of a
  document window; active-document targeting extends to windows outside the tab
  strip; closing a tab or using `File > Close` closes one window. Adds the
  full-label tooltip, `Ctrl+Tab` cycling, and the document-tab context menu.

## Impact

- `crates/pictura-app/cpp`:
  - `frame.h` / `frame.cpp` (`DocEntry`, `activeDocumentIndex`,
    `updateTabTitle`, `refresh`, `closeEvent`)
  - `frame_session.cpp`, `frame_open.cpp`, `frame_build.cpp`,
    `frame_columns.cpp`, `frame_search.cpp`, `frame_test.cpp`,
    `frame_recovery.cpp`, `frame_menus.cpp`, `frame_canvas.cpp`,
    `frame_guides.cpp`, `frame_keys.cpp`, `frame_preferences.cpp`, and the other
    `docs_` / `tabs_` users
  - `preferences_dialog.*`, `session.{h,cpp}`
  - `keyboard_shortcuts_dialog.cpp` (`workspaceKeys()`)
  - new `document_window*`, `document_float_window*`, `document_tile_host*`, and
    `frame_document_windows.cpp` sources, added to `CMakeLists.txt`
- `frame.cpp` sits at 1198 of its 1200-line cap, so new logic goes in new files.
- `crates/pictura-script`: the nine `window.arrange.*` entries in
  `src/command_catalogue/entries.rs` flip to `implemented: true` and join
  `NON_RECORDABLE` in `src/command.rs`; `tests/golden/catalogue.tsv` and the
  implemented count in `tests/catalogue.rs` follow.
- `options_bar.cpp` and a new `options_bar_navigate.cpp`, `tool_hand_zoom.cpp`,
  `tool_options.h`, and `image_view.*` for the Hand and Zoom options.
- New Qt Test suites `tst_document_windows`, `tst_document_float`,
  `tst_document_arrange`, and `tst_document_navigate`. `tst_loading_tab` gets two tooltip checks. No
  self-test check is added.
- `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (UI-3 and UI-4 closed, new
  divergence rows), and `docs/dev/STATE.md` are updated.
- No new dependencies.
