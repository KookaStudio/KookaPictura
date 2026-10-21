# Workspace and Docks

- **Spec ID:** `UI-003`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the Application bar was removed and the workspace switcher moved into the **Options bar** (top-right, directly above the panel columns); CS6 ships the predefined workspaces `Essentials (Default)`, `New in CS6`, `3D`, `Advanced 3D` (Extended), `Motion`, `Painting`, `Photography` and `Typography`; a darker interface with **four brightness themes** and `Auto-Collapse Iconic Panels` shape dock behavior.
- **Depends on:** `UI-001` application-frame, `UI-002` menus, `ARCH-003` qt6-ui-design, `01-architecture/document-model.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts confirmed by the fetched CS6 Help reference and other
> CS6-era primary/secondary sources are stated plainly; reconstructed/community
> facts are marked *(inferred)* or `(to verify)`.

## CS6 behavior

### Panels and panel groups

- A **panel** is a dockable module (Layers, Color, History, …). Panels can be
  **grouped, stacked, or docked**. A **panel group** is several panels sharing
  one title bar, switched by their tabs.
- Each panel has a **tab** (with its name) and a **panel title bar** — the solid
  empty bar above the tabs. The **panel menu** icon sits at the **upper-right**
  corner of the panel and opens even when the panel is minimised.
- The **Window** menu lists every panel (plus `Window > Options` for the options
  bar). A checkmark means the panel is open **and active** in its group — a panel
  nested behind another active tab is open but unmarked. The complete CS6 panel
  inventory (Help reference; *Adobe Photoshop CS6 for Photographers*):

| Panel | CS6 notes |
|---|---|
| 3D | Photoshop Extended |
| Actions | |
| Adjustments | adjustment-layer launcher; its CS5 adjustment-controls mode moved to Properties |
| Animation | frame-by-frame animation; the Help also documents a Timeline mode |
| Brush | |
| Brush Presets | |
| Channels | |
| Character | |
| Character Styles | |
| Clone Source | |
| Color | |
| Histogram | |
| History | |
| Info | |
| Layer Comps | |
| Layers | |
| Measurement Log | Photoshop Extended |
| Mini Bridge | `Window > Extensions > Mini Bridge` |
| Navigator | |
| Notes | new panel in CS6 |
| Options | `Window > Options` (options bar, not a dock panel) |
| Paragraph | |
| Paragraph Styles | |
| Paths | |
| Properties | new in CS6; absorbs the former Adjustments-controls mode and the CS5 Masks panel |
| Styles | |
| Swatches | |
| Timeline | clip-based video timeline (CS6 redesign) |
| Tool Presets | |
| Tools | Tools-panel visibility toggle |

- **Not CS6 panels:** `Libraries` (introduced Photoshop CC 2014), `Gradients`,
  `Patterns` and `Shapes` (all three added in Photoshop 2020/v21), and `Masks`
  (CS4–CS5, folded into Properties in CS6). The task brief's
  `Properties/Adjustments/Libraries` grouping is not a CS6 arrangement.
- Default CS6 **Essentials** layout: two columns on the right. The wider
  **main column** holds three stacked groups — `Color|Swatches`,
  `Adjustments|Styles`, `Layers|Channels|Paths`. The narrower **secondary
  column** to its left is collapsed to icons — `History` and `Properties`. The
  official Adobe CS6 Project 1 guide additionally shows `Mini Bridge` and
  `Timeline` docked beneath the document window (see Open questions).
- The **workspace switcher** sits at the top-right of the window, at the right
  end of the **Options bar**, immediately above the panel columns.
- CS6 added **panel text font size** (`Interface > UI Font Size`, Small/Medium/Large,
  applied after relaunch), **Enable Text Drop Shadows**, and **on-image displays**.
  The interface offers **four colour themes** (brightness levels; the default is
  dark) selected as swatches in the Interface preferences, and the canvas colour
  is chosen per screen mode. `Shift+F1` darkens and `Shift+F2` lightens the theme
  (the CS6 Help body also prints `Shift+1`/`Shift+2` for the same action).

### Docking, floating, and moving

- A **dock** is a collection of panels/groups, generally vertical. Create a dock
  by dragging panels to an edge until a **drop zone** appears.
- **Dock a panel:** drag it by its tab to the top, bottom, or between panels.
  **Dock a group:** drag it by its title bar.
- **Undock/float:** drag a panel or group out by its tab/title bar, into another
  dock, or free-floating.
- While moving, **blue highlighted drop zones** appear. The *pointer* position
  activates a drop zone, not the panel position (important UX detail).
- **Prevent docking:** hold `Ctrl` (Windows) / `Cmd` (Mac) while moving a panel.
  **Cancel:** press `Esc` while moving.
- **Stack floating panels:** drag a panel's tab to the drop zone at the bottom of
  another panel; reorder by dragging tabs up/down. Release on the narrow zone
  between panels, not the broad title-bar zone.
- A dock need not fill its column — drag the dock's bottom edge up so it stops
  short of the workspace edge.
- **Reorder within a group:** drag a panel's tab left/right; **add to a group:**
  drop on the highlighted group border; **remove from a group:** drag the tab out.
- If all panels are removed from a dock, the dock disappears.

### Tabs, minimizing, resizing

- **Minimise/maximise** a panel, group or stack: double-click a tab (or the empty
  tab area). Double-clicking a tab minimises the group; a single click on a tab
  maximises it again.
- **Resize** a panel by dragging any side. Some panels, such as the Color panel,
  cannot be resized by dragging; the full exception set is unknown (see Open
  questions).
- **Collapse/expand a group:** click its tab; the tab toggles the group.
- **Close a panel:** right-click (`Ctrl`-click on Mac) its tab → `Close`, or
  deselect it in the `Window` menu. **Close a whole group:** panel menu →
  `Close Tab Group`. **Add a panel:** select it from the `Window` menu; reopening
  restores the group and position it had before it was closed.

### Collapsing to icons

- Panels can be **collapsed to icons** to reduce clutter. Collapse/expand all
  icons in a column with the **double arrow at the top of the dock** (top-right
  of the column). In some default workspaces the secondary column already ships
  collapsed to icons.
- Expand a single icon by clicking it. Resize the dock narrower until labels
  disappear to show icons only; widen to show labels again.
- Collapse an expanded panel back to its icon by clicking its tab, its icon, or
  the double arrow in the panel's title bar.
- `Interface > Auto-Collapse Iconic Panels` (CS6 label; the Help body writes
  "Auto-Collapse Icon Panels") collapses an expanded icon panel when the user
  clicks away. It can also be toggled by right-clicking a panel tab.
- Add a floating panel/group to an **icon dock** by dragging it in; panels added
  to an icon dock are automatically collapsed to icons.
- Move a panel icon (or icon group) by dragging the icon: up/down within a dock,
  into another dock (it adopts that dock's style), or outside (floating icon).

### Hiding panels

- `Tab` hides/shows **all** panels including the Tools panel and options bar;
  `Shift+Tab` hides/shows all panels **except** the Tools panel and options bar.
- With `Interface > Auto-Show Hidden Panels` selected, hovering the edge of the
  application window (Windows) or monitor (Mac) shows the hidden panels
  temporarily.
- `Window > Options` shows/hides the options bar.

### Workspace presets

A **workspace** is a named panel arrangement. CS6 ships predefined workspaces and
lets users create their own. In CS6 the **Application bar was removed**, so the
workspace switcher now lives at the right end of the **Options bar** (top-right
of the window, directly above the panel columns); document-layout options moved
to `Window > Arrange`.

| Workspace | Status in CS6 | Panel set |
|---|---|---|
| Essentials (Default) | Confirmed default | Main column: `Color\|Swatches`, `Adjustments\|Styles`, `Layers\|Channels\|Paths`; secondary icon column: `History`, `Properties` |
| New in CS6 | Confirmed | Marks new features/commands throughout the menus (highlight colour unverified) |
| 3D | Confirmed | 3D-oriented panels (Photoshop Extended) |
| Advanced 3D | Confirmed (Help PDF, `Window > Workspace > Advanced 3D`) | Extended 3D panel set |
| Motion | Confirmed | Video/animation (`Timeline`) |
| Painting | Confirmed | Replaces Color→`Navigator` and Adjustments/Styles→`Brush Presets`; adds `Brush`, `Clone Source`, `Tool Presets`; keeps `History`, `Layers\|Channels\|Paths` |
| Photography | Confirmed | Adds `Histogram`, `Info`, `Actions` to the general photo panels |
| Typography | Confirmed | Type panels (`Character`, `Paragraph`, `Character Styles`, `Paragraph Styles`) |
| Design | **Not in CS6** | Later CC workspace — no CS6 source lists it; CS6 sources name `Typography` instead |

`New in CS6` is a helper workspace that highlights the new CS6 commands in the
menus.

### Save, switch, delete, reset

- **Save:** `Window > Workspace > New Workspace…` (or `New Workspace` from the
  switcher); enter a name; optionally **Capture** `Keyboard Shortcuts` and
  `Menus` (both Photoshop). Saved names appear in both the workspace switcher
  and the `Window > Workspace` menu.
- **Switch:** choose a workspace from the switcher (or `Window > Workspace >`).
  The same document can be shown in any workspace. The switcher also carries
  `New Workspace`, `Delete Workspace` and `Reset [Workspace]` entries.
- **Delete:** `Window > Workspace > Delete Workspace…` (choose from the switcher)
  and confirm.
- **Reset one workspace:** `Window > Workspace > Reset [Workspace Name]`.
- **Restore all built-ins:** `Interface > Restore Default Workspaces`.
- **Reorder** workspaces by dragging them in the switcher.
- Workspaces remember panel positions as you last left them until reset —
  switching away and back does **not** restore the saved arrangement
  automatically; the explicit `Reset` does.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Panel | Dock widget | n/a | Tab, title bar, panel menu (upper-right) |
| Panel group | Dock widget | n/a | Multiple tabs, one title bar |
| Dock | Dock area | n/a | Vertical column; icon or expanded |
| Dock collapse toggle | Double arrow | n/a | Top of each column; collapse/expand all icons |
| Workspace switcher | Drop-down | n/a | Right end of the Options bar (top-right) in CS6 |
| `Window > Workspace` | Menu | n/a | New/Delete/Reset/Keyboard & Menus |
| `Window > [panel]` | Menu toggle | see `UI-002` | Show/hide a panel |
| Hide all panels | Shortcut | `Tab` | Includes tools + options bar |
| Hide all but tools/options | Shortcut | `Shift+Tab` | Auto-show hidden panels option |
| `Window > Options` | Menu toggle | n/a | Show/hide options bar |
| Prevent docking | Modifier (while dragging) | `Ctrl` / `Cmd` | Drag floats instead |
| Cancel panel move | Shortcut (while dragging) | `Esc` | Returns panel to previous spot |
| Theme brightness | Shortcut | `Shift+F1` / `Shift+F2` | Darken/lighten among 4 themes |
| Reset all workspaces | Preference button | n/a | Interface preferences |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Panel width | px (DIP) | theme | e.g. 200–480 | Per panel; persisted |
| Dock tab position | enum | Bottom `(inferred)` | Bottom / Top / Left / Right | Maps to `setTabPosition` |
| Dock nesting | bool | Off | on / off | `AllowNestedDocks` |
| Dock animation | bool | On | on / off | `AnimatedDocks` |
| Icon dock width | px (DIP) | icon size | `(to verify)` | Labels hide below threshold |
| Auto-Collapse Iconic Panels | bool | Off `(inferred)` | on / off | Interface preferences; also tab context menu |
| Auto-Show Hidden Panels | bool | Off `(inferred)` | on / off | Interface preferences |
| UI Font Size | enum | Small | Small / Medium / Large | `Interface > UI Font Size`; after relaunch |
| Enable Text Drop Shadows | bool | `(to verify)` | on / off | Interface preferences |
| Canvas colour | theme token | theme-linked | per screen mode | Standard / Full Screen variants |
| Workspace | enum | Essentials (Default) | built-in + custom | Captured per workspace |
| Capture keyboard shortcuts | bool | Off | on / off | New Workspace dialog |
| Capture menus | bool | Off | on / off | New Workspace dialog |
| Panel background | theme token | dark (default) | 4 brightness levels | `UI-001`; `Shift+F1`/`Shift+F2` |

## Algorithms & pipeline

Design proposal.

- **`QDockWidget` is the panel primitive.** Each panel is a `QDockWidget` with a
  custom title bar (`setTitleBarWidget`) so the title bar, tabs and collapse
  arrow match CS6. Grouping is achieved with `QMainWindow::tabifyDockWidget` and
  a shared tab bar; stacking/floating uses `Qt::Floating` `QDockWidget`s whose
  geometry is restored per workspace.
- **Dock model.** In CS6 a *dock* is a vertical column that stacks one or more
  panel *groups*; a group is a tabbed set. Model this as a `QMainWindow` dock
  area holding several tabified `QDockWidget`s, not as Qt nested docks.
- **Four dock areas + optional nesting.** Default `DockOptions`:
  `AnimatedDocks | AllowTabbedDocks`, and **not** `AllowNestedDocks` (keep drag
  semantics simple, per `ARCH-003`).
- **Drop-zone resolution.** On drag-move, compute nearest dock edge / inter-panel
  gap; honour the modifier that suppresses docking (`Ctrl`/`Cmd`) and `Esc`
  cancel. Use the *cursor* position, matching the documented CS6 behavior.
- **Icon collapse.** A dock can be in `Expanded` or `Icon` mode. Collapse all =
  iterate dock widgets and switch to icon mode (title bar only / vertical tab
  strip). Widen/narrow the dock to show/hide labels. `Auto-Collapse Iconic
  Panels` collapses on focus loss.
- **Workspace model.** A workspace is
  `{ name, layout: QByteArray (saveState), panelWidths, iconMode[], activeTabs[],
  capturedShortcutSet?, capturedMenuSet? }`. `saveState()` keys by
  `objectName`, so every dockable panel needs a stable, unique `objectName`.
- **Apply/reset.** `apply(workspace)` calls `restoreState` + panel widths + icon
  modes. `reset(workspace)` restores the *factory* layout stored at build time,
  because CS6 remembers the last-used arrangement between switches.
- **Hide-all.** `Tab`/`Shift+Tab` animate tray hiding of docks; with
  `Auto-Show Hidden Panels`, an edge hover re-shows them.

## Rust module mapping

Proposals.

- `pictura_ui::workspace::Workspace` — name, serialized layout, panel widths,
  icon modes, active tabs, optional captured shortcut/menu sets.
- `pictura_ui::workspace::Registry` — built-in and user workspaces; resolve
  duplicate names; ordering.
- `pictura_ui::workspace::persist` — save/restore against the session/preference
  store; versioned schema so layout persists across releases.
- `pictura_ui::docks::PanelId` — stable ids matching `objectName`s.
- `pictura_ui::docks::IconState` — expanded/icon per dock, animation policy.

Crossing types: `WorkspaceName`, `PanelId`, `QByteArray` layout blob opaque to
Rust (or an equivalent neutral layout description), widths as `f32` DIP. No
document data.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PicturaMainWindow` | `QMainWindow` | Owns dock areas; `saveState`/`restoreState`; `setDockOptions` |
| `PicturaDock` | `QDockWidget` | Panel host; custom `PanelHeader` title bar; `toggleViewAction` |
| `PanelHeader` | `QWidget` | Title, tab strip, panel-menu button, collapse arrow, drag hints |
| `WorkspaceSwitcher` | `QComboBox`/button+menu | Lists workspaces; New/Delete/Reset entries |
| `WorkspaceController` | `QObject` | Apply/reset/save/delete; persists session layout |
| `IconDockController` | `QObject` | Collapse/expand icon columns, label-width threshold, auto-collapse |

