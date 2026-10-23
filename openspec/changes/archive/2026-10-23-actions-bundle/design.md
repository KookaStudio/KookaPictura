# Design

## Context

See `proposal.md` for motivation. The implementation has to fit five existing
constraints:

- `CommandRegistry` owns a 589-entry C++ command table (582 explicitly written
  rows — 221 `add` calls plus 361 `leaf` calls — plus seven extra
  `Image > Mode` entries produced by the helper, excluding 64 separators) and
  229 frozen `command_ids` constants used by 1026 call sites. It dispatches
  through `std::function<void()>` handlers; dialogs and document mutations live
  in the C++ shell and in `PictureView` invokables.
- The Rust-to-C++ pattern already exists: `filter_tools.rs:152` exposes
  `filter_descriptors()` as a `Vec<FilterDescriptorRow>`, and the C++ shell
  builds its filter commands from it. The Actions catalogue should use the same
  shape rather than inventing a new bridge style.
- Panels are content `QWidget`s hosted by `PanelColumn`/`PanelGroup`, not
  `QDockWidget`s. The group supplies tabs, the per-widget menu, the iconic
  flyout, and the floating overlay (`ui/panel-column`).
- `pictura-codec::descriptor` is a tolerant PSD typed view (`DescValue`). It
  preserves unknown values as `Raw`, but it does not preserve the exact
  action-descriptor identifier form, tag form, or ATN `Idnt`/`indx` class
  prefix.
- History is per-edit snapshot capture (`document/edit-history`). There is no
  grouping of several commands into one undo state.

## Goals / Non-Goals

**Goals:**

- One command catalogue, authored in Rust, consumed by the C++ registry and
  inspection surfaces.
- One action execution path shared by the panel, future batch/droplets, and
  scripting: command identity plus parameter values in, recorded steps or
  document mutations out.
- Byte-exact Adobe version-16 `.atn` and `.psp` read/write for supported values,
  with unknown events preserved.
- Full Actions panel behavior in the docked group, iconic flyout, and floating
  overlay.
- Staged, test-guarded recording coverage with no silently unclassified
  commands.
- Engine code testable without Qt; Qt tests for the panel and command bus.

**Non-Goals:**

- Droplets, Batch, the scripting host, and Adobe binary plug-ins.
- Adobe-compatible encoding for tool strokes, conditional steps, and Insert
  Path; these are app-native `TEXT` events.
- Shipping Adobe preset actions or a bundled `.atn`.
- Moving every command handler or dialog into Rust.
- One undo state per played action; CS6 parity is per-command history.

## Decisions

### 1. New `pictura-script` crate; exact descriptor codec in `pictura-codec`

`pictura-script` is Qt-free and owns the catalogue, command definitions, action
model, library, recorder, and player. `pictura-codec` gains a separate exact
Action Manager descriptor module and the `.atn`/`.psp` container reader/writer.
`pictura-app` links the new crate and adds thin cxx-qt bridges.

*Why:* the architecture already names `pictura-script` for the command API; the
engine stays headless and fast to test. *Alternatives:* put the engine in
`pictura-app` (couples it to Qt/FFI and makes engine tests heavy), or put it in
`pictura-codec` (mixes image codecs with automation semantics).

The exact descriptor AST is separate from `DescValue` because `.atn` needs the
length-prefixed string vs interned-code distinction, `Idnt`/`indx` with a class,
`ObAr`/`UnFl`, `Pth `, and item order/duplicates. A command's embedded descriptor
has no version or length prefix: the step's `hasDescriptor` marker is `0` or
`-1`, and the descriptor starts directly with its class. Extending `DescValue`
would touch the PSD paths and risk changing their output. Both modules share the
low-level big-endian reader and key/string primitives. `Pth ` keeps its own
little-endian internal lengths.

### 2. Rust catalogue plus a generated C++ identifier header

`pictura-script::command_catalogue` holds every entry with id, path, label,
default shortcut, implemented flag, and checkable flag. The app bridge exposes
`Vec<CommandSpecRow>`; C++ `addDefaultCommands` iterates it. The 229 frozen ids
become a Rust `const_names` table; `scripts/gen-command-ids.py` emits
`crates/pictura-app/cpp/command_ids.h`, which is checked in. A Rust test parses
the header and fails on drift. `commandIdForPath` stays a tiny pure function in
C++ and is checked against the catalogue.

*Why:* 1026 C++ references need compile-time constants. *Alternatives:* delete
the constants and rewrite every call site (large, no benefit); hand-maintain two
tables (drift); generate the header at build time (more CMake/build-order
surface). The checked-in header plus drift test keeps one source of truth with
no build-system coupling.

