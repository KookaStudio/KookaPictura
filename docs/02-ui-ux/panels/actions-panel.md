# Actions Panel

- **Spec ID:** `PAN-021`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Actions panel, action sets, recording/playback, and Button mode are in both CS6 editions. (The underlying action engine is `09-automation/actions.md`.)
- **New in CS6:** `Changed` — CS6 adds **Allow Tool Recording** (record brush/tool strokes directly); this release records Brush/Pencil strokes and refuses native tool-specific and mask strokes until their replay paths land. **Insert Conditional is not a CS6 feature**: Adobe documents conditional actions for Creative Cloud; this project ships it as a Kooka extension with a documented condition subset (see *Inserting stops and non-recordable commands*). The panel, sets, modal controls, and `.atn` files are otherwise CS5-equivalent.
- **Depends on:** `AUTO-001` `09-automation/actions.md` (canonical action semantics, `.atn` I/O, playback engine), `09-automation/droplets.md` (`AUTO-002`), `09-automation/batch-processing.md` (`AUTO-003`), `09-automation/script-events-and-jsx.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/qt6-ui-design.md` (`ARCH-003`). This panel spec is the **UI contract**; playback/`.atn` semantics are owned by `AUTO-001`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*.

## CS6 behavior

The **Actions panel** (`Window > Panels > Actions`, shortcut `F9`) records, plays, edits, and deletes actions. An **action** is a sequence of tasks — menu commands, panel options, tool actions, and the like — played back on a single file or a batch of files. Actions are grouped into **sets**; actions are *"the basis for droplets"* and drive **Batch** processing (`09-automation/`).

**Host.** The panel is a plain `QWidget` content panel with the stable object name `actionsPanel`, hosted by a `PanelGroup` in a `PanelColumn`. It is **not** a `QDockWidget`, and the same widget is presented in the docked group, the iconic flyout, and the floating overlay with no host-specific behavior. The panel fits the shared normal-mode minimum width and the 300 px flyout.

**Panel anatomy.** The tree has three levels:

- **A. Action set** — a named group of actions.
- **B. Action** — a named recorded sequence.
- **C. Recorded step** — a command, stop, menu item, path, or conditional inside an action.

Each row also carries two state columns in this order: an **inclusion check** and a **modal control**. A step row shows its name and whether it is a recorded command or an inserted Stop, Menu Item, Path, or Conditional step; expanding a step shows its recorded parameters when it carries them. The tree uses `uniformRowHeights`, does not expand every node by default, and keeps expansion state across save and reload. Expand/collapse a set, action, or step with its disclosure triangle; `Alt`/`Option`-click the triangle expands/collapses all actions in a set or all steps in an action.

**Default placement.** The panel is registered in its own group and is not visible in the default Essentials workspace; it is reachable from `Window > Panels > Actions` (and is part of the Photography workspace).

**Button mode.** `Button Mode` from the panel menu renders each action as a clickable button (with an optional assigned **Color**). *"You can't view individual commands or sets in Button mode."* Buttons follow library order: sets in order, then actions within each set in order. Clicking a button executes the entire action; previously excluded steps are not executed. Modal controls and inclusion checks are not editable in Button mode. Choose `Button Mode` again to return to the tree with expansion, selection, check, and modal state unchanged.

**Selection.** Click an action name; `Shift`-click for contiguous multi-select; `Ctrl`/`Cmd`-click for discontiguous multi-select.

### Recording

Guidelines: most but not all commands can be recorded. Recordable tool operations include **Marquee, Move, Polygon, Lasso, Magic Wand, Crop, Slice, Magic Eraser, Gradient, Paint Bucket, Type, Shape, Notes, Eyedropper, and Color Sampler**, plus operations in the **History, Swatches, Color, Paths, Channels, Layers, Styles, and Actions** panels. Results depend on file/program state (active layer, foreground color, image resolution). Dialog/panel values in effect at record time are stored (with the warning that dialogs retain prior settings). Position-recording tools and modal operations use the current ruler units; the Help recommends percentage ruler units for actions played on different-sized files.

To record: open a file → click **Create New Action** (or **New Action** from the panel menu) → enter a name, choose a target action set, and optionally set **Function Key** (`F1`–`F12` with optional `Ctrl`/`Cmd` and `Shift`; Windows refuses `F1`, `Ctrl+F4` and `Ctrl+F6`) and **Color** → **Begin Recording** (the button turns red) → perform operations → **Stop Playing/Recording** (or **Stop Recording**; `Esc` also stops). Recording can be resumed with **Start Recording**; recording and playback are mutually exclusive. Recording starts at the selected action or step insertion point, or appends to the last action of a selected non-empty set; an empty or absent selection is refused.

