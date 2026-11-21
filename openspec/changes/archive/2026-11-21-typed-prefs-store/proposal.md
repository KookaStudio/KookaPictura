# Proposal

## Why

The application stores global preferences in the same `state.json` file as window geometry, dock arrangements, and recent files. Issue #641 replaces that stopgap with a typed preferences store and gives users a way to recover from a damaged file or reset their settings at launch.

## What Changes

- Store existing global preferences in versioned, user-editable `prefs.toml`. Give the Rust store responsibility for the schema, validation, writes, and recovery. Keep session and workspace layout in their JSON stores.
- Migrate existing preference keys from `state.json` on upgrade. Preserve session data and honor explicit resets on later launches. Recover malformed TOML from the last good backup or defaults, require a fresh agent grant, and tell the user what happened.
- Offer the documented combined reset through the startup key gesture where it is detectable. Offer the same confirmation through `--reset-preferences` on Wayland. The reset covers preferences, custom command and tool shortcuts, color settings, and workspaces.
- Include preferences in portable settings backups, continue accepting version 1 backups, and extend profile isolation to the new config store.
- **BREAKING** for tools that read or write application preference keys directly in `state.json`. Those keys move to TOML. New settings exports use format version 2, which older builds refuse.

## Capabilities

### New Capabilities

- `ui/preference-storage` defines the typed store, migration and recovery, isolated launches, and startup reset.

### Modified Capabilities

- `document/workspace-persistence` moves interface brightness to preferences and keeps geometry and layout in session state.
- `ui/settings-backup` includes TOML in settings exports and translates legacy preference keys during import.
- `ui/keyboard-shortcuts-editor` persists existing shortcut overrides in the preferences store.
- `tools/tool-framework` persists Use Shift Key For Tool Switch in the preferences store.
- `compositing/gpu-compute-backend` persists the effective GPU preference in the preferences store.
- `interop/agent-permissions` persists opt-in agent preferences in the preferences store and protects the config root from agent file access.
- `document/raw-settings-storage` persists the database or sidecar choice in the preferences store.
- `ui/type-preferences` persists the Type preference options in the preferences store.
- `ui/update-preferences` persists automatic update consent in the preferences store.

## Impact

- `pictura-app` gains a Qt-free Rust preferences module and a bridge for the shell. The work touches frame startup, preference saves, legacy session loading, portable backups, and test profile setup.
- Add a TOML parser and serializer so the store can validate typed fields and retain unrecognized TOML values. Reuse `serde` from the workspace dependency graph.
- The existing feature controls and their live or restart behavior define the migration scope. The work uses the existing shortcut override model and current Color Settings controls.
