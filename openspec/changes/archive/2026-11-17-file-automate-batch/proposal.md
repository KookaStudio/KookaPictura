# Proposal

## Why

`File > Automate > Batch…` is specified in `docs/09-automation/batch-processing.md`
(`AUTO-003`) but is a disabled catalogue stub (`file.automate.batch`,
`implemented: false`); the parity register records `AUTO-3` as `MISSING`
(#618). An action recorded in the Actions panel can only be played on one open
document at a time, so the CS6 workflow of running an action over a folder of
files, saving each result under a naming scheme, does not exist.

Two gaps in the action engine also block the documented Batch overrides:

- `File > Open` is not a recordable step, so `Override Action "Open" Commands`
  has nothing to override.
- A recorded `Save As` step stores only its format (`as`) and path (`in`);
  replay uses the session's remembered format options, so an action cannot carry
  its own JPEG quality or TIFF compression into a batch.

## What Changes

- Make `File > Open` recordable as an Adobe `Opn ` step with the opened path,
  replayed by opening that path without the open-options dialogs.
- Record the chosen per-format options in every `Save As` step and replay a
  step with its own options; a step without options (older actions) keeps the
  remembered options.
- Add a pure Rust batch engine (`pictura-script::batch`): folder source
  resolution (optional subfolders), the CS6 File Naming tokens, starting serial,
  name compatibility, option validation, Save As override paths, and the error
  log.
- Add `File > Automate > Batch…`: the `BatchDialog` over a reusable
  `BatchOptionsWidget` and `FileNamingWidget`, a progress dialog with Cancel,
  and a batch runner that opens, plays, saves, and closes each input with
  playback dialogs suppressed. Sources are `Folder` and `Opened Files`.
  Destinations are `None`, `Save And Close`, and `Folder`. Errors either stop
  the batch or are logged to a file.
- Persist the last-used Batch options.
- Update the parity register (`AUTO-3`), `ROADMAP.md`, and `docs/dev/STATE.md`.

**Non-goals:** the `Import` and `Bridge` sources (no acquire plug-ins or Bridge
exist; Mini-Bridge is #624), Create Droplet (#617, which will reuse
`BatchOptionsWidget`), Image Processor and the other Automate commands (#619),
`Application.batch` scripting (#621), PDF Save As options in a step (PDF replay
keeps the remembered PDF settings), and parallel processing of inputs.

## Capabilities

### New Capabilities

- `document/batch-processing`: the batch job: sources, per-input pipeline,
  Open/Save As overrides, destinations, error policy, cancellation, and option
  validation.
- `document/batch-file-naming`: the File Naming token set, serial numbers and
  letters, dates, extensions, compatibility sanitizing, and the unique-field
  rule.
- `ui/batch-dialog`: `File > Automate > Batch…`, its controls and enablement,
  warnings, progress dialog, and last-used options.

### Modified Capabilities

- `document/action-engine`: adds recordable Open steps, Save As steps that
  carry their format options, and suppressed-dialog playback for batch runs.

## Impact

- `crates/pictura-script`: new `batch` module (`naming`, `source`, `options`),
  the `file.open` command definition, and the catalogue row flip.
- `crates/pictura-app`: a cxx-qt bridge over the batch engine; the Open
  recorder and replay executor; Save As option recording/replay; the dialog,
  widgets, runner, and progress dialog; `cmake/sources/*.cmake` registration.
- Tests: Rust unit tests for naming, sources, validation, and the action-step
  changes; Qt Test suites `tst_batch_dialog` and `tst_batch_run`.
- Docs: parity register, roadmap, state notes (separate `docs:` commit with
  `TASK-ALLOWS-DOCS`).
