# Design — settings JSON backup and restore

## Context

Kooka Pictura already persists user state in several places, all below the XDG
state and data roots:

| Store | Path |
|---|---|
| Session/preferences | `$XDG_STATE_HOME/kooka-pictura/state.json` |
| Named workspaces | `$XDG_STATE_HOME/kooka-pictura/workspaces/` |
| Actions palette | `$XDG_STATE_HOME/kooka-pictura/Actions Palette.psp` |
| PDF presets | `$XDG_STATE_HOME/kooka-pictura/PDF Presets/*.joboptions` |
| Adjustment presets | `$XDG_DATA_HOME/kooka-pictura/Presets/Adjustments/*.presets` |
| Installed action sets | `QStandardPaths::AppDataLocation/Presets/Actions/*.atn` |

The session store is still C++ (`session.cpp` is a documented stopgap for the
deferred Rust `pictura-prefs` store). The feature is therefore a C++ module
beside the existing stores, with Qt Test coverage, so it does not depend on a
store migration that has not happened.

`gpu-backend.json` / `gpu-init.marker` are a machine-specific backend cache and
`recovery/` is a transient crash-recovery store; neither is a setting and both
are excluded. MCP client registration and external lens profiles are not
application data and are also excluded.

## Goals

- One portable `pictura.json` containing every user store listed above.
- Export and import from `Edit > Preferences > General`.
- Import never partially reconfigures the running window and never loses the
  imported file to the running process's quit-save.
- Malformed, newer, or unsafe files are refused without changing live stores.
- A same-build round trip restores every store losslessly.

## Non-goals

- A Rust `pictura-prefs` store or a TOML migration.
- Merging two settings sets; import replaces the stores it carries.
- Backing up document data, crash-recovery snapshots, caches, or external
  client configuration.
- Live re-application of imported settings in the running window.

## File format

The export is one pretty-printed JSON object:

```json
{
  "format": "pictura-settings",
  "formatVersion": 1,
  "app": "Kooka Pictura",
  "appVersion": "0.x.y",
  "exportedAt": "2026-11-09T12:34:56Z",
  "session": { "...": "the stored session object, unknown keys included" },
  "workspaces": {
    "index": { "...": "workspaces/index.json" },
    "files": { "<id>.json": { "...": "one workspace object" } }
  },
  "presets": {
    "adjustments": [ { "file": "levels.presets", "text": "..." } ],
    "actions": [ { "file": "set.atn", "base64": "..." } ],
    "pdf": [ { "file": "press.joboptions", "text": "..." } ],
    "actionsPalette": { "file": "Actions Palette.psp", "base64": "..." }
  }
}
```

A preset file is encoded as `text` when its bytes are valid UTF-8 and
round-trip through `QString::fromUtf8`/`toUtf8`; otherwise it is encoded as
canonical base64. Text files stay inspectable, and binary `.atn`/`.psp`
payloads stay lossless. The `presets` keys are optional; an absent key means
"leave that category alone".

The `session` object is the raw stored object, not a reserialization from
`SessionState`, so unknown keys survive. The frame saves the live session
before exporting so the file reflects the current window.

## Validation and sanitization

`readSettingsBackup` enforces a size ceiling, parses an object, and checks the
format and `formatVersion`. A version greater than the build's is refused
rather than downgraded. `stageSettingsImport` then:

- removes `writerToken` and `writtenAt` from the session so the next launch's
  save owns them;
- keeps only recent-file entries that are absolute and exist on this machine;
- keeps `windowGeometry` only when a throwaway `QWidget::restoreGeometry`
  intersects an available screen, and drops `windowMaximized` with it
  otherwise;
- accepts workspace and preset file names only when `QFileInfo::fileName()`
  equals the name, the name is not `.`/`..`, contains no `/` or `\`, and has
  the category's expected suffix;
- validates base64 canonically (`fromBase64` then re-encode equals the input);
- caps the number of files and the decoded bytes.

Unsafe entries are refused as a whole file rather than silently skipped, so a
person is not told a restore succeeded when part of it was discarded.

## Staging and applying

Import writes the sanitized wrapper to
`$XDG_STATE_HOME/kooka-pictura/pending-settings.json` and leaves the live
stores alone. `main.cpp` calls `applyPendingSettingsImport()` after the GPU
backend is pinned and before `PicturaMainWindow` is constructed, so the frame's
`loadSession()`, workspace initialization, and Actions-palette load all see the
imported values.

This staging avoids the quit-save race: the running process may still write
its old session and Actions palette on quit, but the next process applies the
pending file before it reads anything. The restart prompt therefore offers
**Quit Now** (quit; the next launch applies) and **Later** (keep working; the
next launch applies). Auto-relaunch is deliberately omitted because the
exiting process's destructors can write stores after `aboutToQuit`, which would
race a freshly started process.

`applyPendingSettingsImport` writes each present section and then removes the
pending file. It writes workspace files before their index, and it removes
stale workspace files only when the imported index is present. Preset
categories are replaced only when their key is present. A corrupt pending file
is renamed to `pending-settings.json.corrupt-<epoch>` and the launch continues.

## Backup before import

`importSettingsFrom` calls `saveSession()` to flush the live window, then
writes the current settings to
`$XDG_STATE_HOME/kooka-pictura/settings-backups/pictura-<timestamp>.json`.
If that backup cannot be written, the import is refused. The backup folder
keeps the ten most recent files so it does not grow without bound.

## UI

`PreferencesDialog` gains two buttons on the General page and two signals,
`exportSettingsRequested` and `importSettingsRequested`. The frame connects
them in `frame_preferences.cpp`, opens the file chooser, and calls the public
`exportSettingsTo` / `importSettingsFrom` test hooks. A failed operation shows
a `QMessageBox::warning`; a staged import shows the restart prompt. The prompt
is non-interactive on the offscreen platform and under a test override, so Qt
Test never blocks.

## Risks

- A backup from a newer build can carry session keys this build does not
  understand. The loader ignores them and the raw object preserves them on the
  next rewrite, matching the existing store's forward-key behavior.
- The Actions palette and installed `.atn` sets can both describe actions; the
  palette is the live library and the `.atn` files are installed sets. Both are
  exported so a restore does not lose either.
- A partial apply is possible if a disk fills mid-restore. The timestamped
  pre-import backup is the recovery path; the pending file is only removed
  after every present section is written.
