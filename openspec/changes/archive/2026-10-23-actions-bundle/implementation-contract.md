# Implementation contract — actions-bundle

Working note for the parallel implementation agents. Read `proposal.md`,
`design.md`, the capability specs, `tasks.md`, and `test-plan.md` first. This
file only fixes shared names and file ownership so the agents do not collide.

**No build, compile, test, ctest, clippy, fmt-check, or verification commands.**
The owner will prompt for those later. Do not commit or archive. Follow the
global `search_graph` rule before reading/editing code.

## File ownership

| Owner | Files |
|---|---|
| Rust codec | `crates/pictura-codec/src/action_descriptor.rs` (new), `crates/pictura-codec/src/action_file.rs` (new), `crates/pictura-codec/src/lib.rs`, codec tests under `crates/pictura-codec/tests/`, `scripts/fetch-atn-corpus.sh` |
| Rust script | `crates/pictura-script/**` (new crate), root `Cargo.toml`, `crates/pictura-app/Cargo.toml`, `crates/pictura-app/src/cxxqt_object/command_catalogue.rs` (new), `crates/pictura-app/src/cxxqt_object/actions_engine.rs` (new), `crates/pictura-app/src/cxxqt_object.rs` module list |
| C++ command layer | `crates/pictura-app/cpp/commands.h`, `commands.cpp`, `command_tree.cpp` (delete), `crates/pictura-app/cpp/command_ids.h` (generated/checked in), `scripts/gen-command-ids.py`, `control_server.cpp`, `frame_menus_filter.cpp`, `filter_commit.cpp`, `frame_menus_layer_ops.cpp`, `frame_menus_image.cpp`, `frame_menus_image_mode.cpp`, `frame_menus_edit.cpp`, `crates/pictura-app/cpp/tests/tst_command_tree.cpp`, `crates/pictura-app/cpp/tests/tst_command_bus.cpp` (new) |
| C++ actions panel | `crates/pictura-app/cpp/panels/actions_panel.*` (new), `actions_tree_model.*` (new), `actions_button_view.*` (new), `actions_dialogs.*` (new), `panel_group.cpp/.h`, `panel_group_menu.cpp`, `frame_build.cpp`, `frame.h`, `session.h/.cpp`, `crates/pictura-app/cpp/tests/tst_actions_panel.cpp` (new), all `CMakeLists.txt` additions/removals |
| Tool recording | `crates/pictura-app/src/cxxqt_object/impl_paint.rs`, `paint_tools.rs`, `crates/pictura-app/cpp/tool_*.cpp` only where a stroke begins/ends, plus the recorder API in `pictura-script` |
| Docs/assets | `docs/02-ui-ux/panels/actions-panel.md`, `docs/09-automation/actions.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/dev/m42-panel-menus.md`, `docs/dev/STATE.md`, `ROADMAP.md`, `assets/icons/actions.*`, `assets/pictura.qrc`, `assets/icons/lucide-map.json`, `assets/PROVENANCE.md` |

Do not edit a file owned by another agent. If a shared file is unavoidable,
append a note to `openspec/changes/actions-bundle/agent-notes.md` instead and
leave the edit to the owner.

## Rust codec API (`pictura-codec`)

`pub mod action_descriptor`:

```rust
pub enum ActionKey { Code([u8; 4]), String(Vec<u8>) }
pub struct ActionClass { pub name: String, pub id: ActionKey }
pub struct ActionDescriptor { pub name: String, pub class: ActionClass, pub items: Vec<(ActionKey, ActionValue)> }
pub enum ActionValue { Reference(Vec<ActionReferenceItem>), Descriptor(Box<ActionDescriptor>), GlobalObject(Box<ActionDescriptor>), List(Vec<ActionValue>), Double(f64), UnitFloat { unit: [u8; 4], value: f64 }, UnitFloats { unit: [u8; 4], values: Vec<f64> }, Text(String), Enumerated { kind: ActionKey, value: ActionKey }, Integer(i32), LargeInteger(i64), Boolean(bool), BooleanRaw(u8), Class(ActionClass), GlobalClass(ActionClass), Alias(Vec<u8>), RawData(Vec<u8>), Path(Vec<u8>), ObjectArray(ObjectArray) }
pub enum ReferenceForm { Property, Class, Enumerated, Offset, Identifier, Index, Name }
pub struct ActionReferenceItem { pub form: ReferenceForm, pub class: ActionClass, pub key: Option<ActionKey>, pub value: ReferenceValue }
pub enum ReferenceValue { None, Key(ActionKey), Integer(i32), Unsigned(u32), Text(String) }
pub struct ObjectArrayItem { pub key: ActionKey, pub unit: [u8; 4], pub values: Vec<f64> }
pub struct ObjectArray { /* shared class, object count, packed items */ }
pub struct AtnError { /* message, offset */ }
pub fn read_action_descriptor(bytes: &[u8], integer_references_have_class: bool) -> Result<ActionDescriptor, AtnError>;
pub fn write_action_descriptor(desc: &ActionDescriptor, integer_references_have_class: bool) -> Vec<u8>;
```

`pub mod action_file`:

