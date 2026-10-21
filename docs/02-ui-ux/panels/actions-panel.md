# Actions Panel

- **Spec ID:** `PAN-021`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Actions panel, action sets, recording/playback, and Button mode are in both CS6 editions. (The underlying action engine is `09-automation/actions.md`.)
- **New in CS6:** `Changed` — CS6 adds **Allow Tool Recording** (record brush/tool strokes directly; the CS6 Help lists it under "What's new in CS6 > Automation" and "Brushes"). **Conditional actions** are described in the CS6 Help but its links point to "Adding conditional actions | Creative Cloud", so their shipped-CS6 status is ambiguous (see Open questions and `AUTO-001`). The panel, sets, modal controls, and `.atn` files are otherwise CS5-equivalent.
- **Depends on:** `AUTO-001` `09-automation/actions.md` (canonical action semantics, `.atn` I/O, playback engine), `09-automation/droplets.md` (`AUTO-002`), `09-automation/batch-processing.md` (`AUTO-003`), `09-automation/script-events-and-jsx.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/qt6-ui-design.md` (`ARCH-003`). This panel spec is the **UI contract**; playback/`.atn` semantics are owned by `AUTO-001`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*.

## CS6 behavior

The **Actions panel** (`Window > Actions`, shortcut `F9` / `Option+F9`) records, plays, edits, and deletes actions. An **action** is a sequence of tasks — menu commands, panel options, tool actions, and the like — played back on a single file or a batch of files. Actions are grouped into **sets**; actions are *"the basis for droplets"* and drive **Batch** processing (`09-automation/`).

**Panel anatomy.** The tree has four levels, named in the CS6 figure:

- **A. Action set** — a named group of actions.
- **B. Action** — a named recorded sequence.
- **C. Recorded commands** — the steps inside an action.
- **D. Included command** — a step toggled on.
- **E. Modal control** — a step/action/set flagged to pause on playback.

Expand/collapse a set, action, or command with its triangle; `Alt`/`Option`-click the triangle expands/collapses all actions in a set or all commands in an action.

**Default placement.** The Actions panel is not part of the default Essentials workspace; it is one of the panels supplied by the **Photography** workspace (with Histogram and Info), docked in the right-hand column (`02-ui-ux/workspace-and-docks.md`, `UI-003`).

**Button mode.** `Button Mode` from the panel menu renders each action as a clickable button (with an optional assigned **Color**). *"You can't view individual commands or sets in Button mode."* Clicking a button executes the entire action; previously excluded commands are not executed. Choose `Button Mode` again to return to list mode.

**Selection.** Click an action name; `Shift`-click for contiguous multi-select; `Ctrl`/`Cmd`-click for discontiguous multi-select.

### Recording

Guidelines: most but not all commands can be recorded. Recordable tool operations include **Marquee, Move, Polygon, Lasso, Magic Wand, Crop, Slice, Magic Eraser, Gradient, Paint Bucket, Type, Shape, Notes, Eyedropper, and Color Sampler**, plus operations in the **History, Swatches, Color, Paths, Channels, Layers, Styles, and Actions** panels. Results depend on file/program state (active layer, foreground color, image resolution). Dialog/panel values in effect at record time are stored (with the warning that dialogs retain prior settings). Position-recording tools and modal operations use the current ruler units; the Help recommends percentage ruler units for actions played on different-sized files.

To record: open a file → click **Create New Action** (or **New Action** from the panel menu) → enter a name, choose an action set, and optionally set **Function Key** (F-key plus `Ctrl`/`Cmd`/`Shift`; not `F1`, and not `F4`/`F6` with `Ctrl` on Windows) and **Color** → **Begin Recording** (the button turns red) → perform operations → **Stop Playing/Recording** (or **Stop Recording**; `Esc` also stops). Recording can be resumed with **Start Recording**.

**Allow Tool Recording (CS6).** CS6 adds **Allow Tool Recording** to the Actions panel menu, letting Photoshop record tools such as the brush tool into an action. When on, brush strokes (otherwise often unique per project) are recorded directly; the Help recommends disabling it after recording such actions. This is the CS6 panel's one documented functional addition.

