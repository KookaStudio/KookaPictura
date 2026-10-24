# Actions

- **Spec ID:** `AUTO-001`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Actions panel, action sets, and playback exist in CS6 Standard and Extended.
- **New in CS6:** `Changed` — **Allow Tool Recording** lets brush strokes and other tools be recorded directly in CS6 (CS5 could not); this release records Brush/Pencil strokes and refuses native tool-specific and mask strokes until their replay paths land; Contact Sheet II and PDF Presentation return as Automate options. The Actions panel itself, action sets, modal controls, and `.atn` files are otherwise CS5-equivalent. **Insert Conditional is not a CS6 feature**: Adobe documents conditional actions for Creative Cloud, and this project ships a Kooka extension with a documented condition subset (see *Special steps*).
- **Depends on:** `ARCH-011` (plugin-and-scripting-abi), `AUTO-010` (rust-scripting-replacement), `01-architecture/undo-history`, `01-architecture/document-model`, `AUTO-002` (droplets), `AUTO-003` (batch-processing).

## CS6 behavior

An **action** is a recorded series of tasks (menu commands, panel options, tool actions) played back on one file or a batch of files. Actions are grouped into **action sets** and stored on disk as `.atn` files; they are the basis for **droplets** (`AUTO-002`) and the **Batch** command (`AUTO-003`). Photoshop ships predefined actions (`.atn` presets) that can be used, edited, or replaced.

### Recording

- Create with the **Create New Action** button or `New Action` from the panel menu: name, target set, optional **Function Key** shortcut, and a **Color** (Button-mode display colour). Action function keys are **F1–F12** with optional Shift and Ctrl/Command; on Windows `F1`, `Ctrl+F4` and `Ctrl+F6` are refused. An action key takes precedence over a colliding command shortcut, the dialog warns rather than blocks, and clearing the key or deleting the action restores the command's shortcut. The Keyboard Shortcuts editor does not list action keys. Recording begins at `Begin Recording`; the record button turns red. Stop with the Stop button, `Stop Recording`, or `Esc`.
- Under **recording**, commands/tools are appended in order. This release records implemented catalogue menu commands that call the command bus, plus Brush/Pencil tool strokes. CS6's wider surface (Marquee, Move, Polygon, Lasso, Magic Wand, Crop, Slice, Magic Eraser, Gradient, Paint Bucket, Type, Shape, Notes, Eyedropper, Color Sampler, and History/Swatches/Color/Paths/Channels/Layers/Styles/Actions panel operations) remains a staged gap.
- **Not recordable:** most painting/toning tools, tool options, View commands, and Window commands. These are inserted afterwards with `Insert Menu Item` (the command then executes at playback, prompting its dialog if modal). With **Allow Tool Recording** on, a completed Brush or Pencil stroke is appended as one app-native tool-stroke step. Native tool-specific strokes and mask strokes are refused with a named diagnostic until their replay paths land.
- Recorded dialog/panel settings are the values in effect at record time. Modal operations and position-recording tools use the current ruler units; recording at percentage units is the documented way to keep position relative across file sizes.
- `Save As` records the filename unless changed; leaving the filename unchanged and changing only the folder is the documented way to redirect output without baking in a name.
- Editing: double-click a command to overwrite its values; drag to rearrange within/across actions; `Record Again` re-runs the action with prompts to re-capture modal settings; `Start Recording` appends to the selected action.

### Special steps

| Step | Menu command | Behavior |
|---|---|---|
| Stop | `Insert Stop` | Pauses playback with a message and (optionally) an **Allow Continue** button. |
| Non-recordable command | `Insert Menu Item` | Inserts a command by choosing it from a menu while the dialog is open; no parameter values are recorded. |
| Path | `Insert Path` | Inserts the currently selected path as an app-native step; on playback the work path is set to it. A later `Insert Path` replaces an earlier one inside the same action (record a `Save Path` between them to keep several). |
| Conditional | `Insert Conditional` | **Kooka extension**, not a CS6 feature: `If Current <condition>` → `Then Play Action` / `Else Play Action`, each choosing an action from the same set or `None` (not both `None`). This release evaluates document is square, document mode, and document has a selection on the active document. Common pattern: test the opposite by swapping Then/Else. |
| Conditional mode change | `File > Automate > Conditional Mode Change` | A separate CS6 Automate command (tracked as `AUTO-004`), not the `Insert Conditional` extension. It records a source-mode set and a target mode as one action step, avoiding an error when the opened file is not the expected source mode. |

