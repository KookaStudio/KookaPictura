# Design

## Context

- `PicturaMainWindow` holds `QList<DocEntry> docs_` and `QTabWidget* tabs_`.
  `docs_[i]` is tab `i`, and `activeDocumentIndex()` returns
  `tabs_->currentIndex()`. The `tabMoved` handler (`frame.cpp:88`) moves the
  `DocEntry` to keep that equality, and `reorderDocumentsForTest()`
  (`frame_test.cpp`, called by the self-test at `selftest.cpp:880`) checks it.
- `tabs_` appears on 57 lines in 7 `frame*.cpp` files (`frame.cpp` 30,
  `frame_open.cpp` 9, `frame_columns.cpp` 7, `frame_build.cpp` 6,
  `frame_session.cpp` 2, `frame_test.cpp` 2, `frame_search.cpp` 1). `docs_`
  appears on about 80 lines in 13 files.
- `tabs_` is itself the stretch pane of `centerSplitter_`. `frame_columns.cpp`
  places panel columns by `centerSplitter_->indexOf(tabs_)`, and
  `frame_build.cpp:918` installs the empty-workspace double-click and file-drop
  filter on `tabs_` and its inner `QStackedWidget`.
- `DocEntry` mixes per-document state (`view` is the `PictureView` document,
  plus `path`, `displayName`, `recentPath`, `untitledNumber`, `loading`,
  `token`, `kind`, `userMovedAway`, `previousActiveView`) with per-window state
  (`canvas` `ImageView`, `canvasHost`, `tipPreview`, `page`, and the loading
  page's `loadingCard`).
- The canvas is fed by pushes. `setupCanvas` sets the document size, level
  provider, canvas colour, free pan, snap, and guides. `wireCanvas` connects
  `regionBlitted` to a lambda that captures one canvas, and `refresh()`
  (`frame.cpp:951`) presents only the active canvas. Several frame settings
  loop over `docs_` and set `entry.canvas` (canvas colour, free pan, snap,
  guides). There are 45 `.canvas` uses in `frame*.cpp` and about 95
  `canvasAt()` / `imageView()` calls in non-test sources.
- `PanelFloat` (`panels/panel_float.cpp`) already solves floating on Linux:
  `PanelFloat::overlayUsesTopLevel()` picks a frameless `Qt::Tool` top-level
  window parented to the frame, except on Wayland, where it uses an in-frame
  child overlay because the compositor ignores client `move()`.
  `PanelFloat::setForceChildOverlayForTest` forces child mode.
- Menu actions and the tool `QShortcut`s use the default `Qt::WindowShortcut`.
  Undo (`frame_menus.cpp:52`) and the Actions panel's function keys
  (`actions_panel_menu.cpp:539`) use `Qt::ApplicationShortcut`.
- Many keys are handled in `PicturaMainWindow::keyPressEvent` /
  `keyReleaseEvent` (`frame_keys.cpp`), not by shortcuts: `Space` panning,
  `Enter` commit, crop keys, `dispatchCommandKey` (`Ctrl+=`, `Ctrl+\`), `Esc`,
  `F`, the brush brackets, arrow nudging, and the modifier hints. They reach the
  frame only because the canvas is the frame's descendant.
- `frame.cpp` is 1198 lines, against a 1200-line cap.
- The catalogue (`pictura-script/src/command_catalogue/entries.rs:536-544`)
  defines the nine `window.arrange.*` ids as `implemented: false`. The golden
  table `crates/pictura-script/tests/golden/catalogue.tsv` and the implemented
  count in `crates/pictura-script/tests/catalogue.rs`
  (`default_catalogue_shape_is_stable`, now 342) both track that flag. An
  implemented id not listed in `NON_RECORDABLE` (`pictura-script/src/command.rs`)
  is classified as a recordable app-native command.
- The Interface preferences live in `SessionState` (`session.h` /
  `session.cpp`), wired through `frame_preferences.cpp` and
  `frame_session.cpp`. `useShiftKeyForToolSwitch` is the precedent for a boolean
  whose missing key loads `true` without a schema bump.
- Crash recovery reopens documents through `addDocument` in
  `frame_recovery.cpp:156`. The session store does not reopen documents.
- `workspaceKeys()` (`keyboard_shortcuts_dialog.cpp:172`) lists the frame keys
  that the Keyboard Shortcuts dialog reserves.

## Goals / Non-Goals

**Goals:**
- Build one window registry that serves float, consolidate, Tile, cycling, and
  multiple windows per document.
- Keep the tab-only behavior pixel- and test-identical after the refactor
  commit.

**Non-Goals:**
- Docking documents into split groups or stacks (CS6 drop zones).
- Tile variants (2-up, 3-up, and so on) beyond the single `Tile` grid.
- Saving the window layout.
- The open-document list at the bottom of the Window menu.
- The rest of the Hand and Zoom options bars (Scrubby Zoom, Fit On Screen and
  100% buttons, Bird's Eye View, flick panning); only the multi-window options
  are added.
- Resizing a window on Fit On Screen.

## Decisions

### D1. Split `DocEntry` into a document and its windows

```
DocEntry  { view(PictureView), path, displayName, recentPath, untitledNumber,
            loading, token, kind, userMovedAway, previousActiveView,
            QList<DocWindow*> windows }
DocWindow { DocEntry* doc; ImageView* canvas; CanvasScrollBars* host;
            BrushTipPreview* tip; QWidget* page; QPointer<LoadingCard> card;
            Host kind }
Host      = Tab | Float | Tile
```

- `docs_` stays the document list. The public index API (`viewAt(i)`,
  `setActiveDocumentIndex(i)`, `documentPath(i)`) keeps meaning document `i`;
  `setActiveDocumentIndex(i)` activates that document's most recently active
  window.
- A new `QList<DocWindow*> windows_` holds the cycle order. Tabs come in their
  visual order, followed by floats and tiles in the order they entered that
  host. The `tabMoved` handler reorders `windows_`, and no longer `docs_`.
- `DocWindow* activeWindow_` is the single source of truth. It is updated from
  `QTabWidget::currentChanged`, from float window activation, and from a tile
  pane taking focus. All three call one `onActiveWindowChanged()` that holds
  today's `currentChanged` body: mark loading documents `userMovedAway`, cancel
  transform sessions on other documents, `refresh()`, and `refreshPanels()`.
- `activeDocumentIndex()` becomes `indexOfDoc(activeWindow_->doc)`.
- `canvasAt(i)` returns the active window's canvas when that window belongs to
  doc `i`, and otherwise the doc's first window. Existing callers keep working.
- Tab index ↔ window conversion goes through two helpers, `windowAtTab(int)` and
  `tabOf(DocWindow*)`. These replace every direct `tabs_` index use.
- A PDF probe's temporary view (`pdfProbes_`) never enters `docs_` or
  `windows_`; only the loading tab that a chosen page starts does.
- Edit Contents of a smart object adds its untitled document through the same
  path. `editSessions_` is keyed by `PictureView`, so its windows need no
  bookkeeping, and closing a non-last window does not end the edit session.

*Alternative rejected:* keep a hidden placeholder tab for each floated document
(`setTabVisible(false)`). The diff is smaller, but a `QTabBar` whose current tab
is hidden is undefined territory. It also can't represent tile panes or several
windows per document.

The refactor ships as its own commit with no change in behavior. The existing
suites (`tst_loading_tab`, `tst_smoke`, `tst_open_save`, `tst_recovery`) and the
self-test's `reorderDocumentsForTest` check are the regression gate.

### D2. Push fan-out and per-window wiring

- Every document-wide push goes to each window of the document through
  `forEachCanvas(DocEntry&, fn)`: `setDocument`, `setImage` / `replaceImage`,
  `blitRegion`, the level provider, the filter preview, mask and channel
  overlays, and guides.
- `setupCanvas` / `wireCanvas` become per-window. Each new window gets its own
  level provider, `regionBlitted` connection, `zoomChanged`, `mouseMoved`, and
  context-menu connections, and the frame-setting loops (canvas colour, free
  pan, snap, guides) iterate windows, not documents.
- `refresh()` presents every visible window whose document's frame revision
  moved (the `kFrameKey` check), not only the active canvas. Otherwise a
  floating or tiled document that changes while another is active, for example
  when a background filter commit lands, stays stale (`dtw_inactive_repaint`).
- Tool interaction stays on the active window's canvas, as today through
  `imageView()`. Per-window state (zoom, scroll, rulers origin) stays on each
  `ImageView`.
- The `.canvas`, `canvasAt()`, and `imageView()` sites are classified once.
  Each is either a document-wide push (fan out) or an active-window interaction
  (unchanged).
- This is the riskiest part, so it lands last (group 8), after the registry has
  been proven by groups 2–7. The repaint of an inactive visible window lands
  with the float primitive (group 4), since it is needed as soon as two
  documents are visible at once.

### D3. Hosts

- **Float:** a `DocumentFloatWindow` that reuses `PanelFloat::overlayUsesTopLevel()`.
  It is a frameless `Qt::Tool` top-level window parented to the frame, except
  on Wayland, where it is an in-frame child overlay that stays below the
  floating panel overlays. The window has a header widget (label, modified
  marker, close control) that handles drag-move, the dock-drop check, and the
  header context menu. The resize grip follows `PanelEdgeGrip`. Placement by a
  command is clamped so the header stays inside the document area.
- **Document area:** a new `QStackedWidget documentArea_` replaces `tabs_` as
  the splitter's stretch pane and holds `tabs_` and the tile host. Every
  `centerSplitter_->indexOf(tabs_)` and stretch check in `frame_columns.cpp`,
  the empty-workspace filter in `frame_build.cpp:918`, and the search bar host
  in `frame_search.cpp:96` move to `documentArea_`. This lands in the refactor
  commit, with only `tabs_` in the stack, so the column layout tests guard it.
- **Tile:** a `DocumentTileHost` widget in `documentArea_`. Each pane is a header
  plus the window's canvas host, laid out with `QGridLayout` using
  `ceil(sqrt(n))` columns. When its last pane closes, the stack shows `tabs_`
  again.
- **Tab:** unchanged.
- **Moving a window between hosts** reparents its `page` (the
  `CanvasScrollBars`). The `ImageView` is never recreated, which keeps zoom and
  scroll intact.
- **Screen modes:** entering a full-screen mode hides every float and tile pane
  but the active window and remembers their visibility; returning to Standard
  restores it. `Tab` / `Shift+Tab` and workspace switching touch only panels.

The Wayland in-frame float departs from `docs/02-ui-ux/application-frame.md:241`
and `:297`, which say a Wayland float becomes a compositor-managed top-level
window. A top-level window on Wayland can't be moved by the client, so
drag-to-dock and Cascade wouldn't work. Keeping parity with `PanelFloat` gives
the same behavior on every platform. The divergence is recorded as a row in
`doc-code-parity-gaps.md`.

### D4. Keys in floating windows

- Both float kinds are parented to the frame. For an in-frame child, the
  frame's `WindowShortcut` actions and its `keyPressEvent` already apply.
- For the `Qt::Tool` top-level window, Qt's widget shortcut matcher
  (`correctWidgetContext` in `qshortcut.cpp`) treats an active `Qt::Tool`
  window that has a parent as its parent's window, so the frame's
  `WindowShortcut` actions and tool `QShortcut`s still match.
  `dtw_float_menu_shortcut` and `dtw_float_tool_shortcut` prove this in
  top-level mode. `dtw_float_shortcut` (undo) does not, because undo is an
  `ApplicationShortcut`.
- Key events do not propagate past a window boundary, so the frame's
  `keyPressEvent` / `keyReleaseEvent` never see unhandled keys from a top-level
  float. The float forwards the key press and release events its children
  leave unhandled to the frame. `dtw_float_frame_key` proves this.
- If the matcher does not cover a case, the fallback is to add the frame's
  `QAction`s to each float window (`addActions`). Changing the shortcut context
  for the whole registry is not the fallback.

### D5. Ctrl+Tab

- Two `QShortcut`s on the frame (`Ctrl+Tab`, `Ctrl+Shift+Tab`) with the default
  `Qt::WindowShortcut` context. By D4 they fire from top-level floats. Unlike
  `ApplicationShortcut`, they don't fire while a dialog is the active window,
  modal or not, which keeps a non-modal dialog such as Replace Color on the
  document it previews. A modal dialog blocks them in any case.
- They step `windows_` through `activateWindow(DocWindow*)`. They are not
  catalogue commands, because CS6 has no menu item for them. Both are added to
  `workspaceKeys()` so the Keyboard Shortcuts dialog reports a conflict.
- X11 delivers `Shift+Tab` as `Qt::Key_Backtab`. `dtw_cycle_tabs` sends the
  backward key both ways, so the sequence is proven against both forms.
- The existing `Tab` / `Shift+Tab` panel shortcuts don't match a key sequence
  that includes Ctrl, so the two don't interfere.
- `QTabWidget::keyPressEvent` cycles only tabs on Ctrl+Tab when the tab widget
  has focus. The shortcut map matches before the key press is delivered, and
  `QTabWidget` does not claim Ctrl+Tab in `ShortcutOverride`, so the frame
  shortcut wins. `dtw_cycle_tabs` runs with focus on the tab bar to confirm
  this. `Ctrl+PageUp` / `Ctrl+PageDown` stay `QTabWidget`'s and cycle tabs only.

### D6. Tear-out detection

- An event filter on `documentTabBar` tracks a press on a tab. When the pointer
  leaves the bar's rect by more than `QApplication::startDragDistance() * 3`
  perpendicular to the bar, it ends the bar's internal move (a synthesized
  release at the press point), floats that window centred under the cursor, and
  hands the drag to the float header. A loading tab is ignored.
- On release, a float header checks `tabs_->tabBar()` geometry in global
  coordinates. A hit docks the window at `tabAt(pos)`, or at the end of the
  strip, unless the docking preference is off.

### D7. Arrange and context menus

- New file `frame_document_windows.cpp` holds `wireWindowArrange()`, the tab
  context menu, and the header menu. Their handlers are registered with
  `registry_->setHandler` and enabled through `setEnabledProvider`. The nine ids
  are referenced as `QLatin1String` literals, as `frame_menus_image.cpp` does
  for `Image Size…`, so `const_names` and `command_ids.h` stay unchanged.
- The catalogue entries flip to `implemented: true`, together with their rows in
  the golden `catalogue.tsv` and the implemented count. The nine ids join
  `NON_RECORDABLE`, like `window.workspace.*` and the view commands, because
  they change only the view.
- The `New Window For` label comes from `setLabelProvider`.
- **Cascade:** step = header height + 8 px. Size = 60% of the document area. The
  windows are placed in cycle order, the cascade restarts at the top-left when a
  header would leave the document area, and the active window is raised last.
- **Match Location:** read the active canvas's centre with
  `widgetToImage(rect().center())`, convert it to document fractions, and apply
  it to each other canvas with `centerOnImageRect`. `ImageView` needs no new
  API.
- **Reveal in File Manager:** `QDesktopServices::openUrl` on the parent folder,
  through a frame-level URL opener that tests replace (as `pdfViewer_` is
  replaced). The label replaces CS6's Windows-only "Reveal in Explorer".
- **Header menu:** each entry activates its window, then dispatches the menu-bar
  command id, so enablement and the transform lock stay those of the menu bar.

### D8. Preferences

- `openDocumentsAsTabs` and `enableFloatingDocumentDocking` are created with
  `makeCheckbox` on the Interface page, emit a change signal, and are stored in
  `SessionState` like `useShiftKeyForToolSwitch`: a missing key loads `true`
  and no schema version changes.
- The frame reads `openDocumentsAsTabs` when it hosts a new `DocWindow` once its
  document is ready (a loading document keeps its loading tab), and reads
  `enableFloatingDocumentDocking` in the header drop check. Crash recovery
  always hosts as a tab.
- `docs/02-ui-ux/preferences.md:108` names the second option "Enable Floating
  Document Window". The CS6 dialog label is "Enable Floating Document Window
  Docking", which this change uses. The register records the label mismatch.

### D9. Closing and labels

- `closeWindow(DocWindow*)` deletes the window without a prompt when
  `doc->windows.size() > 1`. Otherwise it goes through the existing
  `closeDocument` path. The tab close control, a float or tile header's close
  control, and `File > Close` all call it.
- `Close All` and quit loop over documents, not windows.
- The label builder in `updateTabTitle` is split into `fullLabel(DocWindow*)`,
  which yields name, `:N`, zoom, and mode/bits. Tab text, tooltip, float header,
  and tile header all use it. Only tab text is elided to the tab width; a header
  elides to its own width. The `:N` ordinal is the window's index in
  `doc->windows` + 1, and only windows with N ≥ 2 show it, so closing a window
  renumbers the rest.

### D11. Multi-window Hand and Zoom options

- Hand and Zoom have no options-bar page today (they fall to the default page
  in `options_bar.cpp`). Add `buildHandPage` / `buildZoomPage` in a new
  `options_bar_navigate.cpp` with only the multi-window checkboxes. The values
  live in `ToolOptions` and persist with the other tool options; `Zoom Resizes
  Windows` is a `SessionState` field on the General page, read as off when
  missing.
- **Scroll All:** `ImageView` already emits `viewChanged`. The Hand pan path
  (`setPanEnabled` and `Space` pan) reports its screen delta. The frame applies
  `panBy(delta)` to every other visible window's canvas when the option is on
  or `Shift` is held. A re-entrancy guard stops the fan-out from echoing.
- **Zoom All:** `ZoomToolHandler::onRelease` computes `factor = new / old` and
  asks the frame, through `ToolServices`, to apply it to the other windows
  around their centres when the option is on or `Shift` is held. `Shift` is
  free on the Zoom tool (Ctrl/Alt mean zoom-out).
- **Resize To Fit:** after a Zoom-tool zoom, or a View zoom with `Zoom Resizes
  Windows` on, a floating window whose canvas is active is resized to
  `image size × zoom + header + scrollbar` and clamped to the document area,
  with the same clamp as Cascade. Tab and tile hosts ignore it.
- The docs' "`Match All` + `Shift`+click" is approximated by the `Shift` forms.
  Hand `Shift`+drag (Scroll All) and Zoom `Shift`+click (Zoom All) cover
  location and zoom separately. This is recorded as an approximation in the spec.

### D10. File layout

New sources keep `frame.cpp` under its cap:
- `document_window.{h,cpp}`: the `DocWindow` struct and the label builder
- `document_float_window.{h,cpp}`
- `document_tile_host.{h,cpp}`
- `frame_document_windows.cpp`: the window registry, hosting, New Window For,
  moving windows between tab, float, and tile, closing, presenting, cycling,
  and the screen modes
- `frame_document_menus.cpp`: the tab and float-header context menus and
  tab tear-out
- `frame_document_arrange.cpp`: `Window > Arrange` (Float, Consolidate,
  Cascade, Tile, Match, New Window For) and the multi-window Hand and Zoom
  fan-out
- `options_bar_navigate.cpp`: the Hand and Zoom options-bar pages

`frame_canvas.cpp` makes `setupCanvas` / `wireCanvas` per-window, adds
`wireDocument` for the once-per-document view connections and the
`forEachCanvas` pushes, and reports Hand pans for Scroll All;
`tool_hand_zoom.cpp` reports Zoom-tool factors. All new sources are listed
explicitly in the root `CMakeLists.txt`. A QObject's `.cpp` ends with
`#include "moc_<name>.cpp"`. `frame.cpp` loses its tab-title code to
`document_window.cpp`. The four Qt Test suites (`tst_document_windows`,
`tst_document_float`, `tst_document_arrange`, `tst_document_navigate`) are
added to `PICTURA_QT_TESTS` in `crates/pictura-app/cpp/tests/CMakeLists.txt`;
a suite goes into `PICTURA_QT_TEST_SHARDS` (with `shardedMain`) only if it
turns out slow, which its per-test window allows.

## Risks / Trade-offs

- [Refactor regressions across ~140 `docs_`/`tabs_` sites] → The refactor is its
  own commit with no behavior change, gated by the existing tab, session,
  column-layout, and loading suites before any feature lands.
- [A missed fan-out site leaves the second window stale] → `dtw_new_window`
  paints, filters, and toggles a mask overlay with two windows open, then
  compares both canvases' images.
- [The tear-out event filter fights `QTabBar`'s movable drag] → Tear-out fires
  only on perpendicular exit. `dtw_tear_reorder` guards the sideways case.
- [Offscreen QPA has no real window manager, so global positions are
  synthetic] → Geometry checks (Cascade, dock drop) use in-frame hosting. The
  top-level key checks need the float to be `QApplication::activeWindow()`; if
  offscreen activation of a `Qt::Tool` window proves unreliable (open question
  10, accepted), those rows move
  to the manual list and the test plan says so.
- [Frame code that asks whether the frame is the active window] → With a
  top-level float active, `isActiveWindow()` on the frame is false. The focus
  and Space-pan guards are checked against the float during group 4.
- [Tool-window shortcut matching differs between Qt versions] → D4 fallback.
- [A loading document is floated or tiled mid-load] → Every path that moves a
  window refuses one whose document is `loading`.
- [Large diff in one PR] → The commits are sequenced so each one is expected to
  be green on its own (see tasks.md).

## Migration Plan

There is no data migration. Nothing is persisted apart from three new preference
keys (`openDocumentsAsTabs`, `enableFloatingDocumentDocking`,
`zoomResizesWindows`) and three new tool options (`Scroll All Windows`, `Zoom
All Windows`, `Resize Windows To Fit`), whose defaults when missing keep
today's behavior.

## Open Questions

- Exact Cascade step and size, and the tile header height. These are cosmetic
  and get tuned during implementation.