### Inserting stops and non-recordable commands

- **Insert Stop** — inserts a pause plus an optional message; `Allow Continue` adds a Continue button so the action can proceed without an intervening task.
- **Insert Menu Item** — inserts a non-recordable menu command (painting/toning tools, tool options, View commands, Window commands) without recording values; when played, the command runs and any dialog appears and pauses playback. A modal control for an Insert Menu Item command cannot be disabled.
- **Insert Path** — includes an existing Paths-panel path in an action; each Insert Path replaces the previous one unless a `Save Path` is recorded between them.

### Modal controls

A **modal control** pauses playback so values can be entered in a dialog or a modal tool can be used; it is drawn as a dialog-box icon. A **red** dialog-box icon means some but not all commands in that action/set are modal. Toggle modal for a command, for all commands in an action (box beside the action name), or for all actions in a set (box beside the set name). Modal controls cannot be set in Button mode.

### Excluding commands

Clear the check mark beside a single command to exclude it; click the parent check mark to include/exclude all commands or actions in an action/set; `Alt`/`Option`-click a command check mark to exclude/include all except it. The parent check mark turns red when some children are excluded.

### Playing

Playback runs the recorded commands in the active document. Options: select a set and Play; select an action and Play; press its assigned function key; select a command and Play to start there; `Ctrl`/`Cmd`-click Play (or `Ctrl`/`Cmd`-double-click a command) to play a single command. Modal actions pause for input. Photoshop has no built-in "undo action"; the Help's workaround is a History-panel snapshot taken before playback.

**Playback Options** (`Playback Options` from the panel menu):

- **Accelerated** — normal speed (default); the screen may not update (files can open/modify/save/close off-screen).
- **Step By Step** — completes each command and redraws before the next.
- **Pause For __ Seconds** — pauses a set number of seconds between commands.

### Editing and managing

- **Overwrite a single command** — double-click it, enter new values, OK.
- **Add commands** — select an action (append) or a command (insert after), click Begin Recording, record, stop.
- **Rearrange** — drag commands within/between actions; drag actions before/after other actions.
- **Record Again** — re-record an action, stepping through modal tools/dialogs.
- **Duplicate** — `Alt`/`Option`-drag, **Duplicate** from the menu, or drag onto the Create New Action button (sets too).
- **Delete** — select and click the Delete icon (`Alt`/`Option`-click deletes without confirmation; dragging to the Delete icon also skips confirmation) or **Delete** from the menu. **Clear All Actions** empties the panel.
- **Rename/options** — **Action Options** changes name, set, function key, and button color; in Photoshop an action name can also be renamed inline by double-clicking.

### Action sets and `.atn` files