**Allow Tool Recording (CS6).** CS6 adds **Allow Tool Recording** to the Actions panel menu, letting Photoshop record tools such as the brush tool into an action. It is a checkable panel option and is **off by default**. When on, a completed Brush or Pencil stroke appends one app-native tool-stroke step with a bounded, versioned payload; when off, no tool step is appended. Native tool-specific strokes and mask strokes are refused with a named diagnostic until their replay paths land. An over-limit stroke is refused with a named diagnostic rather than truncated. The encoding is a Kooka extension and is not Adobe-interoperable.

### Inserting stops and non-recordable commands

- **Insert Stop** — inserts a pause plus an optional message; `Allow Continue` adds a Continue button so the action can proceed without an intervening task.
- **Insert Menu Item** — inserts a non-recordable menu command (painting/toning tools, tool options, View commands, Window commands) chosen from a command-picker modal dialog, without recording parameter values. It is a *specific* step: its modal control is shown off and cannot be enabled, but playback still opens the command's dialog when it has one.
- **Insert Path** — inserts the selected Paths-panel path as an app-native step; playback sets the work path to it. Each Insert Path replaces the previous one unless a `Save Path` is recorded between them.
- **Insert Conditional** — a **Kooka extension**, not a CS6 feature: `If Current <condition>` → `Then Play Action` / `Else Play Action`. This release evaluates document is square, document mode, and document has a selection on the active document. At least one branch must be chosen, and both referenced actions must belong to the action's own set.

### Modal controls

A **modal control** pauses playback so values can be entered in a dialog or a modal tool can be used; it is drawn as a dialog-box icon. A **red** dialog-box icon means some but not all commands in that action/set are modal. Toggle modal for a step, for all steps in an action (box beside the action name), or for all actions in a set (box beside the set name). Parent modal state is derived from the displayed children, not stored separately. Modal controls cannot be set in Button mode.

The step's **dialog options** decide whether the control is editable:

| Dialog options | Control | Playback |
|---|---|---|
| `0` optional | editable; follows `withDialog` | dialog opens only when `withDialog` is on |
| `1` required | on and not editable | dialog always opens |
| `2` none | off and not editable | dialog never opens |
| `3` specific | off and not editable | dialog still opens (Insert Menu Item and similar) |
| `4` stop/continue | in-memory recording value | written to `.atn` as `0` |

### Excluding commands

Clear the check mark beside a single command to exclude it; click the parent check mark to include/exclude all commands or actions in an action/set; `Alt`/`Option`-click a command check mark to exclude/include all except it. The parent check mark turns red when some children are excluded.

### Playing

Playback runs the recorded commands in the active document. Options: select a set and Play (an explicit **app decision**, not documented CS6 behavior: its included actions play in library order); select an action and Play; press its assigned function key (`F1`–`F12` with optional `Shift` and `Ctrl`/`Cmd`); select a command and Play to start there; `Ctrl`/`Cmd`-click Play (or `Ctrl`/`Cmd`-double-click a command) to play a single command. Modal steps pause for input; a *specific* step still opens its dialog; Stop messages appear with Continue; cancellation is available. Only one playback runs at a time and recording is refused during playback. Playback stops on the first error with a named diagnostic; this release has no continue-after-error path. Photoshop has no built-in "undo action"; the Help's workaround is a History-panel snapshot taken before playback. Playing an empty action or set is a no-op success.

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
- **Rename/options** — **Action Options** changes name, **target set**, function key (`F1`–`F12` with optional `Shift` and `Ctrl`/`Cmd`), and button color; in Photoshop an action name can also be renamed inline by double-clicking. Assigning a combination already owned by another action is refused; shadowing a command shortcut warns but is allowed, and the action wins. On Windows `F1`, `Ctrl+F4` and `Ctrl+F6` are refused. Clearing the key or deleting the action restores the command's shortcut. The Keyboard Shortcuts editor does not list action keys.

### Action sets and `.atn` files