`saveState()`/`restoreState()` require every `QDockWidget` and toolbar to have a
unique `objectName` before the first restore; enforce this at registration.
Nested docks are disabled by default, so `saveState()` output is simpler and more
stable across Qt versions.

## Data-model impact

- Workspace/layout is **UI state**, stored in the preference/session store and
  optionally captured into a workspace; never in PSD/XMP.
- A workspace may capture **keyboard shortcut and menu customisation sets**
  (`UI-002`); those are preference records referenced by name.
- Restoring a workspace must not mutate the document model or add history
  states. Panel visibility toggles likewise produce no history.
- Panel content (Layers/Channels/Paths/History) is a **view over the document
  model**; the panel owns no document data (`ARCH-003`).

## Edge cases

- **Wayland.** Dock/floating geometry is compositor-controlled; restore relative
  layout only, never absolute positions.
- **Multi-monitor float.** A floated panel/group must be clamped to available
  geometry on restore; screen geometry is DIP with scaling gaps.
- **High-DPI change.** Icon assets and dock metrics rebuild when
  `devicePixelRatio` changes; persisted pixel widths should be stored as DIP.
- **Many panels.** Tabification plus deep nesting can exceed screen space; keep
  `AllowNestedDocks` off and provide overflow/scroll.
- **Duplicate `objectName`.** Breaks `restoreState` silently; registration must
  reject duplicates.
