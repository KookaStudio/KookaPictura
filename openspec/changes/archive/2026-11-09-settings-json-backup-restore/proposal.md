# Proposal

## Why

Issue #1155 asks for a backup/restore of Kooka Pictura's settings as a single
`pictura.json` file. Today every setting is already persisted, but in several
places: `state.json`, the `workspaces/` store, adjustment presets, action sets,
the Actions palette, and PDF presets. There is no in-app export or import, so a
person must quit the application and copy the XDG directories by hand, and
there is no safe way to restore a backup on another machine.

## What Changes

- `Edit > Preferences > General` gains **Export Settings…** and **Import
  Settings…** buttons.
- **Export** writes one pretty-printed `pictura.json` with a format id, a format
  version, the app version, the export time, and every user store: the session
  state (preferences, panel layout, tool options, shortcuts, swatches, actions
  palette state, recent files, window geometry), the named workspaces, user
  adjustment presets, installed `.atn` action sets, the Actions palette, and
  PDF presets. Machine caches and crash-recovery snapshots are excluded.
- **Import** validates the file, backs up the current settings to a timestamped
  file, sanitizes machine-specific values (missing recent files and off-screen
  window geometry are dropped), and stages the import. The running window is
  not partially re-laid-out; the import is applied before the next launch's
  session load, and the person is offered **Quit Now** or **Later**.
- Absent sections in an imported file leave the corresponding store untouched.
  Unknown keys in the session object survive the round trip.
- Malformed files, a newer format version, unsafe file names, and invalid
  base64 are refused with a message and leave the live stores unchanged.

## Capabilities

### New Capabilities

- `ui/settings-backup`: the Preferences export/import surface, the
  `pictura.json` backup format, the staging and next-launch apply, validation
  and sanitization, and the restart prompt.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app/cpp/settings_backup.{h,cpp}` — new store collector,
  validator, stager, and startup applier.
- `crates/pictura-app/cpp/preferences_dialog.{h,cpp}` — General-pane buttons
  and request signals.
- `crates/pictura-app/cpp/frame_preferences.cpp` — file dialogs, export/import
  handlers, and the restart prompt.
- `crates/pictura-app/cpp/frame.h`, `main.cpp` — the public handler surface and
  the pre-frame pending-import apply.
- `cmake/sources/shell.cmake`, `crates/pictura-app/cpp/tests/CMakeLists.txt` —
  the new source and Qt Test suite.
- `ROADMAP.md`, `docs/dev/STATE.md` — record the shipped feature.