- **New Set** — Create New Set button or **New Set**; name it. Create the set before creating an action to group it there.
- **Move** an action between sets by dragging.
- **Rename** a set via double-click or **Set Options**.
- **Save Actions** writes a whole set to a `.atn` file (one set per file); it is enabled only when a set is selected, and it is disabled while recording or playing. Placing the file in `Presets/Actions` makes it appear at the bottom of the panel menu after restart. `Ctrl+Alt`/`Cmd+Opt` at save time writes a human-readable Kooka text listing that is deliberately **not reloadable**; its Adobe layout is undocumented and it is not claimed as Adobe-compatible.
- **Load Actions** loads an additional set; a set at the bottom of the panel menu can also be chosen directly. Replace confirms destructive replacement; Load appends without a confirmation.
- **Replace Actions** replaces *all* sets in the current library (warn/back up first).
- **Reset Actions** restores the compiled-in `Default Actions` set — Duplicate Layer, Flatten Image, Grayscale, and Resize to 1000 px (Image Size width 1000 px, constrain proportions, Bicubic), each with a distinct colour — with OK = replace and Append = add under a unique name. **No Adobe preset actions ship**, and the default set is not discovered as an installed `.atn`.

**Persistence.** The library is stored as `Actions Palette.psp` in the application state directory, using the exact version-16 palette codec (set count then sets, no magic). Writes are serialized and atomic through `QSaveFile`, happen on a debounced change and at shutdown (not only at quit), and a failed write leaves the previous palette intact. The panel's Button mode, expanded set/action/step identities, selection, Allow Tool Recording, and Playback Options are stored as additive keys under session schema 10 by set/action name path, ordinal among same-named siblings, and step index; a missing name drops that entry. Missing keys load defaults and unknown keys are preserved.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Panels > Actions` | Menu → content panel | `F9` | The panel, hosted by a `PanelGroup` (not a `QDockWidget`) |
| Panel menu | Menu | — | `Button Mode`; separator; New Action…, New Set…, Duplicate, Delete, Play; separator; Start Recording (Stop Recording while recording), Record Again…; separator; Insert Menu Item…, Insert Stop…, Insert Conditional… (Kooka extension), Insert Path; separator; Action Options…/Set Options…, Playback Options…, Allow Tool Recording; separator; Clear All Actions, Reset Actions, Load Actions…, Replace Actions…, Save Actions…; separator; installed action sets; separator; Close, Close Tab Group |
| Panel bottom | Buttons | — | Stop, Record, Play, Create New Set, Create New Action, Delete |
| Action/set/step row | Triangle | `Alt`/`Option`-click | Expand/collapse; `Alt` expands/collapses the subtree |
| Step row | Check box | — | Include/exclude; `Alt`/`Option` variant excludes/includes all siblings except the clicked step |
| Step/action/set row | Modal box | — | Dialog-box icon; red = partial; derived from children |
| Action row (Button mode) | Button | — | Executes whole action; color-coded |
| `File > Automate > Batch` | Dialog | — | Runs an action on a folder; Override Action Open/Save As; Suppress dialogs |
| `File > Automate > Create Droplet` | Dialog | — | Saves an action as a drag-and-drop app |
| `File > Scripts > Image Processor` | Dialog | — | Has a Run Action option (set + action menus) |

**Menu enablement.** Button Mode, New Set, Playback Options, Allow Tool Recording, Load, Replace, Reset, and the close rows are enabled while idle. New Action needs a selected set or a non-empty library; Duplicate and Delete need any selection; Play needs an action, step, or set selection; Start Recording needs an action, step, or non-empty set; Record Again needs an action; Clear All needs a non-empty library. Insert Menu Item, Insert Stop, and Insert Conditional need an action or step; Insert Path also needs a selected path. Action Options needs an action, Set Options needs a set, and Save Actions needs a set. Load, Replace, Reset, and Save are disabled while recording or playing; editing rows are disabled while recording or playing except Stop Recording; Play is disabled while recording and Start Recording is disabled while playing.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Action name | string | "Action N" | — | New Action / Action Options dialog |
| Action set | enum/ref | current set | loaded sets | New Action and **Action Options** dialogs; Action Options can move the action |
| Function key | key combo | none | `F1`–`F12` + optional `Shift` + `Ctrl`/`Cmd`; Windows refuses `F1`, `Ctrl+F4`, `Ctrl+F6` | A duplicate action combination is refused; a command collision warns, not blocks, and the action wins |
| Button color | enum | none | None, Red, Orange, Yellow, Green, Blue, Violet, Gray (indices `0`–`7`) | Button-mode display |
| Recording state | enum | idle | idle / recording / stopped | Record button turns red |
| Allow Tool Recording | bool | off | on/off | CS6; one app-native step per completed supported stroke; bounded payload |
| Modal control | tri-state | off | off / on / partial (red) | Per step/action/set; derived for parents |
| Command included | bool | on | on/off | Check mark; parent red when partial |
| Playback speed | enum | Accelerated | Accelerated / Step By Step / Pause For N Seconds | Playback Options; persisted in the session |
| Pause duration | seconds | 0 | ≥ 0 | Only when Pause is chosen |
| Action-set file | file | none | `.atn` | Save/Load/Replace; one set per file; `Presets/Actions` for menu entry |
| Palette file | file | none | `Actions Palette.psp` | Version-16 palette; atomic debounced and shutdown writes |
| Text dump | file | none | text | `Ctrl+Alt`/`Cmd+Opt` + Save; Kooka format, not reloadable |
| Stop message | string | empty | — | Insert Stop; optional Allow Continue |
| Conditional condition | enum | — | document is square / document mode / document has a selection | Kooka extension; evaluated on the active document |
| Droplet destination | enum | Save and Close | folder / Save and Close / None | Create Droplet; see `09-automation/droplets.md` |