Before deleting `command_tree.cpp`, capture a golden `list_commands` snapshot and
a golden id list from the current build. The Rust catalogue test compares
against them, so the control-server output stays unchanged.

### 3. C++ command bus with executors and a recording sink

`CommandRegistry` gains:

- `setExecutor(id, std::function<bool(const CommandParams&)>)` — replay without
  UI.
- `setContextHandler(id, std::function<void(const CommandContext&)>)` — the UI
  handler that can see the mode and recorded parameters; the existing `void()`
  handler stays for simple commands.
- `setRecordingSink(std::function<void(QString, const CommandParams&)>)` — the
  active recorder.
- `recordCommand(id, params)` — handlers call after a successful mutation.
- `executeRecorded(id, params)` — playback calls; missing executor is a named
  error.

`dispatch(id)` does not infer success from a `void()` handler. A handler that
mutates records through `recordCommand` after the mutation succeeds; a `void()`
handler that does not call it is not recorded. A context handler receives
`CommandContext { mode, params }`: UI mode behaves as today, replay mode uses the
executor, and modal replay opens the dialog prefilled. The control server's
`dispatch_command` goes through `dispatch` and therefore the same sink.

*Why:* dialogs, current selections, and panel state are C++. A Rust-only bus
would have to reimplement them or call back into C++ anyway. *Alternatives:*
annotate every `PictureView` method with a command identity (hundreds of
methods, no UI context); record only at menu dispatch (misses dialog values and
panel operations); build a second document-mutation layer (drift).

### 4. Rust engine, C++ playback loop

Rust owns the action library, recording session, and playback cursor. C++ owns
the loop, timers, redraws, modal dialogs, and cancellation UI. The engine
exposes the current step, advances it, and records results; C++ calls the
registry executor or the UI handler.

Play and conditional nesting share a fixed limit of 64 invocations; exceeding
it is a named error.

*Why:* Qt timers and dialogs are C++ concerns, and driving the loop from C++
avoids re-entrant `Pin<&mut PictureView>` borrows. *Alternatives:* a Rust player
with callback traits (modal input would cross cxx repeatedly); a C++ action
library (duplicates the `.atn` model and codec).

### 5. Panel is a content widget with a dynamic menu hook

`ActionsPanel : QWidget` is added with `addPanel(...)` exactly like
`HistoryPanel`. `PanelGroup` supplies tabs, the corner menu, iconic flyout, and
float. The static menu table in `panel_group_menu.cpp` gains `Insert
Conditional…` and keeps the labels and order; a new
`PanelGroup::setPanelMenuHook(panelName, std::function<void(QMenu*)>)` runs on
`aboutToShow` for the Actions panel to set enabled/checked states, swap the
`Action Options…` row to `Set Options…` for a set selection, remove the
`(installed sets)` placeholder, and append one row per installed `.atn` set.
Installed-set discovery and the default-set source are overridable in tests.
`runPanelMenuAction` gains an Actions branch. The group's close rows stay
`Close` and `Close Tab Group`; the older `Close Group` wording in the
`ui/panel-column` spec is pre-existing label drift. This change records the
current label in `ui/actions-panel` and leaves the unrelated panel-column
capability untouched; the mismatch is reported rather than silently copied.

*Why:* a `QDockWidget` contradicts `ui/panel-column`; a separate `QMenu` would
duplicate group chrome. *Alternatives:* special-case Actions inside
`updateHeaderMenu` (works but does not scale), a full dynamic menu model (more
surface than this change needs).

The tree is a custom `QAbstractItemModel` with stable node ids and a C++ mirror
of the Rust snapshot. `QStandardItemModel` would make parent check/modal states
and moves harder to keep correct. Button mode uses a `QListView` in icon mode
over a flat action adapter, keeping the same engine data.

### 6. `.atn` policy: exact read, verified-subset write, app-native fallback

The writer emits version-16 fields exactly. Verified commands use Adobe event
ids and descriptor keys: `GsnB` with `Rds ` and `#Pxl`, `save` with `As  ` and
`In  ` (not `SveA`), `FltI`, `CnvM`, `ImgS`, layer show/hide/set, fill,
transform, rotate/flip, `Stop` with `Msge`/`Cntn`, `slct` for Insert Menu Item,
and `Ply ` for Play. Commands without a verified mapping, tool strokes,
conditional steps, and Insert Path use a namespaced `TEXT` event
(`pictura.<kind>`). Unknown imported event identifiers are preserved and
reported at playback; an unknown descriptor value type fails closed because the
next event boundary is unknowable. The event display name and `TEXT` event
identifier are length-prefixed byte strings (preserved verbatim; ASCII for
authored ids), while set and action names are counted UTF-16BE strings whose
trailing NUL is preserved when present.

