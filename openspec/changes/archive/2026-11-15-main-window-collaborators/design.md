# Design

## Context

See `proposal.md` for motivation. The tree this plan was measured in:

- `frame.h` is 1,200 lines — exactly the code hard cap, so 1,201 fails
  `scripts/check-file-size.sh`; it cannot grow even transiently.
- `frame*.cpp` is 45 translation units and 14,910 lines; `frame_includes.h`
  pulls 94 includes into 43 of them.
- `frame.h` has 159 members, ~408 declarations (55 test hooks), and 221
  includers: 25 production, 35 self-test, 159 `tst_*.cpp` suites, and 2 test
  helper headers (222 `tst_*.cpp` suite files and 233 sharded CTest entries in
  total).
- Inside the frame translation units: `registry_` 547 references, `tools_` 239,
  panel pointers 338, `docs_`/`windows_`/`activeWindow_` 259,
  `centerSplitter_` 76, `panelColumn_` 59.
- `PanelColumn` calls exactly 15 frame methods, all column-host operations.
  `PanelGroup` needs only the command registry and the already-separate
  `PanelMenuHookStore`. `panel_column_test.cpp` and `panel_group_menu.cpp` are
  the other frame consumers under `panels/`.
- The frame's constructor is the composition root: it loads the session, applies
  the theme, registers every command, builds the menu bar, 23 panels, the tools
  column/toolbox/options bar, the status bar, then restores the panel/workspace
  layout. Any collaborator extraction has to preserve this order and its lazy
  lookups (`layersPanel_` is read by handlers registered before the panels
  exist).
- New Qt tests are the GUI growth path; the self-test only shrinks
  (`scripts/check-selftest-budget.sh`).

## Goals / Non-Goals

**Goals:**

- Move the primary state and behavior of four concerns out of
  `PicturaMainWindow` into owned collaborators: `DocumentRegistry`,
  `PanelHost`, `ImportSaveCoordinator`, `MenuRegistrar`.
- Make each collaborator constructible and exercisable without
  `PicturaMainWindow`.
- Remove `frame.h` from the production panel and menu translation units and
  split `frame_includes.h` along the same seams.
- Keep every existing public entry point used by `main.cpp`, the control server,
  the self-test, and the 222 `tst_*.cpp` suites (including the 159 that include
  `frame.h`) behaving identically. No file under
  `crates/pictura-app/cpp/tests/` is edited; the panel test-support TU
  `panels/panel_column_test.cpp` is test-support code and may change.
- Leave a fixed-ceiling guard that fails a future panel/menu frame dependency,
  a surviving `frame_menus*.cpp`, or growth past `frame.h` <= 760 lines or
  `frame_includes.h` <= 60 include lines.

**Non-Goals:**

- Extracting a fifth `ShellChrome` or `ToolHost` collaborator. Theme, brightness,
  UI scale, screen modes, canvas colour, status bar, guides, quick mask, search,
  preferences, keyboard shortcuts, transform lock, update checks, and the
  `ToolController`/toolbox/options bar stay on the frame. Those are the natural
  next seams after this change.
- Moving the control server off the frame. It remains a shell surface; it may
  use the new collaborator accessors where convenient.