- **New Set** — Create New Set button or **New Set**; name it. Create the set before creating an action to group it there.
- **Move** an action between sets by dragging.
- **Rename** a set via double-click or **Set Options**.
- **Save Actions** writes a set to a `.atn` file (Photoshop action-set files use the extension `.atn`); only a whole set can be saved, not an individual action. Placing the file in `Presets/Actions` makes it appear at the bottom of the panel menu after restart. `Ctrl+Alt`/`Cmd+Opt` at save time writes a text listing (not reloadable).
- **Load Actions** loads an additional set; a set at the bottom of the panel menu can also be chosen directly.
- **Replace Actions** replaces *all* sets in the current document (warn/back up first).
- **Reset Actions** restores the shipped default set, with OK = replace and Append = add to the current list.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Actions` | Menu → dock panel | `F9` / `Option+F9` | The panel |
| Panel menu | Menu | — | New Action/Set; Start/Stop/Record Again; Insert Stop/Path/Menu Item; **Allow Tool Recording (CS6)**; Playback Options; Button Mode; Save/Load/Replace/Reset Actions; Clear All; Duplicate/Delete; Action/Set Options |
| Panel bottom | Buttons | — | Stop Playing/Recording, Record, Play, Create New Set, Create New Action, Delete |
| Action/set/command row | Triangle | `Alt`/`Option`-click | Expand/collapse |
| Command row | Check box | — | Include/exclude; `Alt`/`Option` variant |
| Command/action/set row | Modal box | — | Dialog-box icon; red = partial |
| Action row (Button mode) | Button | — | Executes whole action; color-coded |
| `File > Automate > Batch` | Dialog | — | Runs an action on a folder; Override Action Open/Save As; Suppress dialogs |
| `File > Automate > Create Droplet` | Dialog | — | Saves an action as a drag-and-drop app |
| `File > Scripts > Image Processor` | Dialog | — | Has a Run Action option (set + action menus) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Action name | string | "Action N" | — | New Action dialog |
| Action set | enum/ref | current set | loaded sets | New Action dialog |
| Function key | key combo | none | F-key + `Ctrl`/`Cmd` + `Shift`; excludes `F1`, and `F4`/`F6` with `Ctrl` (Win) | Assigning an existing command shortcut makes the action win |
| Button color | enum | none | CS6 action color set | Button-mode display |
| Recording state | enum | idle | idle / recording / stopped | Record button turns red |
| Allow Tool Recording | bool | off | on/off | CS6 only; records brush/tool strokes directly; disable after use |
| Modal control | tri-state | off | off / on / partial (red) | Per command/action/set |
| Command included | bool | on | on/off | Check mark; parent red when partial |
| Playback speed | enum | Accelerated | Accelerated / Step By Step / Pause For N Seconds | Playback Options |
| Pause duration | seconds | 0 | ≥ 0 | Only when Pause is chosen |
| Action-set file | file | none | `.atn` | Save/Load/Replace; `Presets/Actions` for menu entry |
| Text dump | file | none | text | `Ctrl+Alt`/`Cmd+Opt` + Save (not reloadable) |
| Stop message | string | empty | — | Insert Stop; optional Allow Continue |
| Droplet destination | enum | Save and Close | folder / Save and Close / None | Create Droplet; see `09-automation/droplets.md` |

## Algorithms & pipeline

The panel is a **tree model/view over an action document**; the execution engine is `09-automation/actions.md`. Panel-level pipeline:

1. **Record** — while recording, every recordable action-manager command is appended to the current action as a command descriptor with its parameters (dialog values at record time, ruler-relative positions). Non-recordable commands are inserted as placeholders (Insert Menu Item/Stop/Path) that resolve at playback.
2. **Serialize the tree** — set → action → command with `recorded?`, `enabled?`, `modal?`, function key, and button color. This is the shape persisted to `.atn` (format owned by `09-automation/actions.md`).
3. **Play** — walk the selected subtree in order, skipping excluded commands; at each modal command pause for input; honour Playback Options (accelerated vs. step-by-step vs. pause). Single-command and start-from-command playback are subrange walks.
4. **Button mode** — flatten actions to buttons; the tree/modal/exclude controls are unavailable, and a click plays the whole action.
5. **Undo** — actions are *not* one history state in Photoshop; the Help's snapshot workaround is the documented behavior. The document command layer (`ARCH-009`) still records each command's normal undo state as it plays.

Modal-control evaluation, "some but not all" tri-state, and Insert Menu Item deferral are the only panel-specific logic beyond the tree.

## Rust module mapping

The **action engine belongs to `09-automation/actions.md`**; the panel maps onto it:

- `pictura_script::action::ActionSet` / `Action` / `ActionCommand` — tree model (`{ name, enabled, modal, commands }`; command carries an op id + parameter block).
- `pictura_script::action::ActionLibrary` — loaded sets, `create/rename/delete/duplicate/reorder`, `save_atn/load_atn/replace/reset` (format owned by `09-automation/actions.md`).
- `pictura_script::action::Recorder` — record/stop; maps executed commands to `ActionCommand` descriptors.
- `pictura_script::action::Player` — playback with `PlaybackSpeed { Accelerated, StepByStep, Pause(Duration) }`, start index, single-command mode, and modal callbacks.
- `pictura_script::action::ModalState` — `Off | On | Partial` per node.
- `pictura_ui_bridge::ActionBarState` — record/play/stop state surfaced to the toolbar/status.
- Droplet/batch wiring lives in `09-automation/droplets.md` and `09-automation/batch-processing.md`.

Crossing types: `ActionId`, `ActionSetId`, `CommandId`, `ModalState`, `PlaybackSpeed`, and a callback trait for modal input. No Qt types cross into `pictura_script`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ActionsPanel` | `QDockWidget` | Host; header/menu, bottom buttons, status |
| `ActionsTreeModel` | `QAbstractItemModel` | Set/action/command tree; roles for enabled, modal, type, color |
| `ActionsTreeView` | `QTreeView` | Tree rendering; `uniformRowHeights`; drag-reorder; tri-state modal column |
| `ActionButtonMode` | `QListView` icon mode / `QToolButton` grid | Button-mode rendering with per-action color |
| `ActionsToolBar` | `QToolBar` | Record/Play/Stop and New Set/New Action/Delete buttons |
| `PlaybackOptionsDialog` | `QDialog` | Accelerated / Step By Step / Pause |
| `ActionOptionsDialog` | `QDialog` | Name, set, function key, color |
| `InsertStopDialog` | `QDialog` | Message + Allow Continue |
| `ModalInputBridge` | `QObject` / signal | Raises the per-command dialog/modal tool during playback |