*Why:* Adobe's tool-stroke and conditional encodings are undocumented; a real
CS6 capture is not available. A namespaced event keeps our own library fully
functional and honest. *Alternatives:* JSON-only actions (breaks Save Actions),
claim full Adobe compatibility (false), omit tool/conditional (violates the
requested feature set).

Event ids are a small hand-authored table for implemented commands only. The
project does not vendor a wholesale Adobe SDK terminology table. Unknown event
identifiers pass through opaquely; an unknown descriptor type fails closed.

### 7. Staged recording coverage

Every implemented command id in the catalogue, and every recordable panel
operation and tool stroke, gets a `CommandDefinition` with a class:

- **Adobe-verified:** real event id and descriptor schema.
- **App-native:** `pictura.<command-id>` event.
- **Non-recordable:** no step; insertable through Insert Menu Item.

The first release instruments the verified dialog families and the high-value
parameterless commands whose handlers report `recordCommand`; every implemented
command is classified, and the remaining app-native commands are recorded once
their handler is migrated. A Rust test enumerates implemented catalogue ids and
fails on a missing class; the panel-operation and tool tables get the same
exhaustive check. `docs/dev/doc-code-parity-gaps.md` records the remaining
instrumentation coverage.

*Why:* exhaustive Adobe descriptor mapping for every command is not available
from public sources and would delay the panel indefinitely. The classification
makes gaps visible instead of silent. *Alternatives:* all-or-nothing Adobe
mapping (blocked), record only menu dispatch (misses the feature).

### 8. Function keys remove the colliding command shortcut

The engine enforces one action per key combination. Function keys are F1–F12;
on Windows F1 and Ctrl+F4/Ctrl+F6 are refused. The check takes a platform seam
so the Qt test can exercise the Windows branch on Linux. When an action takes a
combination, the frame removes it from the command registry's shortcuts and
restores it when the action is deleted or loses the key. The keyboard-shortcuts
editor is unchanged: it does not list action function keys, and a shadowed
command keeps its stored binding even while the action wins.

*Why:* Qt reports duplicate application shortcuts as ambiguous rather than
giving a deterministic winner. *Alternatives:* `ApplicationShortcut` on a
`QShortcut` (ambiguous, steals text focus), check-first in every key handler
(easy to miss paths).

### 9. Default set and persistence

`Default Actions` is a compiled-in Rust `ActionSet` with Duplicate Layer, Flatten
Image, Grayscale, and Resize to 1000 px, each with a distinct color. Resize to
1000 px records Image Size with width 1000 px, constrain-proportions on, and
Bicubic resampling. The set is not a file asset and is not discovered from the
presets directory; Reset restores it. The library persists as
`Actions Palette.psp` in the app state directory using the exact palette codec;
writes are serialized and atomic through `QSaveFile` and happen on a debounced
change and at shutdown, not only at quit, so a crash cannot lose a successfully
parsed library. Panel state persists in the existing session JSON under schema
10 as additive keys: missing keys load defaults, unknown keys are preserved, and
no schema bump is required. Expansion and selection are stored by set/action
name path, the ordinal among same-named siblings, and step index because `.atn`
has no stable ids; a missing name drops that entry.

*Why:* provenance rules prohibit Adobe presets, and the palette codec already
exists. *Alternatives:* ship an empty reset set (less useful), store the
library in session JSON (large, loses `.psp` parity), quit-only writes (matches
Photoshop but loses edits on a crash).

### 10. Docs corrections

`docs/09-automation/actions.md`, `docs/02-ui-ux/panels/actions-panel.md`, and
`docs/dev/doc-code-parity-gaps.md` are corrected for the exact `.atn` layout
(including `hasDescriptor`, byte-string display names, `ObAr`/`UnFl`/`Pth `,
and the `save` event rather than `SveA`), the five dialog-option values and
their `withDialog` combinations, the CC-only conditional feature, the
content-panel host model, F1–F12 function keys, Action Options including the
target set, the project-authored default set, whole-set Play uncertainty, and
the app-native tool/conditional/path limits. Commits carry `TASK-ALLOWS-DOCS`.

### Capability deltas not taken

- **`ui/icon-assets`:** the existing `<panel>.<item>` requirement already
  covers `actions.*`; the change adds assets and provenance, not a new icon
  contract.
- **`ui/keyboard-shortcuts-editor`:** action function keys are not editor
  bindings. The editor still lists commands and tools; an action only shadows a
  command at dispatch time, and the command's stored binding is unchanged.
- **`ui/panel-column`:** the panel remains a content widget hosted by the
  existing `PanelGroup`. The dynamic menu hook fills the researched entries the
  existing requirement already names; no host or menu requirement changes.
  `Close Tab Group` is the current code label; the older `Close Group` wording
  is pre-existing spec drift, not a new requirement, and is reported as a
  residual issue.