- The Layers panel model seam (#1167), per-document float/tile persistence
  (parity gap UI-15), or any behavior/session format change.
- Renaming the remaining chrome/settings frame methods or their test hooks.
- Docs edits in the same commit as code; they use a separate
  `TASK-ALLOWS-DOCS` commit.

## Decisions

### D1. Four collaborators, each owning state and behavior

| Collaborator | Owns | Must not own |
|---|---|---|
| `DocumentRegistry` | `DocEntry`/`DocWindow` lists, active window, tab/float/tile hosts, loading entries, window host operations, window arrange, canvas/title presentation; requests tab/header context menus through signals | File I/O, dialogs, panels, commands, menu presentation |
| `PanelHost` | Panel widgets, groups, columns, the tools column, central splitter, workspace store, panel menu hook store, `panelsHidden_` and panel-column visibility, panel binding/refresh, column resize/float state | Document storage, file I/O, command table |
| `ImportSaveCoordinator` | Open tokens/probes/warnings, async open pipeline, save/save-as/revert, document close prompting/saving, PDF/EPS/Open As, recent list/open, save-format options, smart-object edit sessions, recovery store/timer/snapshots | Document/window registry, panels, menu table, menu actions |
| `MenuRegistrar` | `CommandRegistry`, every handler/provider/context-menu registration, menu bar construction, dynamic menu refresh (recent/workspace/UI size), `MenuRuntime` use | Document/panel/file state; the main window |

`PicturaMainWindow` keeps `QMainWindow` overrides, chrome/settings state, the
composition root, signal wiring, the close entry point that forwards to
`ImportSaveCoordinator`, and test hooks that forward to the owning collaborator.

Alternatives considered:

- **Keep methods on the frame and add collaborator adapters that forward back.**
  Rejected: that is the facade the issue explicitly forbids, and it would not
  let a collaborator be tested alone.
- **Extract `ShellChrome` as well so `MenuRegistrar` never needs a frame-owned
  command.** Deferred: it is a fifth, much larger extraction. The registrar
  instead receives a `registerFrameCommands(CommandRegistry&)` callback from the
  composition root for the chrome/settings commands that remain frame-owned.

### D2. DocumentRegistry is a QObject that owns the window hosts

`DocumentRegistry` creates/owns the document tab widget, the document-area
stacked widget, and the tile host, and exposes `documentArea()` for `PanelHost`.
It emits:

- `activeDocumentChanged()` — the frame refreshes chrome, panels, status/title.
- `documentsChanged()` — list add/remove/reorder.
- `documentStateChanged(int index)` — title, dirty, loading/progress.
- `windowsChanged()` — host/geometry changes, screen-mode window presentation.
- `aboutToRemoveDocument(int index)` — emitted before destruction so the frame,
  `ImportSaveCoordinator`, and `PanelHost` can cancel dialogs, forget open
  tokens/recovery snapshots/tool views, and retarget panels.
- `tabContextMenuRequested(DocWindow* window, const QPoint& barPos)` and
  `headerContextMenuRequested(DocWindow* window, const QPoint& globalPos)` — the
  registry resolves the tab index/window internally; the registrar shows the
  menu and calls the registry's window operations. The registry does not own
  menu presentation.
- `closeDocumentRequested(int index)` — the last window of a document asks the
  coordinator for the unsaved prompt/save; the registry never prompts.

The registry owns the loading entry/window/tab/spinner/close-button/progress UI
and emits the change signals. It does not call the frame's `applyColorSettings`,
`rememberRecent`, `statusBar`, `alertMissingFonts`, `finishOpen`, `refresh`, or
`refreshPanels` directly: those effects arrive through `DocumentHooks` or the
coordinator. `DocumentHooks` is supplied by the composition root and covers the
canvas/host factory, view wiring, colour/GPU settings application, recent sink,
refresh sink, panel-refresh sink, and guide preparation. The standalone test
supplies simple real implementations of those hooks.

`ImportSaveCoordinator` owns open tokens/probes/warnings, the async open job,
`finishOpen`, and the post-finish effects (recent, status message, missing-font
alert). It drives the registry's loading-entry API.

`closeWindow` stays on the frame/coordinator because it needs the unsaved prompt
and save. `DocumentRegistry` exposes `detachWindow`/`destroyWindow` and the
non-last-window teardown, and emits `closeDocumentRequested` for the last window.
`ImportSaveCoordinator::closeDocument(index, interactive)` prompts and saves,
then calls `DocumentRegistry::removeDocument(index)`. `closeDocument` and
`closeActiveDocument` move to the coordinator in task 3.4; the frame forwards the
existing hooks.

### D3. PanelHost is a QObject that owns the central splitter and workspace state

`PanelHost` is constructed after `MenuRegistrar` with the shell widget,
`DocumentRegistry*`, the registrar's `CommandRegistry`, `ColorState*`,
`ActionsEngineBridge*`, and the `MenuRuntime`. It owns the `QSplitter`, the tools
column, every `PanelColumn`/`PanelGroup`/panel widget, the workspace store, the
`PanelMenuHookStore`, `panelsHidden_`, and panel-column visibility.
`PanelColumn` and `PanelGroup` receive a `PanelHost*` at construction; their
`owningFrame()` and `refreshSharedFloor(PicturaMainWindow*)` become
`owningHost()` / `refreshSharedFloor(PanelHost*)`. `PanelHost` exposes the shell
widget used to parent floating overlays, `commands()` for panel menus, and the
panel-column visibility state. The frame keeps the full-screen/menu-bar behaviour
and calls `PanelHost` to show or hide the columns. `PanelHost` creates or adopts
the tools column through `adoptToolsColumn`/`createToolsColumn`; the frame
supplies the toolbox content.

`PanelHost` emits `layoutChanged()` (session save) and
`panelVisibilityChanged()` (command checked state). It binds panels from
`bind(PictureView*, ImageView*)`; the frame calls that from its active-document
subscription. Cross-domain wiring (Layers panel signals to document/panel
actions) is passed in as callbacks the composition root supplies, so
`PanelHost` does not include `frame.h`.

### D4. ImportSaveCoordinator takes a registry and a dialog/status surface

Constructor: `DocumentRegistry&`, `QWidget* dialogParent`, `QObject* parent`, and
small status/memory-refusal callbacks. It owns the open token maps, PDF probes,
open-job wiring, recovery store, autosave timer, recent list and recent
open/maintenance, save-format options, smart-object edit sessions,
`closeDocument`, and `closeActiveDocument`. `refreshRecentMenu` and the recent
menu actions belong to `MenuRegistrar`, not this collaborator.

It exposes the current open/save/import/close entry points so the frame can keep
one-line forwarders for `main.cpp`, the control server, the self-test, and the
Qt suites. Its session slice is `recent`, `saveFormatOptions`,
`autoSaveRecovery`, `autoSaveMinutes`, and the recovery timers.

### D5. MenuRegistrar owns the command table, not the chrome

`MenuRegistrar` creates and owns the `CommandRegistry` and is constructed with a
`std::function<void(CommandRegistry&)> registerFrameCommands` callback. The
collaborators arrive later through
`setCollaborators(DocumentRegistry*, PanelHost*, ImportSaveCoordinator*,
ToolController*)` because the runtime construction order is `DocumentRegistry` →
`MenuRegistrar` → `PanelHost` → `ImportSaveCoordinator`; `ToolController*` is set
after `buildTools`. It:

1. Calls `addDefaultCommands`.
2. Registers every document/window/file/panel/layer/filter/select/transform/type
   handler and provider, including the registration sites currently in
   `frame_document_arrange.cpp` (19), `frame_panel_commands.cpp` (23), and
   `frame_workspace_controller.cpp` (6), the recent-menu actions, and the
   tab/header context menus.
3. Calls the frame-supplied callback for chrome/settings commands.
4. Builds the menu bar and keeps the dynamic menus refreshed.

Handler bodies need a runtime surface the composition root supplies as
`MenuRuntime`: `requestRefresh()` (or a `refreshRequested` signal the frame
connects), `showStatus(message, ms)`, and an `OptionsBar*` provider/accessor.
`activeView()`/`tools_`/`layersPanel_` access comes from
`DocumentRegistry`/`ToolController`/`PanelHost`. The registrar's translation
units never include `frame.h`; `tst_menu_registrar` supplies a stub chrome
registration and a stub runtime.

The frame's command registration moves to `frame_commands.cpp` and is passed as
the callback, including `showDesignSystemDialog`. `beginFreeTransform`,
`beginTransformMode`, and `selectLayerPath` stay as command-layer helpers that
call `ToolController`/`PanelHost`. Every per-menu implementation file keeps its
code and is renamed from `frame_menus_*.cpp` to `menu_*.cpp`;
`frame_menus.cpp` becomes `menu_registrar.cpp`, and no `frame_menus*.cpp`
remains. Shared helpers go in `menu_registrar_internal.h`, following the
`*_internal.h` pattern used by the panel splits.

### D6. Session state stays a frame-composed value

The frame keeps `saveSession()`/`restoreSessionWindowGeometry()` and the
`SessionState` schema. Each collaborator exposes `saveState(SessionState&)`
and `loadState(const SessionState&)` for its slice:

- `PanelHost`: `panelRailMode`, `railWidth`, `autoCollapseIconic`,
  `autoShowHidden`, `panelGroups`, `panelColumns`.
- `ImportSaveCoordinator`: `recent`, `saveFormatOptions`, `autoSaveRecovery`,
  `autoSaveMinutes`.
- `MenuRegistrar`: command shortcut overrides.
- `DocumentRegistry`: no session writing; documents and float/tile state are not
  restored today (existing gap UI-15).

No fifth session coordinator is added.

### D7. Test hooks stay on the frame; new tests construct collaborators

The 55 `ForTest` hooks stay declared in `frame.h` and become one-line forwarders
(e.g. `documents_->documentCount()`, `panels_->columnForPanel(...)`). That is the
only re-exposure permitted: it exists so the 222 `tst_*.cpp` suites (including
the 159 that include `frame.h`) stay unchanged, and it delegates to state the
collaborator owns. `PicturaMainWindow::registry()` stays a forwarding hook to
`MenuRegistrar::commands()` so existing tests do not change; the control server
may keep using that hook or be rewired as a production consumer. A new
`DocumentRegistry* documentRegistry() const` hook exposes the registry to
`tst_document_registry::frameForwardsDocuments`. The public panel/session entry
points (`panelsHidden()`, `setPanelsHidden()`, `panelColumn()`, `panelColumns()`,
`createPanelColumn()`, `columnForPanel()`, and the workspace test hooks) also
stay on `PicturaMainWindow` as forwarders to `PanelHost`; `togglePanelsHidden`
and the full-screen/menu-bar behaviour stay on the frame and call `PanelHost`.
No file under `crates/pictura-app/cpp/tests/` is edited; the panel test-support
TU `panels/panel_column_test.cpp` may change.

Four new Qt suites exercise the collaborators without a frame:

- `tst_document_registry` (including the removal-notification test)
- `tst_panel_host`
- `tst_import_save_coordinator`
- `tst_menu_registrar`

They are registered explicitly in `crates/pictura-app/cpp/tests/CMakeLists.txt`
and registered like the other suites, with sharding only where a suite is slow.

### D8. Commit order follows the dependency chain

1. `DocumentRegistry` (root state and hosts).
2. `PanelHost` (panels/columns/workspaces; uses `DocumentRegistry` and the
   frame's current command table until phase 4).
3. `ImportSaveCoordinator` (files/recovery; uses the registry).
4. `MenuRegistrar` (command layer; uses all three plus tools).
5. Include split, guard, docs, final measurements.

The finished frame's runtime construction order is `DocumentRegistry` →
`MenuRegistrar` (owns `CommandRegistry`, calls `addDefaultCommands`) → `PanelHost`
(receives `menu_->commands()`, `ColorState*`, `ActionsEngineBridge*`,
`MenuRuntime`, `DocumentRegistry*`, and the shell widget) →
`ImportSaveCoordinator` → `menu_->setCollaborators(...)` and register/build.
During phases 2–3, before `MenuRegistrar` exists, `PanelHost` takes the frame's
current `CommandRegistry` and runtime; task 4.1 re-points it.

Each commit builds, passes both headless self-tests, and leaves the existing
suites green. Pure moves and rename-only commits are separated from the thin
wiring edits where practical. The alternative — `PanelHost` first — was rejected
because it would need temporary active-view callbacks that are thrown away once
the registry lands.

### D9. The guard is a fixed-ceiling ratchet, not a style suggestion

`scripts/check-shell-seams.py` checks:

- every production file under `crates/pictura-app/cpp/panels/` has no
  `#include "frame.h"` and no `PicturaMainWindow`;
- every `menu_*.cpp`/`menu_registrar*` file has no `frame.h` and no
  `PicturaMainWindow`, and no `frame_menus*.cpp` remains;
- every collaborator translation unit has no `PicturaMainWindow` and no
  `#include "frame.h"`;
- the four collaborator headers have no `#include "frame.h"`;
- `frame.h` is at or below its fixed ceiling of 760 lines;
- `frame_includes.h` is at or below its fixed ceiling of 60 include lines.

The fixed ceilings are hard caps; the final measured values are recorded for the
PR and MUST be at or below them. There is no allowlist, and the ceilings SHALL
NOT increase. The script has a `--self-test` that runs the checks against
synthetic content so the gates are themselves testable. It runs from
`scripts/verify-fast.sh`. The target is zero panel/menu frame dependencies.

### D10. `frame_includes.h` is split, not deleted

The remaining frame translation units keep a small `frame_includes.h` containing
only what they share. Each collaborator source includes its own headers
directly; panel and menu sources include `panel_host.h`, `menu_registrar.h`, and
the domain headers they use. `frame.h` also drops `document_window.h`,
`panels/panel_column.h`, `recovery_store.h`, and `workspace_store.h` where the
moved members allow it, forward-declaring `PanelColumn`, `DocEntry`, and
`DocWindow`. A panel or menu implementation edit therefore does not recompile
the frame set, and editing `panels/panel_column.h` no longer rebuilds frame TUs.
The guard's fixed ceilings (`frame_includes.h` <= 60 include lines, `frame.h`
<= 760 lines) freeze the gain.

## Risks / Trade-offs

- **Ownership/destruction ordering** — the registry owns documents/windows and
  `PanelHost` owns columns; a naive QObject child order can leave a host pointer
  or a panel retarget dangling while widgets are torn down.
  → The frame destructor explicitly stops signals, shuts the registry down
  first, then lets `PanelHost` destroy its splitter/columns; `PanelColumn`/
  `PanelGroup` clear their host pointer in their destructors. The recovery/
  close tests plus both self-tests exercise teardown.
- **Missed cleanup on document removal** — recovery snapshots, open tokens,
  smart-object edit sessions, color-range/replace-color dialogs, and tool views
  all hang off a document today.
  → `aboutToRemoveDocument` is emitted before destruction and every owner
  connects; `mwc_removal_notifies` /
  `tst_document_registry::removingADocumentNotifiesOwners` uses a connected stub
  owner to assert the signal and ordering (the smart-object and recovered
  cases are exercised by their existing suites).
- **`MenuRegistrar` becomes a new god object** — the command layer is ~4.8k
  lines and touches everything.
  → Its header exposes only construction, registration, menu build/refresh and
  the collaborator accessors; the bodies stay in the existing per-menu files
  behind `menu_registrar_internal.h`, and the frame dependency is forbidden by
  the guard. If a single TU exceeds the cap it is split by menu family.
- **Chrome commands still live on the frame** — a new chrome behavior still
  edits a frame TU.
  → Accepted: the issue's acceptance is about panel/menu plumbing, and chrome
  remains the frame's own concern. A future `ShellChrome` extraction can remove
  the callback.
- **Test hooks are re-exposure** — `frame.h` keeps 55 forwarding hooks.
  → Accepted only for the unchanged suites under
  `crates/pictura-app/cpp/tests/`; new tests target collaborators directly and
  the guard forbids a collaborator from calling the frame.
- **Session save logic stays on the frame** — `saveSession()` still reads the
  chrome fields.
  → Collaborators supply their slices; the frame remains the schema owner, which
  keeps the session format unchanged.
- **Big diff** — roughly 12k lines are moved.
  → Land in five green commits in dependency order; keep pure moves separate
  from wiring edits; do not change behavior or files under
  `crates/pictura-app/cpp/tests/` while moving.
- **`frame.h` hard cap** — the header cannot grow while the collaborators are
  introduced.
  → The first commit of each phase removes the declarations it replaces before
  adding the forward-declared `std::unique_ptr<Collaborator>` member, so the
  header only shrinks. Never exempt `frame.h` from the 1,200-line file cap or the
  760-line shell-seam ceiling.

## Migration Plan

No user-facing migration and no persisted-format migration. The change ships as
one branch/PR (#1166) with the commit order in D8. Rollback is per commit: each
phase is independently revertible because the frame's public API and the files
under `crates/pictura-app/cpp/tests/` do not change; the panel test-support TU
`panels/panel_column_test.cpp` may change with the host pointer. The guard is
added after the moves, so a revert of the final commit only removes the ratchet,
not behavior.

## Open Questions

- None blocking. The follow-up seams (`ShellChrome`, `ToolHost`) and the Layers
  panel model seam (#1167) are explicitly out of scope and can be separate
  changes once the four collaborator boundaries are stable.