### Modal controls

A **modal control** pauses playback at a command/action/set so the user can enter values in its dialog or use a modal tool. Enabled/disabled by clicking the box left of the name; a red dialog icon on an action/set means *some but not all* its commands are modal. Modal controls cannot be set in Button mode, and a command inserted via `Insert Menu Item` is *specific*: its control is shown off and cannot be enabled, but playback still opens the command's dialog.

**Dialog options** are stored per step and combine with the user-toggleable `withDialog` flag:

| Dialog options | Meaning | Playback |
|---|---|---|
| `0` | optional | follows `withDialog` (`false` → no dialog, `true` → dialog) |
| `1` | required | always opens the dialog; the control is on and cannot be turned off |
| `2` | none | never opens the dialog; the control is off and cannot be turned on |
| `3` | specific | control is off and not editable, but playback opens the command's dialog (Insert Menu Item and similar) |
| `4` | stop/continue | an in-memory recording value only; it is written to `.atn` as `0` |

### Playback

`Playback Options` sets one of three speeds:

- **Accelerated** (default) — normal speed; the screen may not update, so files open/modify/save without appearing.
- **Step By Step** — each command completes and the image redraws before the next.
- **Pause For __ Seconds** — a fixed pause between commands.

Playback can start at a set, an action, or a selected command; `Ctrl`/`Command`-click Play plays a single command. Commands excluded (unchecked) are skipped. Playing a **set** is an explicit app decision, not documented CS6 behavior: its included actions play in library order. In **Button mode**, clicking a button plays the whole action (excluded commands still skipped) but individual commands/sets are not shown and modal/exclude editing is unavailable. One playback runs at a time; a second start is refused and recording is refused during playback. Playback stops on the first error and this release has no continue-after-error path. An empty action or set plays as a no-op success.

### Managing sets

Actions can be rearranged, duplicated, deleted, renamed, and have their options changed. Sets can be created, saved (`Save Actions` → `.atn`), loaded, replaced, reset to defaults, or printed: `Ctrl+Alt`/`Command+Option` + `Save Actions` writes a human-readable text listing that Kooka Pictura cannot reload; the exact Adobe text layout is undocumented, so this listing is a Kooka format and is not claimed as Adobe-compatible. A set placed in `Presets/Actions` appears at the bottom of the panel menu after restart. The project ships **no Adobe preset actions**: `Default Actions` is compiled in (Duplicate Layer, Flatten Image, Grayscale, and Resize to 1000 px, each with a distinct colour) and is not discovered as an installed set. `Reset Actions` restores it, with OK replacing and Append adding the set under a unique name.

### Action descriptor basis

Each recorded command is an **Action Manager** event: an *event ID* plus an *ActionDescriptor* of key/value parameters, executed with a dialog mode. The ScriptListener plug-in logs these as `charIDToTypeID` + `ActionDescriptor.putInteger/...` + `executeAction` source (`AUTO-004`). The event-data model (`ActionDescriptor`, `ActionList`, `ActionReference`) is shared with the scripting host and native plug-ins via the command layer (`ARCH-011`).

### `.atn` file

An action set file is a big-endian binary structure with **no leading magic or
file-type signature**; the first field is the version. One `.atn` file holds
exactly one action set. All container integers are big-endian; descriptor
internals follow the value grammar below, and `Pth ` keeps its own
little-endian path fields.