- **User workspace name collision** with a built-in (`Essentials`) must be
  prevented or namespaced.
- **Deleted/corrupt layout blob.** Fall back to Essentials and keep the user's
  custom workspaces intact.
- **Panel removed by a future version.** Restoring an old layout must ignore
  unknown panel ids without dropping the rest.
- **Screen mode interacts with docks** — entering Full Screen hides docks; the
  layout must survive the round trip.

## Parity acceptance criteria

- Given the default Essentials workspace, the main column contains
  `Color|Swatches`, `Adjustments|Styles` and `Layers|Channels|Paths`, and the
  secondary column contains `History` and `Properties` collapsed to icons.
- Given two panels in one dock, dragging a tab above/below another re-docks it
  with a blue drop zone snapped to the pointer; holding `Ctrl`/`Cmd` and dropping
  floats it; pressing `Esc` mid-drag cancels.
- Given a floating panel dragged to the bottom of another floating panel, they
  stack and move together as a unit; dragging one out un-stacks it.
- Given a dock, clicking the top double arrow collapses all its panels to icons
  and clicking again expands them; widening past the label threshold shows names.
- Given `Tab`, all panels including the Tools panel and options bar hide; given
  `Shift+Tab`, the Tools panel and options bar remain; with `Auto-Show Hidden
  Panels` on, hovering the screen edge reveals them.
