# Design: Agent Control page

## Wiring

`AgentControl` exists only for an interactive launch, so `showPreferences` adds the
page only when it finds one. `PreferencesDialog::addPage` adds a named page after
the CS6 panes. `frame.h` does not change.

`AgentControlPage` edits `AgentSettings` directly through `AgentControl::apply`,
which persists them and starts, stops, or re-applies. The dialog has no agent state.

## Status

`ControlServer` emits `clientsChanged`, and `AgentControl` forwards it and its own
changes as `statusChanged`, so the page updates without a timer.

## Agents and this page

An agent cannot change its own permissions: `key` and window `pointer` are refused,
and an open Preferences window answers edits with `human_busy`. The page says so,
because agents appear to stall while it is open.

## Snippet

The snippet names `$APPIMAGE` when set, since an AppImage's own path is a temporary
mount, and otherwise this executable, with `--mcp`.