| Field | Size | Notes |
|---|---|---|
| Version | 4 | `16`; any other version fails with a named diagnostic |
| Action set | variable | Name (counted UTF-16BE: 4-byte code-unit count, trailing NUL preserved when present), `expanded` (1), action count (4), then actions |
| Action | variable | Function-key number (2; `0` = none, `n` = Fn; values outside 1–12 are preserved opaquely), Shift (1), Cmd/Ctrl (1), colour index (2; `0`–`7` documented, other values preserved opaquely), name (counted UTF-16BE, trailing NUL preserved when present), `expanded` (1), step count (4), then steps |
| Step | variable | `expanded`, `enabled`, `withDialog` (1 each), dialog options (1, `0`–`4`), event-ID form (`long`/`TEXT`, 4), event ID, display name (4-byte length-prefixed byte string, no NUL), `hasDescriptor` (4, `0` or `-1`), and, when `-1`, the descriptor |

- **Event IDs.** A `long` event carries a four-character code after the form; a
  `TEXT` event carries a length-prefixed byte string. The display name is always
  a length-prefixed byte string (not UTF-16); imported bytes are preserved
  verbatim and decoded lazily.
- **Descriptor.** The embedded descriptor has **no version or length prefix**;
  it starts directly with its class. It uses Adobe's OSType-tagged structure
  (`obj `, `Objc`, `VlLs`, `doub`, `UntF`, `TEXT`, `enum`, `long`, `comp`,
  `bool`, `type`/`GlbC`, `alis`, `tdta`, `ObAr`, `UnFl`, `Pth `) with the
  reference forms `prop`, `Clss`, `Enmr`, `rele`, `Idnt`, `indx`, and `name`.
  In version-16 `.atn` descriptors, `Idnt` and `indx` references carry the
  desired class before the numeric value. `ObAr` stores its object count, shared
  class, item count, and one key/unit/double-list entry per item; each `UnFl`
  carries a unit ID and a double count equal to the object count. The codec
  distinguishes a length-prefixed string identifier from an interned
  four-character code and preserves item order and duplicate keys.
- **Event mappings.** Verified commands use Adobe event IDs and keys, including
  `GsnB` (Gaussian Blur) with `Rds ` and `#Pxl`, and `save` for Save As with
  `As  ` and `In  ` — **not** `SveA`. Tool strokes, conditional steps, and
  Insert Path use namespaced Kooka `TEXT` events (`pictura.<kind>`); these are
  app-native and are not claimed as CS6- or Adobe-interoperable.
- **Trailing bytes** after the single declared set are preserved opaquely and
  re-emitted on write.

The Actions palette persists to `Actions Palette.psp` as a version-16 palette:
a set count followed by that many sets, with **no magic signature**. It is app
state, not a backup or interchange format. The library is read at launch and
written atomically (through `QSaveFile`) on a debounced change and at shutdown,
not only at quit; a failed write leaves the previous palette intact. The
application ships no Adobe preset palette, and the compiled-in `Default Actions`
set is not discovered from disk.