- Given a rearranged layout, `Window > Workspace > New Workspace…` with Capture
  `Keyboard Shortcuts` saves it; selecting it later restores layout plus
  shortcuts; `Delete Workspace` removes it after confirmation.
- Given a modified built-in workspace, `Reset Essentials` restores the factory
  arrangement; `Restore Default Workspaces` restores every built-in.
- Given the `Window` menu, each panel's entry toggles its visibility;
  right-clicking its tab offers `Close`, and the panel menu offers
  `Close Tab Group`.
- Given a saved workspace and a restart at the same scale factor, the layout,
  panel widths, icon modes and active tabs restore within one pixel.
- Given `devicePixelRatio = 2`, restored widths and icons are crisp and correct.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help reference (downloaded and text-extracted).
  Establishes: panel/group/dock definitions; dock and undock by tab/title bar;
  drop zones and the pointer-activation detail; `Ctrl`/`Cmd` prevent-docking and
  `Esc` cancel; stacking floating panels; double-click to minimise/maximise;
  resizing (and the Color-panel exception); collapse-to-icons and the double
  arrow; `Auto-Collapse Icon Panels`; `Auto-Show Hidden Panels`; `Tab`/`Shift+Tab`
  hide-all behavior; the panel menu opens when minimised; add/remove panels via
  the `Window` menu and tab context menu; `Window > Options`; UI Font Size;
  workspace save/switch/delete/reset and `Restore Default Workspaces`;
  `Window > Workspace > New Workspace`, `Delete Workspace`,
  `Keyboard Shortcuts & Menus`, `New in CS6`, `Advanced 3D`; workspaces
  auto-remember the last arrangement until reset; reorder workspaces by dragging;
  the CS6 What's New note that "functions formerly in the application bar have
  moved elsewhere" and `Shift+F1`/`Shift+F2` brightness (body text also prints
  `Shift+1`/`Shift+2`). Primary.