## Algorithms & pipeline

The panel is a **tree model/view over an action document**; the execution engine is `09-automation/actions.md`. Panel-level pipeline:

1. **Record** — while recording, every recordable command that reports to the command bus is appended to the current action as a step with its parameters (dialog values at record time, ruler-relative positions). Non-recordable commands are inserted as steps (Insert Menu Item/Stop/Path/Conditional) that resolve at playback; Brush/Pencil strokes append only under Allow Tool Recording.
2. **Serialize the tree** — set → action → step with `expanded`, `enabled`, `withDialog`, dialog options, function key, and button color. This is the shape persisted to version-16 `.atn` (format owned by `09-automation/actions.md`).
3. **Play** — walk the selected action or set in order, skipping excluded steps; at each modal step pause for input, and always open a *specific* step's dialog; honour Playback Options (accelerated vs. step-by-step vs. pause). Single-step and start-from-step playback are subrange walks.
4. **Button mode** — flatten actions to buttons in library order; the tree/modal/exclude controls are unavailable, and a click plays the whole action.
5. **Undo** — actions are *not* one history state in Photoshop; the Help's snapshot workaround is the documented behavior. The document command layer (`ARCH-009`) records each command's normal undo state as it plays; this change adds no per-action grouping.

Modal-control evaluation, the derived "some but not all" tri-state, the specific-step deferral, and the Kooka conditional branch are the panel-specific logic beyond the tree.

## Rust module mapping

The **action engine belongs to `09-automation/actions.md`**; the panel maps onto it:

- `pictura-script::action` — `ActionSet` / `Action` / `ActionStep`, `FunctionKey`, `ActionColor`, `DialogOptions`, `StepFlags`, `ModalState`; the tree row model is a C++ mirror of one engine snapshot.
- `pictura-script::library` — loaded sets, `create/rename/delete/duplicate/reorder`, `save/load/replace/reset/clear`, `Default Actions` (format owned by `09-automation/actions.md`).
- `pictura-script::recorder` — record/stop; appends command steps, or one tool-stroke step when Allow Tool Recording is on.
- `pictura-script::player` — playback with speed, pause, start index/step, single-step mode, modal handling, cancellation, and the 64-depth limit.
- `pictura-script::atn` — converts the `pictura-codec::action_file` raw model to and from the owned actions.
- `pictura-app::actions_engine` — the cxx-qt bridge: snapshot, CRUD, recording/playback, persistence, and function-key/playback-option operations.
- Droplet/batch wiring lives in `09-automation/droplets.md` and `09-automation/batch-processing.md`.

Crossing types: stable action/step node ids, `CommandId`, `ModalState`, playback speed, and row snapshots. No Qt types cross into `pictura-script`.

## Qt6 component mapping

| Component | Base | Responsibility |
|---|---|---|
| `ActionsPanel` | `QWidget` (`actionsPanel`) | Content panel: tree or Button view, bottom toolbar, panel-menu hook; used unchanged by docked, flyout, and floating hosts |
| `ActionsTreeModel` | `QAbstractItemModel` | Set/action/step tree with stable node ids; check and modal columns; roles for enabled, modal, type, color |
| `ActionsTreeView` | `QTreeView` | Tree rendering; `uniformRowHeights`; not expanded by default; drag-reorder; tri-state modal column |
| `ActionsButtonView` | `QListView` icon mode | Button-mode rendering in library order with per-action color |
| `ActionsToolBar` | `QWidget` / buttons | Stop, Record, Play, New Set, New Action, Delete in that order |
| `PlaybackOptionsDialog` | `QDialog` | Accelerated / Step By Step / Pause |
| `ActionOptionsDialog` | `QDialog` | Name, target set, function key, modifiers, color |
| `SetOptionsDialog` | `QDialog` | Set name |
| `InsertStopDialog` | `QDialog` | Message + Allow Continue |
| `InsertConditionalDialog` | `QDialog` | Condition, Then/Else actions from the same set |
| `ModalInputBridge` | `QObject` / signal | Raises the per-step dialog/modal tool during playback |