**Dialog options** are set at record time and combined with the user-toggleable
`withDialog` flag: `0` optional follows `withDialog`, `1` required always opens
the dialog and cannot be turned off, `2` none never opens it and cannot be
turned on, `3` specific is shown off and not editable but playback opens the
command's dialog, and `4` is an in-memory stop/continue value written as `0`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Panels > Actions` | panel | `F9` | The Actions panel (a content widget in a `PanelGroup`). |
| Actions panel buttons | toolbar | — | Stop, Record, Play, Create New Set, Create New Action, Delete. |
| Actions panel menu | menu | — | `Button Mode`; separator; `New Action…`, `New Set…`, `Duplicate`, `Delete`, `Play`; separator; `Start Recording`/`Stop Recording`, `Record Again…`; separator; `Insert Menu Item…`, `Insert Stop…`, `Insert Conditional…` (Kooka extension), `Insert Path`; separator; `Action Options…`/`Set Options…`, `Playback Options…`, `Allow Tool Recording`; separator; `Clear All Actions`, `Reset Actions`, `Load Actions…`, `Replace Actions…`, `Save Actions…`; separator; installed action sets; separator; the group's `Close` and `Close Tab Group` rows. |
| Action function key | shortcut | `F1`–`F12` (+Shift, +Ctrl/Cmd) | Assigned per action; overrides a colliding command shortcut. Windows refuses `F1`, `Ctrl+F4` and `Ctrl+F6`. |
| `File > Automate > Create Droplet…` | dialog | — | Creates a droplet from an action (`AUTO-002`). |
| `File > Automate > Batch…` | dialog | — | Runs an action over files (`AUTO-003`). |
| `File > Scripts > Script Events Manager…` | dialog | — | Binds events to actions/scripts (`AUTO-004`). |
| `File > Automate > Conditional Mode Change…` | dialog | — | Records a conditional mode-change step. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Action name | string | `Action N` | any Unicode | Stored as a counted UTF-16BE string in `.atn`; authored names include the trailing NUL code unit. |
| Action set name | string | `Set N` | any Unicode | Counted UTF-16BE, trailing NUL included when authored. |
| Function key | enum | None | `None`, `F1`–`F12` | Windows excludes `F1` and `Ctrl+F4`/`Ctrl+F6`; a duplicate combination is refused. |
| Shift modifier | bool | false | on/off | |
| Cmd/Ctrl modifier | bool | false | on/off | |
| Button colour | enum | None | None, Red, Orange, Yellow, Green, Blue, Violet, Gray | Colour index `0`–`7`; other indices are preserved opaquely. |
| Modal control | tri-state | off | off / on / partial(red) | Derived from children for actions/sets; cannot be set in Button mode. |
| Command enabled | bool | true | on/off | Unchecked commands are skipped. |
| `withDialog` | bool | per record | on/off | Editable only for dialog options `0`. |
| Dialog options | u8 | `0` | `0` optional, `1` required, `2` none, `3` specific, `4` stop/continue | `4` is written as `0`; `3` still opens the dialog. |
| Playback speed | enum | Accelerated | Accelerated / Step By Step / Pause | Persisted in the session store. |
| Pause duration | seconds | `0` | ≥0 | Only with `Pause For __ Seconds`. |
| Conditional condition | enum | — | document is square / document mode / document has a selection | **Kooka extension**; evaluated on the active document. |
| Conditional source modes | mode set | All | Bitmap…Lab | Separate `File > Automate > Conditional Mode Change` command (`AUTO-004`), not `Insert Conditional`. |
| Conditional target mode | enum | — | Bitmap…Lab | Same separate command. |
| Stop message | string | — | any | `Allow Continue` optional. |

## Algorithms & pipeline

Behavioral parity only; Adobe's exact playback engine is closed. The observable pipeline is:

```text
record:
  UI command → command bus
             → if recording: append ActionStep { event_id, descriptor, dialog_options }
                          (tool recording wraps raw tool input events as steps)

playback(speed):
  for each enabled step in action:
    if step.dialog == modal and displayDialogs allows: prompt / pause for modal tool
    dispatch event_id + descriptor through the command layer
    if speed == StepByStep: repaint document between steps
    if speed == Pause: sleep(pause_seconds)
    if step is Stop: show message (block or Allow-Continue)
    if step is Conditional: evaluate condition → play referenced action (bounded depth)
    if step is InsertPath: set work path
    if playback cancelled / error: stop; error handling per caller (Batch/Droplet)
```

- An action is *not* automatically a single undo step in CS6 (Help advises taking a History snapshot before playback, then restoring it to undo). Kooka Pictura keeps the same **per-command** history: each played command records its normal state, and this release adds no one-history-state-per-action grouping (`01-architecture/undo-history`).
- `Play` recorded as a step composes actions; recursion is bounded at 64 invocations and fails with a named error.
- Tool recording stores a bounded, versioned app-native stroke payload (`TEXT` event `pictura.<kind>`), not an Adobe-interoperable command. The encoding is a Kooka extension with a documented sample/payload ceiling; an over-limit stroke is refused rather than truncated. This is the least-documented area (see Open questions).

## Rust module mapping

The engine is the Qt-free `pictura-script` crate; the exact file codec lives in `pictura-codec`.