- `https://hchsadobeacademy.weebly.com/uploads/7/1/6/2/7162106/p1_intro_photoshop_workspace.pdf`
  — official Adobe CS6 Project 1 guide (© 2012 Adobe Systems). Establishes the
  workspace switcher as part of the **Options bar** at the upper-right, the
  `Essentials (Default)` workspace, the New Workspace Capture options, and the
  default layout showing `Mini Bridge` and `Timeline` beneath the document
  window. Primary (Adobe-published).
- `https://www.photoshopforphotographers.com/pscs6/downloads/Photoshop-interface.pdf`
  — Martin Evening, *Adobe Photoshop CS6 for Photographers*, "The Photoshop CS6
  user interface" chapter extract. Establishes: the Application bar is gone and
  the workspace options are accessed via the Options bar; document layout moved
  to `Window > Arrange`; four interface themes (default dark); `Enable Text Drop
  Shadows`; UI font size; canvas colour tied to the theme. Primary companion.
- `https://www.photoshopforphotographers.com/3101-1901/Help_guide/tools_panels.html`
  — *Adobe Photoshop CS6 for Photographers* tool/panel index. Source for the
  CS6 panel inventory (Actions, Adjustments, Brush, Brush Presets, Channels,
  Character, Character Styles, Clone Source, Color, Histogram, History, Info,
  Layer Comps, Layers, Mini Bridge, Navigator, Notes, Paragraph, Paragraph
  Styles, Paths, Properties, Styles, Swatches, Tool Presets). Note: 3D,
  Animation and Measurement are declared out of scope by that guide.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/propertiespanel.html`
  — *Adobe Photoshop CS6 for Photographers*: the Properties panel is new in
  CS6 and absorbs the former Adjustments-controls mode and the CS5 Masks panel.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Notes_palette.html`
  — *Adobe Photoshop CS6 for Photographers*: the Notes panel is new in CS6.
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` —
  detailed CS6 tutorial. Corroborates the Essentials/Painting/Photography panel
  sets, the workspace switcher at the top-right directly above the main panel
  column, the two-column icon/expanded layout, `New Workspace`, and the
  reset/remember behavior. Secondary.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6/`
  — detailed CS6 panel tutorial. Source for panel groups, tab reordering,
  `Close` vs `Close Tab Group`, the per-column double arrow, icon expand/collapse,
  `Tab`/`Shift+Tab`, and the hover-to-reveal hidden panels behavior. Secondary.