```rust
pub enum RawEventId { Text(Vec<u8>), Long([u8; 4]) }
pub struct RawStep { pub expanded: bool, pub enabled: bool, pub with_dialog: bool, pub dialog_options: u8, pub event_id: RawEventId, pub display_name: Vec<u8>, pub descriptor: Option<ActionDescriptor> }
pub struct RawAction { pub function_key: u16, pub shift: bool, pub ctrl: bool, pub color: u16, pub name: Vec<u16>, pub expanded: bool, pub steps: Vec<RawStep> }
pub struct RawActionSet { pub name: Vec<u16>, pub expanded: bool, pub actions: Vec<RawAction> }
pub struct RawAtn { pub set: RawActionSet, pub trailing: Vec<u8> }
pub fn read_atn(bytes: &[u8]) -> Result<RawAtn, AtnError>;
pub fn write_atn(value: &RawAtn) -> Vec<u8>;
pub fn read_psp(bytes: &[u8]) -> Result<Vec<RawActionSet>, AtnError>;
pub fn write_psp(sets: &[RawActionSet]) -> Vec<u8>;
```

The reader preserves exact `ActionKey` form, item order, duplicate keys, raw
`Pth ` bytes, `ObAr`, and the `Idnt`/`indx` class prefix. Unknown descriptor
types fail closed. `dialog_options == 4` is accepted and written as `0`.

## Rust script API (`pictura-script`)

Crate modules: `command_catalogue`, `command`, `action`, `atn`, `library`,
`recorder`, `player`, `error`.

- `command_catalogue::CommandSpec { id: &'static str, path: &'static [&'static str], label: &'static str, shortcut: &'static str, implemented: bool, checkable: bool }`
- `command_catalogue::command_catalogue() -> &'static [CommandSpec]`
- `command_catalogue::command_id_for_path(path: &[&str]) -> String`
- `command_catalogue::const_names() -> &'static [(&'static str, &'static str)]`
- `command::CommandId` (`String` wrapper), `CommandClass`, `CommandDefinition`, `CommandParams` (`serde_json::Value` object), `DialogMode`, `CommandError`
- `action::{ActionSet, Action, ActionStep, FunctionKey, ActionColor, DialogOptions, StepFlags}`
- `atn::{read_atn, write_atn, read_psp, write_psp}` converting the codec `Raw*` types
- `library::ActionLibrary` with create/rename/delete/duplicate/move/save/load/replace/reset/clear and the compiled-in `Default Actions` set
- `recorder::Recorder` with start/stop/record
- `player::Player` with step cursor, speed, modal, cancellation, 64-depth limit
- `error::ActionError`

`pictura-script` depends on `pictura-codec`, `thiserror`, `serde`, `serde_json`.

## `pictura-app` Rust bridges

- `command_catalogue.rs`: `CommandSpecRow` with `id`, `path`, `label`, `shortcut`, `implemented`, `checkable`; `fn command_catalogue() -> Vec<CommandSpecRow>`; `fn command_id_for_path(path: &QStringList) -> QString`.
- `actions_engine.rs`: an `ActionsEngine` QObject or opaque handle with:
  - `snapshot() -> Vec<ActionNodeRow>` where `ActionNodeRow { id, parent_id, kind, name, enabled, modal, color, function_key, step_index, can_expand }`
  - library CRUD, recording start/stop/record_command, playback start/next/cancel, palette load/save, default/reset/clear, function-key assignment, playback options, allow-tool-recording.
  - A `changed` signal/notification after structural changes.
- Add both modules to `crates/pictura-app/src/cxxqt_object.rs`.

## C++ command layer API

In `commands.h` (QtCore types are already available):

```cpp
struct CommandParams { QJsonObject values; };
enum class CommandMode { Ui, Replay, Modal };
struct CommandContext { CommandMode mode = CommandMode::Ui; CommandParams params; };
using CommandExecutor = std::function<bool(const CommandParams&)>;
using CommandRecordingSink = std::function<void(const QString&, const CommandParams&)>;

void setExecutor(const QString& id, CommandExecutor executor);
void setRecordingSink(CommandRecordingSink sink);
void recordCommand(const QString& id, const CommandParams& params);
bool executeRecorded(const QString& id, const CommandParams& params);
void setContextHandler(const QString& id, std::function<void(const CommandContext&)> handler);
```

`dispatch` never infers success from a `void()` handler. A handler that mutated
calls `recordCommand` after success. `executeRecorded` returns false and names
the id when no executor exists.

## C++ panel API

- `ActionsPanel : QWidget` with `setEngine(...)`, `refresh()`,
  `performPanelMenuAction(const QString&)`, `updatePanelMenu(QMenu*)`.
- `ActionsTreeModel : QAbstractItemModel` with stable node ids and the three
  columns (check, modal, name).
- `ActionsButtonView : QListView` in icon mode over the action library.
- `PanelGroup::setPanelMenuHook(const QString& panelName, std::function<void(QMenu*)>)`; invoke it on `QMenu::aboutToShow` in `updateHeaderMenu`.
- `PanelGroup::runPanelMenuAction` dispatches Actions ids to `ActionsPanel`.
- The panel is added in `frame_build.cpp` as a `QWidget` with object name
  `actionsPanel`, in a separate hidden group, using the existing
  `window.panels.actions` rail icon.

## Rules

- Use 4-space C++ indentation, repository naming, no comments unless they
  explain non-obvious intent. Keep files under the size caps; split by concern.
- New `.cpp`/`.h` files must be added to the appropriate `CMakeLists.txt`.
- Do not add a new dependency beyond `serde`/`serde_json` in `pictura-script`.
- Do not edit `docs/` unless you own the docs/assets scope; any docs commit
  needs `TASK-ALLOWS-DOCS`.
- Return a concise report listing files changed, API assumptions, and any
  remaining integration work.