- `pictura-script::command_catalogue` — `CommandSpec { id, path, label, shortcut, implemented, checkable }`, `command_catalogue()`, `command_id_for_path()`, and the frozen `const_names()` table checked against `command_ids.h`.
- `pictura-script::command` — `CommandId`, `CommandClass`, `CommandDefinition`, `CommandParams`, `DialogMode`, `CommandError`; the one mutation path shared with the app command bus.
- `pictura-script::action` — `ActionSet`, `Action`, `ActionStep`, `FunctionKey`, `ActionColor`, `DialogOptions`, `StepFlags`, and `ModalState`.
- `pictura-script::atn` — converts the `pictura-codec::action_file` `Raw*` types to and from the owned action model; reads/writes `.atn` and `.psp`.
- `pictura-script::library` — `ActionLibrary` CRUD, save/load/replace/reset/clear, and the compiled-in `Default Actions` set.
- `pictura-script::recorder` — `Recorder` start/stop/record; tool strokes are appended only under Allow Tool Recording.
- `pictura-script::player` — `Player` step cursor, speed, modal handling, cancellation, and the 64-depth limit.
- `pictura-script::error` — `ActionError` with named diagnostics.
- `pictura-codec::action_descriptor` — the exact Action Manager AST and reader/writer (`ActionDescriptor`, `ActionValue`, `ActionReferenceItem`), separate from the PSD `DescValue` model.
- `pictura-codec::action_file` — the version-16 `.atn`/`.psp` container codec (`RawAtn`, `RawActionSet`, `RawAction`, `RawStep`).

The app bridges are `command_catalogue.rs` (`CommandSpecRow`, `command_id_for_path`) and `actions_engine.rs` (`ActionsEngine` snapshot, CRUD, recording/playback, persistence); the C++ `CommandRegistry` gains executors, context handlers, and a recording sink.

Boundary types: `ActionDescriptor`/`ActionValue` (exact codec), `ActionSet`/`Action` (owned, serialisable), `DialogMode`, `PlaybackOptions`.

## Qt6 component mapping

Widgets, not QML: the Actions panel is a content `QWidget` hosted by the shared panel chrome, matching CS6's dense list/drag feel. It is not a `QDockWidget`.

- `ActionsPanel` (`QWidget`, object name `actionsPanel`) — tree or Button-mode view, bottom toolbar, and the panel-menu hook; the same widget is used in the docked group, the iconic flyout, and the floating overlay.
- `ActionsTreeModel` (`QAbstractItemModel`) — set → action → step rows with stable node ids; check state = enabled; modal column = modal state; drag-and-drop reorder/duplicate; no filter proxy (CS6 has no filter).
- `ActionsButtonView` (`QListView` in icon mode) — flat colour-tagged action buttons over the same engine data; the panel swaps views when Button Mode toggles.
- `PanelGroup` — owns tabs, `Close`/`Close Tab Group`, the iconic flyout, and the float host; `setPanelMenuHook` fills the Actions rows and enablement on `aboutToShow`.
- `ActionsDialogs` — New Action / New Set, Action Options (including target set), Set Options, Playback Options, Insert Stop, Insert Menu Item (command picker), and Insert Conditional.
- Function-key assignment goes through the frame's key seam; it removes the colliding command shortcut while the action owns the combination and restores it when the key is cleared or the action is deleted. The Keyboard Shortcuts editor is unchanged.

Drag-and-drop uses `QAbstractItemModel`'s native MIME support.

## Data-model impact

- **New persisted types (not in PSD):** `ActionSet`/`Action`/`ActionStep` serialised to exact version-16 `.atn` and to `Actions Palette.psp`; `ActionsPaletteState` (expansion/selection/Button mode/Allow Tool Recording/Playback Options) stored as additive keys under session schema 10.
- **Exact action-descriptor types** (`ActionDescriptor`, `ActionValue`, `ActionReferenceItem`) live in `pictura-codec` and are distinct from the tolerant PSD `DescValue` model. Shared scripting/plug-in Action Manager types remain `ARCH-011` scope.
- **Undo granularity:** per command, CS6-like. Playback adds each command's normal history state; there is no single "undo action" state and this change does not add grouping. Record shape `HistoryRecord::ActionOp { set, action, step_index }`.
- **Serialization:** action sets never enter PSD/PSB; XMP untouched. Stop, path, conditional, and tool-stroke steps carry no image payload.
- **Function-key map:** an action combination takes precedence over the command binding at dispatch; the command's stored binding is unchanged, and clearing/removing the action restores it. The shortcuts editor does not list action keys.
- **Default set and palette:** `Default Actions` is compiled in, not a file asset and not discovered in `Presets/Actions`; the library is written to `Actions Palette.psp` atomically on a debounced change and at shutdown.

