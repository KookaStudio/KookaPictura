# Design

## Context

- **Actions engine.** `pictura-script` owns the action model, `.atn` codec,
  recorder, and `Player` (a step cursor). C++ `ActionsPanel` drives playback on
  a timer (`actions_panel_playback.cpp`), executing command steps through the
  `CommandRegistry` in `Replay` or `Modal` mode. Playback is asynchronous; there
  is no "playback finished" signal today.
- **Step params in `.atn`.** `atn::command_event` writes the Adobe keys a
  `ParameterSpec` names *and* the full JSON params under a private key, so any
  new param (Save As `options`, Open `path`) round-trips without codec work.
- **Save.** `Save As` records `{as, in}` (`import_save_save.cpp`). The replay
  executor (`frame_commands.cpp`, `saveToRecordedPath`) calls
  `saveActiveAsWithRememberedOptions`, which reads `Session::saveFormatOptions`
  (one JSON object per format key, `save_format_options.h`). Saves write to a
  temp file and rename (`pictura-codec/src/save/mod.rs`), so cancellation cannot
  leave a partial output.
- **Open.** `ImportSaveCoordinator::startOpenDocumentAtPath(path, interactive)`
  returns a token; `documentOpenFinished(token, outcome, error)` fires on
  completion, and `waitForOpen(token)` spins a local event loop.
  `confirmProfileForOpen` is the profile-mismatch prompt.

## Goals / Non-Goals

**Goals:** CS6 Batch for Folder/Opened Files sources and all three
destinations, both overrides with real Open and Save As steps, File Naming,
both error policies, cancel, and a widget split that Create Droplet (#617) can
reuse.

**Non-Goals:** Import/Bridge sources, droplets, Image Processor, scripting
`batch()`, PDF options in steps, parallel inputs.

## Decisions

### D1. Pure engine in `pictura-script::batch`

`crates/pictura-script/src/batch/` with `naming.rs` (`NamingField`,
`FileNaming::render`, `has_unique_field`, `sanitize`), `source.rs`
(`resolve_folder(root, recurse, extensions)`), and `options.rs`
(`BatchOptions` serde model, `validate(&options, &library, fs) -> Result<(),
BatchError>`, `action_has_save_as`, `action_has_open`, `override_save_path`,
`error_log_line`). Everything testable without Qt lives here; the C++ runner
only sequences. `pictura-script` already depends on `serde_json`; no new
dependency. The date comes in as `(year, month, day)` so tests are
deterministic.

*Alternative:* put naming in C++ next to the widget. Rejected: Create Droplet
and the future scripting `batch()` need the same rules, and engine tests are
cheaper than Qt tests.

### D2. Bridge: free functions over JSON, in the actions engine bridge

The `actions_engine` bridge gains an `extern "Rust"` block of free functions
taking/returning `QString` JSON (`batch_resolve_folder`, `batch_token_labels`,
`batch_render_name`, `batch_naming_has_unique_field`,
`batch_override_save_path`, `batch_error_log_line`), implemented in
`actions_engine/batch_ops.rs`. A separate bridge file would grow the
lower-only bridge budget and need its own membrane; C++ reaches these through
`api/engine_bridge.h`. Validation needs the live library, so it is an
`ActionsEngine` invokable (`batch_validate`, plus `action_reaches_command` for
the Open warning), reached through two non-pure `ActionsEngineApi` virtuals so
the panel's C++ test engine needs no change. C++ mirrors the serde model as
`BatchOptions` (`batch_options.h`).

### D3. Signal-driven `BatchRunner` QObject

`crates/pictura-app/cpp/batch_runner.{h,cpp}` is a state machine:
`Opening → Playing → Saving → Closing → next`. It starts an open token and
continues on `documentOpenFinished`; it starts playback via a new
`ActionsPanel::playActionForBatch(setName, actionName, BatchPlayback)` and
continues on a new `ActionsPanel::playbackFinished(bool ok, QString error)`
signal. No nested event loops in the runner, so Cancel (from the progress
dialog) is processed between steps: it calls `ActionsPanel` cancellation, which
stops after the current step.

*Alternative:* a synchronous loop with `waitForOpen`. Rejected: it nests event
loops per input and makes cancellation and Qt Test timing fragile.

### D4. Batch playback context

`ActionsPanel::BatchPlayback { overrideOpen }` is set for one run by
`playActionForBatch`. In that mode a command step never opens its dialog, Stop
steps follow the ae_play_suppressed_stop rule, and with `overrideOpen` a
`file.open` step succeeds without executing. The Save As override lives on the
main window: `setBatchSaveAsRedirect(std::function<QString(format)>)`, which the
`FileSaveAs` replay executor consults; the runner installs it for the run and
it returns the batch path for the step's format, or empty to refuse.

### D5. Open step recording and replay

`file.open` joins `VERIFIED` with event `Adobe("Opn ")` and
`OPEN_PARAMS = [param("path", "null")]`. The coordinator records it in
`onOpenFinished` for a successful interactive open (File > Open, Open Recent),
not for batch, recovery, or control-server opens. The replay executor calls
`startOpenDocumentAtPath(path, interactive=false)` then `waitForOpen`; a
synchronous executor is acceptable here because playback steps already run one
per timer tick, and only the runner (D3) must stay loop-free.

### D6. Save As options in the step

`promptSaveFormatOptions` already produces a `SaveFormatOptions` for the chosen
kind. Recording adds `options = saveFormatOptionsToJson(o)[saveFormatKey(kind)]`.
The replay executor, when `options` is present, builds a `SaveFormatOptions`
from the remembered value overlaid with the step's object and saves with it
through `saveActiveAsWithOptions`'s non-dialog path, without calling
`rememberSaveFormatOptions`.

### D7. Destination without override

`Save And Close` calls `saveActive()` (the document's own path and format,
remembered options) and closes; an untitled Opened-Files document is an input
error. `Folder` saves to `destination/<rendered name>` in the document's
current format (its path's extension, PSD for an untitled document) with the
remembered options, then closes.

### D8. Persistence

The last-used options are stored as the `batchOptions` JSON object in the
session store (`Session`), beside `saveFormatOptions`; no new settings file.

## Inferred decisions (flagged for review)

- Input order: case-insensitive relative path.
- Serial letters after `z`: `aa`, `ab`, … (bijective base 26).
- Missing extension field: the extension is appended.
- Default naming `Document Name` + `extension` (the CS6 dialog default; the
  docs table lists `[Document Name]`, which yields the same names under the
  append rule).
- Compatibility replacement character `_`; Unix always on under Linux.
- Error log format: one `path: message` line per error.
- Suppressed-mode Stop steps.

## Risks / Trade-offs

- **A modal dialog opened by a non-recordable side path** (for example a missing
  font alert during open) would stall a batch. Mitigation: batch opens use
  `interactive=false`, the same path the control server uses headless.
- **`frame_commands.cpp` / `import_save_save.cpp` size.** Changes are small;
  check `scripts/file-size-allowlist.txt` ceilings and split if a file nears
  its cap.
