# Workspace Management

- **Spec ID:** `WF-021`
- **Status:** `Draft`
- **Parity tier:** `Core` — saved workspaces, keyboard-shortcut sets, and menu customization are in CS6 Standard and Extended.
- **New in CS6:** `Changed` — CS6 removes the Application bar and moves the workspace switcher into the dock area (the CS6 Help's generic workspace chapter still prints CS5-era "Application bar" text, see Open questions); the startup reset gesture now also resets custom shortcuts, workspaces, and color settings; workspaces automatically remember their last arrangement until an explicit reset.
- **Depends on:** `UI-003` workspace-and-docks, `UI-002` menus, `UI-011` keyboard-shortcuts, `UI-010` preferences, `01-architecture/qt6-ui-design.md`, `01-architecture/document-model.md`, `11-cross-cutting/preference-storage.md`, `10-workflow-io/presets-manager.md` (`WF-020`).

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the
> behavioural contract for the feature, and `docs/dev/doc-code-parity-gaps.md`
> records what shipped and what did not. Facts confirmed by the fetched CS6
> Help reference are stated plainly; community/`*.psp`-documented facts are
> marked *(secondary)*; inferred design choices are marked *(inferred)*.

## CS6 behavior

### Workspaces

A **workspace** is a named arrangement of panels (and, in CS6, optionally a
captured shortcut set and menu set). The switcher lives in the dock area in CS6
(it was in the removed Application bar in CS5).

- **Save:** `Window > Workspace > New Workspace…`; type a name; under
  **Capture** select one or both:
  - **Keyboard Shortcuts** — saves the current shortcut set (Photoshop only).
  - **Menus** (Help also calls it *Menu Customization*) — saves the current menu
    set.
  Saved names appear in the workspace switcher.
- **Switch:** choose from the workspace switcher (or `Window > Workspace >`).
  The same document can be shown in any workspace. Shortcuts can be assigned to
  workspaces to switch quickly.
- **Delete:** select `Manage Workspaces`/the workspace in the switcher, or
  `Window > Workspace > Delete Workspace…`, then confirm.
- **Reset one workspace:** `Window > Workspace > Reset [Workspace Name]`
  restores the original saved arrangement.
- **Restore all built-ins:** **Restore Default Workspaces** in the Interface
  preferences.
- **Reorder:** drag workspaces in the switcher.
- **Auto-remember:** "workspaces automatically appear as you last arranged
  them" — switching away and back does **not** restore the saved arrangement;
  only an explicit **Reset** does.

### Keyboard shortcut sets

The **Keyboard Shortcuts & Menus** dialog (`Edit > Keyboard Shortcuts`, or
`Window > Workspace > Keyboard Shortcuts & Menus` → Keyboard Shortcuts tab) is
a shortcut editor over named sets.

- **Set menu:** choose the active set (defaults + custom sets).
- **Shortcuts For:** `Application Menus`, `Panel Menus`, `Tools`.
- **Assign:** select a row, type a new shortcut. A collision alert offers
  `Accept` (reassign, erasing the previous owner), `Undo Changes`, or
  `Accept and Go To Conflict`.
- **Save Set** writes changes to a custom set; for the Photoshop Defaults set it
  prompts for a new name. **Save Set As** creates a new set based on the current
  one. **Delete Set** removes a set (after confirmation). **Delete Shortcut**
  clears one binding. **Use Default** restores one row. **Summarize** exports
  the current set to an HTML file for viewing/printing.
- Changing to an unsaved set is discarded by `Cancel`.

### Menu customization

`Edit > Menus` (or `Window > Workspace > Keyboard Shortcuts & Menus` → Menus
tab) edits named **menu sets**.

- **Set menu:** choose the set.
- **Menu For:** `Application Menus` or `Panel Menus`.
- **Visibility button:** show/hide individual items; clicking the color swatch
  assigns a menu color (per `Interface > Show Menu Colors`).
- **Save Set / Save Set As / Delete Set** mirror the shortcut-set operations.
- **Temporarily show hidden items:** `Show All Menu Items` at the bottom of a
  menu with hidden entries, or `Ctrl`/`Cmd`-click the menu. Closing the menu
  returns items to hidden.
- **Permanently reveal all items:** select `Window > Workspace > Essentials`.
- Global toggle: `Interface > Show Menu Colors`.

### Tool preset persistence

Tool presets (`WF-020`, `02-ui-ux/panels/tool-presets-panel.md`) are stored in
`ToolPresets.psp` in the version's Settings folder and can be saved as `.tpl`
libraries. They are captured by a workspace only insofar as the Tool Presets
panel is part of the panel layout; the preset data itself is separate and is not
captured by **Keyboard Shortcuts**/**Menus**.

### Cross-platform workspace files

Per Adobe's preference-file reference (version-templated; community confirms CS6
by substituting the version):

| Artifact | File | Location |
|---|---|---|
| Custom workspaces | `<Workspace Name>.psw` | `.../Adobe Photoshop [version] Settings/WorkSpaces/` |
| Modified built-ins | `Essentials.psw`, `[Name].psw` | `.../Settings/WorkSpaces (Modified)/` |
| Workspace list/state | `Workspace Prefs.psp` | `.../Settings/` |
| Shortcut sets | `Keyboard Shortcuts.psp`, `Keyboard Shortcuts Primary.psp` | `.../Settings/` |
| Menu sets | `Menu Customization.psp`, `Menu Customization Primary.psp` | `.../Settings/` |
| Tool presets | `ToolPresets.psp` | `.../Settings/` |

macOS base: `~/Library/Preferences/Adobe Photoshop [version] Settings/`; Windows
base: `Users\<user>\AppData\Roaming\Adobe\Adobe Photoshop [version]\Adobe
Photoshop [version] Settings\`. CS6-specific path (community-verified):
`.../Adobe Photoshop CS6/Adobe Photoshop CS6 Settings/`.

*(secondary)* The CS6 community also treats `.kys` as the exported keyboard
shortcut set and `.mnu` as the exported menu set (see `WF-020`).

### Resetting

- **One workspace:** `Window > Workspace > Reset [Name]`.
- **All built-ins:** Interface preferences → `Restore Default Workspaces`.
- **Everything:** hold `Alt+Ctrl+Shift` (Windows) / `Option+Command+Shift`
  (macOS) while launching and confirm; the CS6 Help states this also resets
  custom shortcuts, workspaces, and color settings.
- **Reset the current shortcut/menu set:** `Use Default` (row) or delete the
  custom set.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Workspace switcher | Drop-down / menu | — | Dock area in CS6. |
| `Window > Workspace > New Workspace…` | Menu | — | Capture Keyboard Shortcuts / Menus. |
| `Window > Workspace > Delete Workspace…` | Menu | — | Confirm dialog. |
| `Window > Workspace > Reset [Name]` | Menu | — | Per workspace. |
| `Window > Workspace > Keyboard Shortcuts & Menus` | Menu | — | Opens the two-tab dialog. |
| `Window > Workspace > Essentials` | Menu | — | Permanently reveals hidden menu items. |
| `Edit > Keyboard Shortcuts` | Dialog | reported `Ctrl/Cmd+Alt/Option+Shift+K` | Shortcut-set editor. |
| `Edit > Menus` | Dialog | — | Menu-set editor. |
| `Interface > Restore Default Workspaces` | Preference button | — | All built-ins. |
| `Show All Menu Items` | Menu item / `Ctrl`·`Cmd`-click | modifier | Temporarily reveals hidden items. |
| `Alt+Ctrl+Shift` (Win) / `Cmd+Option+Shift` (Mac) at launch | Startup gesture | — | Reset workspaces/shortcuts/color. |
| Tool Presets panel | Dock | — | `.tpl`; `ToolPresets.psp` persistence. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Workspace name | string | `Workspace N` | any Unicode; unique | Collides with built-ins ⇒ reject/namespace. |
| Capture: Keyboard Shortcuts | bool | Off | on / off | New Workspace dialog. |
| Capture: Menus | bool | Off | on / off | New Workspace dialog. |
| Workspace order | list | built-in order | user-drag | Persisted. |
| Active shortcut set | enum | Photoshop Defaults | defaults + custom | Keyboard Shortcuts dialog. |
| Shortcuts For | enum | Application Menus | Application Menus / Panel Menus / Tools | Editor scope. |
| Active menu set | enum | Photoshop Defaults | defaults + custom | Menus dialog. |
| Menu For | enum | Application Menus | Application Menus / Panel Menus | Editor scope. |
| Menu item visibility | tri-state | default | shown / hidden | Hidden items reachable via Show All. |
| Menu item color | enum/color | None | None + 7 colors | `Show Menu Colors`. |
| Tool preset file | `.tpl` | — | — | Separate from workspace capture. |

## Algorithms & pipeline

### Workspace model and application

A workspace is UI state; the panel-layout portion reuses `UI-003`'s
`{ name, layout: QByteArray (saveState), panelWidths, iconMode[], activeTabs[] }`
and adds optional captured sets:

```text
Workspace {
  name:            String,
  layout:          LayoutBlob,          # QMainWindow::saveState()
  panel_widths:    Vec<(PanelId, f32)>,
  icon_modes:      Vec<(DockId, IconState)>,
  active_tabs:     Vec<PanelId>,
  shortcut_set:    Option<ShortcutSetId>,   # captured ("Keyboard Shortcuts")
  menu_set:        Option<MenuSetId>,       # captured ("Menus")
}
```

- **apply(ws):** `restoreState(ws.layout)`; restore widths/icon modes; if
  `ws.shortcut_set`/`ws.menu_set` are set, activate them.
- **reset(ws):** re-apply the *factory* layout captured at build time, because
  CS6 remembers the last-used arrangement between switches (`UI-003`).
- **persist:** versioned blob; never in PSD/XMP.

### Shortcut-set resolution

```text
resolve(key_event):
    if key_event in FKey_action_map:      -> play action        # AUTO-001 precedence
    if key_event in active_shortcut_set:  -> dispatch command
    elif key_event in defaults:           -> dispatch command
    else:                                 -> unhandled
```

Custom sets are sparse overlays on the defaults; assignment is last-writer-wins
with an explicit conflict prompt. `Summarize` serializes the *merged* set to
HTML.

### Menu-set resolution

Per menu path, a tri-state (`default` / `shown` / `hidden`) plus optional color.
Effective menu = defaults, overlaid by the active set. `Show All Menu Items` is
a transient override that renders hidden rows for the current pop-up only.

### Persistence and coupling

- Shortcut sets and menu sets are independent stores referenced **by id** from a
  workspace; deleting a set leaves a dangling reference that must degrade to the
  default set, not fail.
- The startup reset gesture moves/ignores the relevant `*.psp`-equivalents
  before they are read.

### Proposed Linux mapping

| Artifact | Proposed path |
|---|---|
| Workspaces | `$XDG_CONFIG_HOME/kooka-pictura/workspaces/<name>.work` |
| Workspace index/state | `$XDG_CONFIG_HOME/kooka-pictura/workspaces.toml` |
| Shortcut sets | `$XDG_CONFIG_HOME/kooka-pictura/shortcuts/<name>.shortcuts.toml` |
| Menu sets | `$XDG_CONFIG_HOME/kooka-pictura/menus/<name>.menus.toml` |
| Tool presets | `$XDG_DATA_HOME/kooka-pictura/presets/Tools/` (`WF-020`) |
| Layout blob | Qt `saveState()` bytes, base64 in the workspace file *(inferred)* |

TOML/JSON is a deliberate deviation from Adobe's `.psp`/`.psw` binaries; CS6
import is opt-in and independent-creation only (`00-overview/licensing-and-provenance.md`).

## Rust module mapping

Proposals; the panel-layout half overlaps `UI-003`.

- `pictura_ui::workspace::Workspace` / `Registry` / `persist` — as in `UI-003`,
  extended with `shortcut_set: Option<ShortcutSetId>` and
  `menu_set: Option<MenuSetId>`.
- `pictura_ui::shortcuts::ShortcutSet` — id, name, sparse overrides
  (`KeyChord -> CommandId`); `assign`, `clear`, `resolve`, `summarize_html`.
- `pictura_ui::shortcuts::Conflict` — `{ chord, existing: CommandId, incoming:
  CommandId }` for the accept/undo/goto prompt.
- `pictura_ui::menus::MenuSet` — per-menu-path `Visibility` tri-state and
  `Option<MenuColor>`; `effective(defaults)` overlay; `resolve_show_all`.
- `pictura_ui::gestures::reset` — parses the startup reset gesture and returns a
  `ResetScope { prefs, shortcuts, workspaces, color }`.
- `pictura_ui::storage` — versioned load/save; dangling-reference fallback.
- `pictura_ui::docks::PanelId` / `DockId` — stable ids matching `objectName`s.

Crossing types: `WorkspaceName`, `ShortcutSetId`, `MenuSetId`, `KeyChord`,
`CommandId`, `MenuPath`, `MenuColor`, `LayoutBlob` (opaque to Rust).

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PicturaMainWindow` | `QMainWindow` | Owns dock areas and the workspace `saveState()`/`restoreState()` (`UI-003`). |
| `WorkspaceSwitcher` / `WorkspaceController` | `QComboBox`+menu / `QObject` | List, apply, reset, save, delete (`UI-003`); extended for captured sets. |
| `ShortcutSetDialog` | `QDialog` | Set menu, Shortcuts For, editor list, conflict prompt, Save Set/As, Delete Set, Summarize. |
| `ShortcutSetModel` | `QAbstractItemModel` | Rows: command, shortcut, scope, default; conflict roles. |
| `MenuSetDialog` | `QDialog` | Set menu, Menu For tree, visibility buttons, color swatches, Save/Delete Set. |
| `MenuSetModel` | `QAbstractItemModel` | Menu tree with tri-state check + color role. |
| `WorkspaceNameDialog` | `QDialog` | Name + Capture Keyboard Shortcuts / Menus checkboxes. |
| `ResetDefaultsDialog` | `QMessageBox`-style | Startup gesture confirmation and scope report. |
| `ShowAllMenuItemsFilter` | `QObject`/event filter | Injects the transient `Show All Menu Items` entry. |

Widgets, not QML: desktop menu/shortcut editing and dock-state serialization are
Widgets-native; QML is not used for the application frame (`ARCH-003`).
`QMainWindow::saveState()` keys by `objectName`, so every dock/toolbar needs a
stable unique name before first restore (`UI-003`).

## Data-model impact

- **Not document data.** Workspaces, shortcut sets, menu sets, and tool-preset
  lists are application state; they never serialize into PSD/XMP and never enter
  the document History stack.
- **Preference records:** `WorkspaceIndex`, `Workspace{...}`, `ShortcutSet`,
  `MenuSet`, active ids; each with a schema version and forward migration.
- **No undo:** applying a workspace or editing a set adds no History state
  (matches CS6's "program-wide changes are not reflected in the History panel").
- **Capture semantics:** capturing a shortcut/menu set must snapshot or
  reference-by-id unambiguously; the spec proposes reference-by-id plus a
  fallback to defaults if the set is later deleted.
- **Menu model:** the CS6 menu tree is defined in `UI-002`; menu sets only carry
  visibility/color overlays keyed by stable menu-path ids.

## Edge cases

- **Dangling captured set:** a workspace references a deleted shortcut/menu set
  ⇒ fall back to the default set and warn once, do not fail to apply.
- **Duplicate workspace name** vs a built-in (`Essentials`) ⇒ reject or
  namespace.
- **Corrupt/old layout blob** ⇒ fall back to Essentials, keep user workspaces.
- **Unknown panel id** in an old workspace ⇒ ignore just that entry.
- **Wayland/multi-monitor:** restore relative layout only; clamp floating
  geometry to available screens (`UI-003`).
- **High-DPI change:** persist widths as DIP; rebuild icons on
  `devicePixelRatio` change.
- **Conflict chains:** reassigning a shortcut may cascade conflicts; the editor
  must offer accept/undo/goto without silently dropping bindings.
- **Reserved function keys:** action-assigned `F2`–`F12` override command
  shortcuts (`AUTO-001`); the editor must warn, not block.
- **Hidden menu with all items hidden:** `Show All Menu Items` must still be
  reachable.
- **Startup reset gesture while another instance runs:** refuse and warn.
- **Read-only config dir:** run with in-memory sets; do not lose the session.
- **Tool presets vs workspace capture:** saving a workspace does not imply
  saving tool presets; document the distinction (CS6 keeps them separate).

## Parity acceptance criteria

1. Given a rearranged layout, `New Workspace…` with **Capture Keyboard
   Shortcuts** and **Menus** saves it; selecting it later restores layout, the
   captured shortcut set, and the captured menu set.
2. Given a modified built-in workspace, `Reset [Name]` restores its factory
   arrangement; `Restore Default Workspaces` restores every built-in.
3. Given a workspace that is switched away from and back without reset, it shows
   the last-used arrangement, not the saved one (CS6 auto-remember behavior).
4. Given a shortcut reassignment over an existing binding, the conflict prompt
   appears and `Accept` moves the shortcut; `Undo Changes` reverts.
5. Given `Save Set` on the Photoshop Defaults set, Photoshop-style naming prompt
   creates a new custom set; `Delete Set` removes a custom set after
   confirmation.
6. Given `Summarize`, an HTML listing of the merged shortcut set is produced.
7. Given a menu set with hidden items, those items are absent from the menu,
   present under `Show All Menu Items` / `Ctrl`-click transiently, and
   permanently revealed by `Window > Workspace > Essentials`.
8. Given `Interface > Show Menu Colors` off, menu colors are not drawn; on, they
   are.
9. Given the startup reset gesture and confirmation, custom workspaces,
   shortcut sets, and menu sets are reset while documents are untouched.
10. Given a deleted captured set, applying the workspace that referenced it
    falls back to defaults and reports the missing set.
11. Given `.tpl` tool presets, saving/loading them is independent of workspace
    capture and survives a restart (via the tool-preset store).

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help (downloaded with `curl`, text-extracted with
  `pdftotext`). Establishes: `Window > Workspace > New Workspace`, Capture
  Keyboard Shortcuts / Menus; delete and reset workspace; `Restore Default
  Workspaces`; automatic last-used arrangement and explicit reset; keyboard
  shortcut set editor (Set menu, Shortcuts For Application/Panel/Tools, Save
  Set, Save Set As, Delete Set, Delete Shortcut, Use Default, Summarize, conflict
  alert); `Edit > Menus` menu editor (Set menu, Menu For, show/hide, colors,
  Save/Delete Set); `Show All Menu Items` and `Ctrl`/`Cmd`-click transient
  reveal; `Window > Workspace > Essentials` permanent reveal; `Show Menu Colors`;
  the startup `Alt+Ctrl+Shift` / `Option+Command+Shift` reset that also resets
  shortcuts, workspaces, and color settings.
- `https://web.archive.org/web/20240419165453/https://helpx.adobe.com/photoshop/kb/preference-file-names-locations-photoshop.html`
  — Adobe "Preference file functions, names, locations": `.psw` custom
  workspaces under `WorkSpaces/`, modified built-ins under `WorkSpaces
  (Modified)/`, `Workspace Prefs.psp`, `Keyboard Shortcuts.psp` / `Keyboard
  Shortcuts Primary.psp`, `Menu Customization.psp` / `Menu Customization
  Primary.psp`, `ToolPresets.psp`, plus the macOS/Windows base paths. Direct
  `helpx.adobe.com` fetch returns HTTP 403; this is the Wayback capture.
- `https://photoshop-viz.blogspot.com/2013/03/photoshop-file-extensions.html` —
  `.KYS` keyboard shortcuts (`Edit > Keyboard Shortcuts`) and `.MNU` custom
  menus (`Edit > Menus`). Secondary/community.
- `https://www.gottheknack.com/a-user-guide/ps/ps-presets-list/ps-presets-list-01.html`
  — `.mnu` (Menu Customization) and `.tpl` (Tools) extensions. Secondary.
- `https://doc.qt.io/qt-6/qsettings.html` — `QSettings` persistence and XDG
  location rules used for the Linux storage proposal.
- Cross-references: `docs/02-ui-ux/workspace-and-docks.md` (`UI-003`),
  `docs/02-ui-ux/menus.md` (`UI-002`), `docs/02-ui-ux/keyboard-shortcuts.md`
  (`UI-011`), `docs/09-automation/actions.md` (`AUTO-001` function-key
  precedence), `docs/10-workflow-io/presets-manager.md` (`WF-020`).

## Open questions

- **Workspace switcher location in CS6.** The CS6 Help's workspace chapter is
  generic CS5-era text referencing the Application bar, which CS6 removed;
  `UI-003` (with a CS6 secondary source) places the switcher in the dock area.
  *Resolves with:* a CS6 screenshot and/or the CS6-specific Help page.
- **CS6 `.psw`/`.psp` extension set.** The `.psw` filename and `WorkSpaces`
  folders are from the version-templated modern article, corroborated for CS6 by
  community posts, not a CS6 primary source. *Resolves with:* a CS6 Settings
  folder capture.
- **Whether workspaces capture tool presets.** The New Workspace **Capture**
  options are only Keyboard Shortcuts and Menus in the CS6 Help; confirm no
  hidden capture of tool presets. *Resolves with:* a CS6 workspace round-trip
  test.
- **`Show All Menu Items` scope.** Whether the transient reveal survives
  submenu navigation and panel menus exactly as described. *Resolves with:* a
  CS6 interaction pass.
- **Shortcut conflict cascade semantics.** Exact behavior when a reassignment
  creates multiple conflicts is only partly documented. *Resolves with:* a CS6
  edit experiment.
- **Storage format choice.** TOML vs JSON vs a Qt-native format for the Linux
  stores is a project decision; the layout blob may need to remain opaque Qt
  bytes. *Resolves with:* `01-architecture/qt6-ui-design.md` and
  `11-cross-cutting/preference-storage.md`.
- **Sharing workspaces across machines.** Whether Kooka Pictura should offer a
  portable workspace bundle (like `WF-020`'s Export/Import Presets) is a product
  decision.