## Edge cases

- **Function-key collision:** an action shortcut overrides the command shortcut with the same keys; the options dialog warns, not blocks. A duplicate action/action combination is refused. Windows refuses `F1`, `Ctrl+F4`, and `Ctrl+F6`.
- **Non-recordable commands:** recording them directly is impossible; they are only reachable as `Insert Menu Item` steps, whose dialog opens at playback even though their modal control is off and not editable.
- **Button mode:** cannot show steps, cannot set modal controls, cannot exclude commands, cannot edit. Toggling back to list mode preserves all hidden state.
- **Conditional:** Kooka extension. Both branches `None` is invalid; at least one branch is required, and referenced actions must be in the same set. Moving or deleting an action referenced by a conditional is refused. A conditional with no active document stops with a named error.
- **Recursive `Play` / conditional:** bounded at 64 invocations and fails with a named error, not a hang.
- **Insert Path memory:** complex paths are memory-hungry; a later `Insert Path` replaces the earlier one, so playback does not duplicate geometry per step.
- **Unicode names / locale:** set and action names are counted UTF-16BE; imported trailing NULs are preserved and lookups by name are locale-stable.
- **`.atn`/`.psp` from another version:** version ≠ 16 fails with a named diagnostic. Unknown event identifiers are preserved and reported at playback; an unknown descriptor type fails closed because the next event boundary is unknowable.
- **Partial load:** parse failures return the sets that parsed plus an error naming the offset; when nothing parses, the existing library is unchanged.
- **File references in steps:** `Open`/`Save As`/settings-file steps that bake in a path may not resolve on Linux; Batch/Droplet override flags (`AUTO-003`) are the documented escape. Save As uses the `save` event with `As  `/`In  `, not `SveA`.
- **Playback errors:** a missing target, unsupported event, wrong-mode document, or failing step stops playback on the first error with a named diagnostic; this release has no continue-after-error path.
- **Accelerated playback:** dialogs opened by a specific step still appear; an optional modal step with `withDialog` off uses recorded values or the command's defaults.
- **Cancellation:** cancelling a play aborts cleanly; commands already applied keep their per-command history states.
- **Empty action / empty set:** play is a no-op success; save/load round-trips the structure.
- **Headless:** the engine is headless-testable; the panel is unavailable without a GUI. Concurrent playback and recording during playback are refused.

## Parity acceptance criteria