Widgets over QML for the dense tree, tri-state icons, drag-reorder, and dialog raising (`ARCH-003`). The view never executes commands; it emits intents to `pictura_script` and repaints from model roles. Button mode is the same model in a different view.

## Data-model impact

- **No PSD/XMP document fields.** Actions are app-level libraries, not document data. A played action produces ordinary document commands with their own history states (`ARCH-009`).
- **`.atn` (and optional text dump)** are external libraries; the binary layout is owned by `09-automation/actions.md`, not by this panel spec.
- **Undo.** Recording/editing/renaming actions is not document undo. Playback adds per-command history states, matching Photoshop; there is no single "undo action" state (the Help's snapshot workaround). Whether the project adds a convenience grouping is a product decision *(inferred)*.
- **Preferences/presets.** Default sets ship in `Presets/Actions`; loaded sets and button colors are app state. A preferences reset restores the default set.
- **Function-key bindings** belong to the keyboard map; they must not silently override menu shortcuts without the documented action-wins rule.

## Edge cases

- **Button mode limits.** No tree, no per-command modal/exclude control; clicking executes the whole action with excluded commands skipped.
- **Insert Menu Item modal control** cannot be disabled; the dialog always appears at playback.
- **Recorded filename** in `Save As` is replayed literally; the Help warns not to rename during recording. Provide the documented folder-only-change escape.
- **Different file sizes** — actions record absolute units unless ruler units are percentages; the panel should make the ruler-unit caveat visible (it is a document/Preferences concern).
- **Non-recordable tool/View/Window commands** — must route through Insert Menu Item, not be silently dropped.
- **Clear/Replace Actions** — destructive to the current panel state; confirm and recommend Save Actions first.
- **`.atn` parse failure** — load what parsed and report the failure; never crash or silently drop the library.
- **Missing command target** (e.g. an action step referencing an unloaded plugin/preset) — report at playback, continue or abort per user choice.
- **Playback on a wrong-mode document** (e.g. Color Balance on Grayscale) — command-level error handling (`09-automation/actions.md`); the panel surfaces the failure and stops.
- **Huge action sets** — virtualize the tree (`uniformRowHeights`); do not expand all by default.
- **Modal playback and no GUI/headless** — documented headless behavior belongs to `09-automation/`; the panel is unavailable when no GUI exists.
- **Concurrent playback** — only one action may run at a time; the panel disables record/play controls during playback.

## Parity acceptance criteria

1. Given a new action set and action, recording a Gaussian Blur and a Save step adds the commands under the action with dialog values captured.
2. Given a recorded action, `Play` re-runs its included commands on the active document in order.
3. Given a modal control on a command, playback pauses there and the command's dialog appears; disabling it runs without pausing.
4. Given **Button mode**, individual commands/sets are not shown, clicking an action button runs the whole action, and excluded commands are skipped.
5. Given **Step By Step**, the image redraws between commands; given **Accelerated**, it need not.
6. Given a command check mark cleared, that command is excluded; clearing the action name excludes the whole action; the parent check mark shows red for partial exclusion.
7. Given an action with a Function Key, pressing it plays the action and takes precedence over a command using the same shortcut.
8. Given `Insert Stop` with a message and **Allow Continue**, playback pauses with the message and can continue without an intervening task.
9. Given `Save Actions` to `Presets/Actions`, restarting shows the set at the bottom of the panel menu; `Load Actions` appends it, `Replace Actions` replaces all sets, and `Reset Actions` restores defaults (OK replace / Append add).
10. Given `Clear All Actions`, the panel empties and can be restored to the default set.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: `Window > Actions`; `F9`/`Option+F9`; the definitions of action/action set/droplet; the A–E panel anatomy (set, action, recorded commands, included command, modal control); expand/collapse and `Alt`-click; Button mode and its restrictions; recording guidelines and the recordable tool/panel list; ruler-unit/position caveat; Create New Action and Function Key exceptions (`F1`, `F4`/`F6`+`Ctrl` on Windows); Begin/Stop/Start Recording and `Esc`; **Allow Tool Recording** as a CS6 addition (with the "disable after use" note); Insert Stop/Allow Continue; Insert Path/multiple-path note; Insert Menu Item and the non-recordable list; modal controls and the red partial icon; exclude/include and the red parent check mark; Play flows (set/action/function key/part/single command); Playback Options (Accelerated/Step By Step/Pause For N Seconds); overwrite/add/rearrange/Record Again; duplicate/delete/Clear All; Action/Set Options; sets (new/move/rename/Save/Load/Replace/Reset) with `.atn` extension and `Presets/Actions` menu behavior; the `Ctrl+Alt`/`Cmd+Opt` text dump; Batch and droplet relationships; Image Processor `Run Action` set/action menus; the ambiguous "Conditional actions" section linking to Creative Cloud.
- `https://searxng` query "Photoshop CS6 Actions panel record play stop modal control button mode" and "Photoshop CS6 Character Styles Paragraph Styles panel Type menu" — discovery snippets locating secondary pages; not used as assertions.
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Essentials vs. Photography workspace panel sets; places Actions (with Histogram and Info) in the Photography workspace, not in default Essentials.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- Adobe's current-version Actions help pages surfaced by the above queries; later-version UI, not asserted as CS6.

Not used in this pass:

- `helpx.adobe.com` Actions pages (HTTP 403 / current-version only).

## Open questions

- **Conditional actions in shipped CS6.** The fetched CS6 Help text contains a "Conditional actions" section, but its heading and cross-reference read "Adding conditional actions | **Creative Cloud**", and the described path is `Insert Conditional` on the Actions panel menu — a CC-era control. The CS6-era conditional feature documented in the same corpus is the narrower `File > Automate > Conditional Mode Change` (a recorded color-mode guard step). `AUTO-001` asserts conditional actions are a CS6 change; this panel spec does not. *Resolves with:* a shipped-CS6 build test and reconciliation with `AUTO-001`.
- **Stepwise vs. batch conditional steps.** If conditional actions are out of CS6 scope, the panel's command tree has no conditional node, and `09-automation/actions.md` must state this.
- **`.atn` binary format and version.** Not documented by the fetched text (owned by `09-automation/actions.md`). *Resolves with:* the file-format/SDK reference and CS6-saved `.atn` files.
- **Exact default action sets and names** shipped with CS6 are not enumerated in the fetched text. *Resolves with:* a CS6 install listing.
- **Function-key conflict resolution** beyond the documented "action wins" rule (e.g. against tool shortcuts) is unverified. *Resolves with:* a CS6 test.
- **Mapping of recorded commands to the substitute script API** (`09-automation/rust-scripting-replacement.md`) — whether the substitute can record/play arbitrary operations or only a documented subset. *Resolves with:* the automation architecture decision.
- **Single history-state grouping of a played action** is not a CS6 behavior; whether Kooka Pictura adds it is a product decision (must not be presented as parity).