Widgets over QML for the dense tree, tri-state icons, drag-reorder, and dialog raising (`ARCH-003`). The view never executes commands; it emits intents through the engine bridge and repaints from model roles. Button mode is the same engine data in a different view.

## Data-model impact

- **No PSD/XMP document fields.** Actions are app-level libraries, not document data. A played action produces ordinary document commands with their own history states (`ARCH-009`).
- **`.atn` (and the optional text dump)** are external libraries; the binary layout is owned by `09-automation/actions.md`, not by this panel spec. The library itself persists as `Actions Palette.psp`, not in the document or session JSON.
- **Undo.** Recording/editing/renaming actions is not document undo. Playback adds per-command history states, matching Photoshop; there is no single "undo action" state (the Help's snapshot workaround). This change does **not** add a convenience grouping.
- **Preferences/presets.** No Adobe default set ships and no default `.atn` is written to `Presets/Actions`; `Default Actions` is compiled in and Reset restores it. Loaded sets and button colors are app state.
- **Function-key bindings** are action bindings, not editor bindings. The action wins over a colliding command at dispatch, the command's stored binding is unchanged, and clearing/removing the action restores it; the Keyboard Shortcuts editor does not list action keys.
- **Panel state** (Button mode, expansion, selection, Allow Tool Recording, Playback Options) is stored as additive session-schema-10 keys by name path, sibling ordinal, and step index; missing keys load defaults and unknown keys are preserved.

## Edge cases

- **Button mode limits.** No tree, no per-command modal/exclude control; clicking executes the whole action with excluded commands skipped.
- **Insert Menu Item modal control** cannot be disabled; the dialog always appears at playback.
- **Recorded filename** in `Save As` is replayed literally; the Help warns not to rename during recording. Provide the documented folder-only-change escape.
- **Different file sizes** — actions record absolute units unless ruler units are percentages; the panel should make the ruler-unit caveat visible (it is a document/Preferences concern).
- **Non-recordable tool/View/Window commands** — must route through Insert Menu Item, not be silently dropped.
- **Clear/Replace Actions** — destructive to the current panel state; confirm and recommend Save Actions first.
- **`.atn` parse failure** — load what parsed and report the failure; never crash or silently drop the library.
- **Missing command target** (e.g. an action step referencing an unloaded plugin/preset) — playback stops on the first error and names the step and target; this release has no continue-after-error choice.
- **Playback on a wrong-mode document** (e.g. Color Balance on Grayscale) — command-level error handling (`09-automation/actions.md`); the panel surfaces the named failure and stops.
- **Insert Conditional** — Kooka extension; both branches `None` is refused, actions must belong to the action's own set, and a referenced action cannot be moved or deleted while the conditional needs it. Conditions are evaluated on the active document.
- **Insert Path and tool strokes** — app-native namespaced `TEXT` steps, not Adobe-interoperable; a tool stroke over the payload bound is refused rather than truncated.
- **`.atn`/`.psp` version or shape** — a version other than 16 fails with a named diagnostic; unknown event identifiers are preserved and reported at playback; unknown descriptor types fail closed. A partially valid palette loads the parsed sets and reports the offset; when nothing parses, the library is unchanged.
- **Huge action sets** — virtualize the tree (`uniformRowHeights`); do not expand all by default.
- **Modal playback and no GUI/headless** — documented headless behavior belongs to `09-automation/`; the panel is unavailable when no GUI exists.
- **Concurrent playback** — only one action may run at a time; the panel disables record/play controls during playback and refuses recording during playback.

## Parity acceptance criteria

1. Given a new action set and action, recording a Gaussian Blur and a Save step adds the steps under the action with dialog values captured.
2. Given a recorded action, `Play` re-runs its included steps on the active document in order.
3. Given a modal control on a step, playback pauses there and the step's dialog appears; disabling an optional control runs without pausing, while required and specific controls still open theirs.
4. Given **Button mode**, individual steps/sets are not shown, clicking an action button runs the whole action, and excluded steps are skipped.
5. Given **Step By Step**, the image redraws between steps; given **Accelerated**, it need not.
6. Given a step check mark cleared, that step is excluded; clearing the action name excludes the whole action; the parent check mark shows red for partial exclusion.
7. Given an action with a Function Key, pressing it plays the action and takes precedence over a command using the same shortcut; a duplicate action key is refused and a command collision warns.
8. Given `Insert Stop` with a message and **Allow Continue**, playback pauses with the message and can continue without an intervening task.
9. Given `Save Actions` to `Presets/Actions`, restarting shows the set at the bottom of the panel menu; `Load Actions` appends it, `Replace Actions` replaces all sets, and `Reset Actions` restores the compiled-in `Default Actions` (OK replace / Append add under a unique name).
10. Given `Clear All Actions`, the panel empties and can be restored to the default set.
11. Given `Insert Conditional` with both branches `None` or an action from another set, the dialog refuses the step.
12. Given a set selected and Play used, its included actions play in library order (app decision), and a second start is refused while playback runs.
13. Given a `.psp` write failure, the previous palette remains intact and the library still loads on the next launch.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: `Window > Panels > Actions`; `F9`; the definitions of action/action set/droplet; the panel anatomy (set, action, recorded steps, inclusion check, modal control); expand/collapse and `Alt`-click; Button mode and its restrictions; recording guidelines and the recordable tool/panel list; ruler-unit/position caveat; Create New Action and Function Key exceptions (`F1`, `F4`/`F6`+`Ctrl` on Windows); Begin/Stop/Start Recording and `Esc`; **Allow Tool Recording** as a CS6 addition; Insert Stop/Allow Continue; Insert Path/multiple-path note; Insert Menu Item and the non-recordable list; modal controls and the red partial icon; exclude/include and the red parent check mark; Play flows (set/action/function key/part/single command); Playback Options (Accelerated/Step By Step/Pause For N Seconds); overwrite/add/rearrange/Record Again; duplicate/delete/Clear All; Action/Set Options; sets (new/move/rename/Save/Load/Replace/Reset) with `.atn` extension and `Presets/Actions` menu behavior; the `Ctrl+Alt`/`Cmd+Opt` text dump; Batch and droplet relationships; Image Processor `Run Action` set/action menus. Conditional actions appear in this corpus linking to **Creative Cloud**, so they are treated as a Kooka extension rather than CS6 behavior.
- `https://searxng` query "Photoshop CS6 Actions panel record play stop modal control button mode" and "Photoshop CS6 Character Styles Paragraph Styles panel Type menu" — discovery snippets locating secondary pages; not used as assertions.
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Essentials vs. Photography workspace panel sets; places Actions (with Histogram and Info) in the Photography workspace, not in default Essentials.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- Adobe's current-version Actions help pages surfaced by the above queries; later-version UI, not asserted as CS6.

Not used in this pass:

- `helpx.adobe.com` Actions pages (HTTP 403 / current-version only).

## Open questions

- **Conditional actions in shipped CS6.** Resolved as a non-parity scope decision: the fetched CS6 Help cross-reference points to Creative Cloud, so `Insert Conditional` ships as a **Kooka extension** with a documented three-condition subset. The panel tree therefore carries a conditional step kind that is not CS6-`.atn`-interoperable; `09-automation/actions.md` states this.
- **Exact default action sets and names** shipped with CS6 are not enumerated in the fetched text. Resolved for this release: no Adobe presets ship; the compiled-in `Default Actions` set is project-authored and listed above.
- **Function-key conflict resolution beyond "action wins"** (e.g. against tool shortcuts) is unverified. Resolved for this release: action combinations are F1–F12 with Shift/Ctrl; a duplicate action key is refused, a command collision warns, and Windows refuses F1/Ctrl+F4/Ctrl+F6.
- **Mapping of recorded commands to the substitute script API** (`09-automation/rust-scripting-replacement.md`) — this release records implemented catalogue menu commands and Brush/Pencil tool strokes; panel operations and native tool-specific strokes remain staged. Commands without a verified Adobe mapping use namespaced app-native events. Full scripting-host parity remains `AUTO-010`.
- **Single history-state grouping of a played action** is not CS6 behavior and is not added; playback keeps per-command history states and a later grouping change needs its own spec.
- **Adobe text-dump layout** is undocumented; the Save-time listing is a Kooka format and is deliberately not reloadable.
- **Whole-set Play** is an app decision (included actions in library order), not documented CS6 behavior; disabling it later would need a new spec change.