1. Given a new action with steps recorded from the UI, playing it reproduces each command's document mutation within the owning command's documented tolerance.
2. Given an action with a modal control on a dialog command, playback pauses and uses the entered values; with the modal control off it uses the recorded values.
3. Given an unchecked (excluded) command, playback skips it and the parent action shows the partial-state indicator.
4. Given `Playback Options = Step By Step`, the image redraws between commands; with `Accelerated` it need not.
5. Given an action with an `Insert Stop` step, playback shows the message and honours `Allow Continue`.
6. Given an `Insert Menu Item` step whose command is not recordable, the command executes at playback and its dialog appears when modal.
7. Given an `Insert Conditional` step (Kooka extension), the correct Then/Else action plays for the condition; both-`None` and cross-set references are refused.
8. Given a `.atn` set written by Kooka Pictura and re-read, the set/action/step structure (names, keys, colour, enabled/modal flags, descriptors) round-trips byte-for-byte, including identifier form, item order, duplicate keys, and trailing bytes.
9. Given an `.atn` action set produced by CS6 (community corpus), Kooka Pictura lists its sets/actions and plays commands whose event IDs it supports; unsupported events fail with a named diagnostic and unknown descriptor types fail closed.
10. Given a function key assigned to an action, pressing it plays the action and overrides a colliding command shortcut; clearing the key or deleting the action restores the command.
11. Given `Button Mode`, clicking a button plays the entire action; switching back to list mode restores step visibility and modal state.
12. Given a `Play`/conditional recursion cycle, playback aborts with a depth error within a bounded time.
13. Given a palette with one valid and one malformed set, the valid set loads and the error names the offset; when nothing parses, the library is unchanged.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — *Photoshop CS6 Help* pp. 625–644: *About actions and the Actions panel*, *Creating actions* (guidelines, record, path, stop, modal controls, exclude, insert menu item, edit/rerecord), *Playing and managing actions* (play, playback speed, manage, sets), *Recording tools in actions | CS6*, *Adding a conditional mode change to an action*; pp. 79–80 What's New (Allow Tool Recording, Contact Sheet II/PDF Presentation). Conditional actions are documented for Creative Cloud, not CS6. Fetched via `curl` + `pdftotext`.
- `https://github.com/johnshopkins/adobe-scripts/raw/master/Photoshop/Photoshop-CS6-Scripting-Guide.pdf` — Action Manager chapter (ScriptListener logging, `charIDToTypeID`, `ActionDescriptor`, `executeAction` pattern), notifier/event context. Community mirror; fetched via `curl` + `pdftotext`.
- `https://github.com/johnshopkins/adobe-scripts/raw/master/Photoshop/Photoshop-CS6-JavaScript-Ref.pdf` — `ActionDescriptor`/`ActionList`/`ActionReference` property and method inventory; `Application.doAction`/`executeAction`; Appendix A (event ID codes). Community mirror; fetched via `curl` + `pdftotext`.
- `https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — *Adobe Photoshop File Formats Specification*, "Additional File Formats → Actions": action file header (version 16), action set/action/command layout, descriptor structure and OSType keys, reference structure. The version-16 files this project reads carry no leading `8BAC` signature. Fetched via `curl`.
- `https://raw.githubusercontent.com/LongMarch7/json-photoshop-scripting/master/Documentation/Photoshop-Actions-File-Format/README.md` — community `.atn`/`.psp` format write-up: action-set/action/command fields, Action Manager type encodings, recording/playback dialog options. Community source.
- Cross-reference `docs/01-architecture/plugin-and-scripting-abi.md` (`ARCH-011`) and `docs/09-automation/rust-scripting-replacement.md` (`AUTO-010`) for the shared Action Manager model, command bus, and scripting host.

## Open questions

- **Exact `PlaybackOptions`/`playbackParameters` encoding.** The JS reference exposes `Application.playbackParameters` as an `ActionDescriptor` but not its keys. This release persists playback options in the session store, not in `.psp`; a future palette-parity pass would need CS6 preference captures.
- **Whole-set Play.** CS6 Help does not document playing an entire set. This release ships included actions in library order as an app decision; a later parity decision to disable it needs a new OpenSpec change.
- **Adobe text-dump layout.** The listing written by `Ctrl+Alt`/`Command+Option` + Save Actions is a deterministic Kooka format; Adobe's exact layout is undocumented, the listing is not reloadable, and it is not claimed as Adobe-compatible.
- **CS6 tool-stroke encoding.** Whether CS6 stores raw input events or a parameterised tool command is not documented. This release uses a bounded, versioned app-native `TEXT` payload; a future CS6 capture can tighten it without changing the user-visible behavior.
- **Tool-recording fidelity.** Imported tool strokes from Adobe are not decoded as tool strokes; unsupported events fail with a named diagnostic. A replay-diff corpus would be needed before claiming CS6 fidelity.
- **Conditional condition list.** Adobe's full list is unavailable because the feature is CC-only. This release ships document is square, document mode, and document has a selection on the active document; extend only with a new OpenSpec change.
- **`.atn` interoperability scope.** The project guarantees exact version-16 read/write for supported values, preserves unknown event identifiers and trailing bytes, and uses namespaced app-native events for tool/conditional/path steps. Full Adobe round-trip for those three step kinds is not claimed.
- **Function-key policy outside Windows.** CS6 restricts `F1`, `Ctrl+F4`, `Ctrl+F6` on Windows; this release applies those refusals only on Windows and leaves the combinations free elsewhere.
- **Action-set preset provenance.** Resolved for this release: no Adobe presets ship, and `Default Actions` is independently authored and compiled in.