- **`document/edit-history`:** played steps use the existing per-command
  snapshot path. The change forbids grouping by default and adds no history
  contract.
- **`interop/agentic-control`:** `dispatch_command` keeps its refusal rules and
  response shape; recording is an internal hook and adds no control-server
  behavior.

## Risks / Trade-offs

- **Real `.atn` files vary more than the published tables** → byte-exact
  synthetic fixtures, a self-skipping corpus test keyed on `PICTURA_ATN_CORPUS`,
  named diagnostics at the failing offset, and no silent fallback.
- **Tool-stroke, conditional, and Insert Path steps are not
  Adobe-interoperable** → they carry a namespaced `TEXT` event and are
  documented as Kooka extensions; no claim of CS6 `.atn` parity for them.
- **Event-id provenance** → small hand-authored table from public documentation
  and recorded examples; no wholesale SDK-derived table; unknown ids preserved.
- **Handler instrumentation breadth** → the coverage matrix and classification
  test are the gate; Adobe-verified families ship first, app-native families next, and
  non-recordable commands are explicit.
- **Panel model complexity** → stable node ids, a C++ mirror, and
  `QAbstractItemModelTester` in the Qt tests; no per-row FFI.
- **`pictura-codec` now has two descriptor models** → the exact model is
  action-only and shares the low-level codec; PSD tests and `DescValue` behavior
  stay unchanged.
- **Deleting the C++ command table touches 1026 references indirectly** → the
  frozen ids stay in a generated header, so call sites do not change; the
  golden `list_commands` snapshot catches behavior drift.
- **The 589-entry catalogue and the codec can exceed the file-size cap** → split
  along menu/concern seams and keep the allowlist unchanged; a file over 1200
  lines is a gate failure, not a new allowlist entry.
- **Function-key collisions** → the engine refuses duplicates, the dialog warns
  on command collisions, and deletion restores the command.
- **Whole-set Play is not documented by CS6 Help** → the spec defines it as
  playing the set's included actions in order, with a test; confirm against a
  CS6 capture or disable Play for set selections without touching `.atn`.
- **Installed-set discovery depends on a writable presets root in tests** → the
  discovery root is injectable; the shipped default set is compiled in and
  never discovered from disk.
- **Tool-stroke payloads can outgrow a `TEXT` event** → samples are bounded,
  the app-native payload is versioned, and an over-limit stroke is refused with
  a named diagnostic instead of corrupting the action.
- **The Adobe text-dump layout is undocumented** → the text listing is a
  deterministic Kooka format, not Adobe-compatible, and Load refuses it.
- **Playback has no UI without a GUI** → the engine is headless-testable and
  the panel is absent; whole-set Play and modal prompts require the panel.
- **Large `.psp`/tree** → tree uses `uniformRowHeights` and does not expand all;
  palette reads/writes are bounded and atomic; the panel snapshot is a single
  typed crossing per refresh.

## Migration Plan

1. **#273:** add `pictura-script` and the catalogue, add the app bridge and
   generated `command_ids.h`, replace `addDefaultCommands`, delete
   `command_tree.cpp`; run the catalogue/`tst_command_tree`/control-server
   checks. No persisted data changes.
2. **#616:** add the exact descriptor codec, action model, `.atn`/`.psp` codec,
   library, recorder/player foundation, and the command-bus hooks; instrument
   the Adobe-verified command families. The panel is still hidden; no
   user-visible change beyond recording being ready.
3. **#301:** replace the placeholder with `ActionsPanel`, wire the menu hook,
   recording/playback, function keys, default set, persistence, icons, docs, and
   tests; archive this change with the PR.

Rollback is commit-by-commit in reverse order. The new `.psp` file is additive;
a build without the Actions panel ignores it. Session keys are optional and
unknown-key-preserving, so an older build can read a newer store.

## Open Questions

- **Whole-set Play:** CS6 Help does not document playing an entire set. This
  change intentionally ships it as an app decision (included actions in library
  order). A later parity decision to disable it would need a new spec change.
- **Conditional conditions:** the CS6 list is unavailable because the feature is
  CC-only. The extension ships a documented subset (document is square, document
  mode, document has a selection) evaluated on the active document; extend only
  with a new OpenSpec change.
- **Insert Path and tool-stroke encodings:** the Adobe encoding is unknown; the
  app-native fallback and preservation of imported bytes are final for this
  change. A future CS6 capture can tighten them without changing the specs'
  user-visible behavior.
- **Text dump:** Adobe's exact listing format is unknown; the Kooka listing is
  deliberately not reloadable and is not claimed as byte-compatible.