- `https://www.chidrestechtutorials.com/graphics/photoshop/photoshop-workspaces.html`
  — CS6 tutorial listing `Essentials (default)`, `New in CS6`, `3D`, `Motion`,
  `Painting`, `Photography`, `Typography`, and placing the workspace drop-down at
  the R.H.S of the options bar. Secondary.
- `https://photoshoptrainingchannel.com/tips/auto-collapse-panels` and
  `https://andrewhaysom.myportfolio.com/auto-collapse-iconic-panels` — establish
  the CS6 label `Auto-Collapse Iconic Panels` and its tab context-menu toggle.
  Secondary.
- `https://andrewhaysom.myportfolio.com/new-and-improved-preset-panels-in-photoshop-cc-2020`
  — establishes that the Gradients, Patterns and Shapes panels were added in
  Photoshop 2020 (v21), i.e. not CS6. Secondary.
- `https://photoshopstar.com/use-libraries-panel-in-photoshop` — establishes the
  Libraries panel appeared with Photoshop CC 2014, i.e. not CS6. Secondary.
- `https://itwiki.wpunj.edu/images/e/ee/Photoshop_CS6_Extended_-_Dacier.pdf` —
  student tutorial; corroborates the palette dock / collapsed palette bar
  terminology. Secondary.
- Earlier draft also consulted `Adobe Photoshop CS6 on Demand` (search-result
  snippet) for the workspace-name set; it is now corroborated by the sources
  above.
- SearXNG meta-search queries were used to locate the sources above.

Not parsed: `helpx.adobe.com` (HTTP 403 from this environment); Wayback playback
for `web.archive.org` was intermittently unavailable.

## Open questions

- **Exact workspace-switcher order.** The workspace *set* is confirmed
  (`Essentials (Default)`, `New in CS6`, `3D`, `Advanced 3D`, `Motion`,
  `Painting`, `Photography`, `Typography`), but the on-screen ordering in the
  switcher has not been captured. Resolve with a switcher screenshot.
- **`New in CS6` highlight colour.** The Help only says new features are
  highlighted in the menus; the exact colour is unverified.
- **Mini Bridge / Timeline in default Essentials.** The official CS6 Project 1
  guide shows `Mini Bridge` and `Timeline` docked beneath the document window,
  while the detailed CS6 panel tutorial describes only the two right-hand
  columns. Confirm which reflects a clean first-run Essentials.
- **Precise dock metrics.** Icon dock width, label-hide threshold, tab height,
  title-bar height are unsourced. Resolve with pixel measurements from a CS6
  screenshot.
- **Nested docks in CS6.** CS6 docks are columns of stacked tab groups, not Qt
  nested docks; whether a group could be split inside one column beyond
  stacking is not fully verified. Our proposal disables `AllowNestedDocks`.
- **Layout persistence format.** CS6's workspace serialization is proprietary;
  we store our own versioned blob. Cross-version migration policy is open.
- **Panel resize exceptions.** The Help says "some panels, such as the Color
  panel, cannot be resized"; the full exception set is unknown. Resolve with a
  CS6 interaction pass.
- **Auto-Collapse / Auto-Show defaults.** Defaults are inferred (both off).
  Resolve with a first-run CS6 preferences capture.
- **Brightness shortcut discrepancy.** The CS6 Help prints both
  `Shift+F1`/`Shift+F2` (What's New) and `Shift+1`/`Shift+2` (workspace body);
  confirm which the shipping CS6 build uses on each platform.
