# Proposal

## Why

The Actions panel is still a `PlaceholderPanel` (`frame_build.cpp:124`), there is
no action engine, and `CommandRegistry` is a menu dispatcher with no
`ActionDescriptor` or recording bus (`commands.cpp:238`). The command catalogue
also lives only in C++ (`command_tree.cpp`, `commands.h`), so actions, droplets,
batch, and the future scripting host cannot read the same command metadata the
UI uses. This change delivers the foundation (#616), the Rust-owned catalogue
(#273), and the Actions panel and engine (#301) as one sequence, because each
stage depends on the previous one.

## What Changes

- **Rust owns the command catalogue.** The 589 command entries and the frozen
  `command_ids` move to `pictura-script`; a checked-in, drift-tested
  `command_ids.h` keeps the C++ call sites compiling. `command_tree.cpp` is
  deleted and `addDefaultCommands` iterates the Rust rows. `tst_command_tree`
  and the control server's `list_commands` output stay unchanged.
- **A shared ActionDescriptor and recording bus exists.** `pictura-codec` gains
  an exact, byte-preserving Action Manager descriptor/reference codec. A new
  `pictura-script` crate defines the command-definition table, action model,
  recorder, player, and library. `CommandRegistry` gains executor and recording
  hooks so UI commands and future scripts report one event.
- **Actions are a first-class library.** The engine reads and writes Adobe
  version-16 `.atn` files and the multi-set `Actions Palette.psp`; it preserves
  unknown event identifiers, reports parse failures without crashing, and
  round-trips Photoshop-written files whose events are parseable. Commands
  with verified Adobe event ids are written Adobe-compatibly; app-only steps
  use namespaced `TEXT` events.
- **The Actions panel is implemented as a content panel.** It is a dense
  three-column tree with tri-state exclusion and modal states, Button mode,
  recording/playback, function keys, drag reorder/duplicate, dialogs, and the
  full panel menu. It works in the docked group, the iconic flyout, and the
  floating overlay. It remains a separate hidden group, as CS6 Essentials has it.
- **Recording is staged and explicit.** Every implemented catalogue command that
  is recordable and every recordable Brush/Pencil stroke is classified as
  Adobe-verified, app-native, or non-recordable; a test fails on an unclassified
  executable. The first release instruments the verified dialog families and the
  high-value parameterless commands; the remaining app-native handlers are
  recorded once they report `recordCommand`. Tool strokes are wired through the
  same bus; panel operations remain staged.
- **A small original default set ships.** `Default Actions` contains
  Duplicate Layer, Flatten Image, Grayscale, and Resize to 1000 px. No Adobe
  preset action is used.
- **Conditional actions are a Kooka Pictura extension.** The CS6 Help labels
  `Insert Conditional` Creative Cloud-only. It is implemented, marked as an
  extension in the specs and docs, and encoded as an app-native step.
- **Behavioral docs are corrected** for `.atn`/`.psp`, dialog options, the
  CC-only conditional feature, and the panel host model.

## Capabilities

### New Capabilities

- `document/action-engine`: the action model, library, recorder, player, default
  set, function-key ownership, staged recording coverage, tool-stroke and
  conditional (extension) steps.
- `codec/actions-file`: the exact v16 ActionDescriptor/Reference codec and the
  `.atn`/`.psp` container reader/writer.
- `ui/actions-panel`: the Actions panel content widget, tree/button views,
  menu, dialogs, bottom toolbar, recording/playback state, and its docked,
  flyout, and floating presentations.

### Modified Capabilities

- `document/command-registry`: the catalogue is Rust-owned and exposed to C++;
  the registry gains a recording sink and per-command executors for playback.
- `ui/panel-rail`: Actions is an implemented content panel, not a placeholder.

## Impact

- **New Rust crate** `pictura-script` (Qt-free): catalogue, descriptor-facing
  command definitions, action model, library, recorder, and player; it uses the
  `pictura-codec` `.atn`/`.psp` container for file I/O.
- **`pictura-codec`**: new exact action-descriptor module and the
  `.atn`/`.psp` container; the existing `DescValue` PSD typed view is left
  unchanged.
- **`pictura-app`**: new cxx-qt bridges for the catalogue and `ActionsEngine`;
  new C++ panel, model/view, dialogs, playback loop, function-key map, and
  persistence; `commands.h` shrinks and `command_tree.cpp` is deleted;
  `CMakeLists.txt` gains the new files and test suites.
- **No PSD/XMP/document-format change.** `.atn` and `.psp` are app-level
  libraries/state, not document data.
- **Dependencies:** `serde`/`serde_json` become direct dependencies of
  `pictura-script` to serialise versioned parameter payloads inside the
  namespaced app-native `TEXT` events and the human-readable text listing; both
  are already in `Cargo.lock`, so no new transitive dependency or licence is
  introduced. `.atn`, `.psp`, and session JSON do not use them. No other new
  dependency is planned.
- **Docs:** `docs/09-automation/actions.md`,
  `docs/02-ui-ux/panels/actions-panel.md`, and
  `docs/dev/doc-code-parity-gaps.md` are corrected under
  `TASK-ALLOWS-DOCS`. `ROADMAP.md` is updated for the shipped Actions
  panel/engine, with remaining items linked to their issues.
- **Out of scope:** droplets (AUTO-002), Batch (AUTO-003), the scripting host
  (AUTO-010), and Adobe binary plug-ins. Tool-stroke, conditional, and Insert
  Path steps are not Adobe `.atn` interoperable; they are app-native and
  documented as such. Whole-set Play is an explicit app decision (the CS6 Help
  does not document it) and is flagged as a risk in `design.md`.
- **Delivery:** one PR with three sequential commits (#273, #616, #301), each
  buildable and tested, and this one OpenSpec change archived with the PR.
