# Proposal: Agent Control page in Preferences

## Why

Agent control can only be enabled by editing the session store, and the bridge's
error already points people at Edit > Preferences > Agent Control (#1075).

## What changes

- Edit > Preferences > Agent Control… opens a new page, for a window that serves
  agents, with the enable toggle, status, Edit and Save permissions, approval mode,
  allowed folders, and a copyable client snippet.
- Changes apply at once and persist, as on the other pages.
- The page says plainly what the permissions cannot do.
- On Flatpak and Snap the page is disabled and says why.

This page is not in CS6, like the rest of agent control.

## Capabilities

- Modified (added requirements): `interop/agent-permissions`.

## Impact

- `pictura-app`: `preferences_agent_page.*`, `agent_control.*`, `control_server.*`,
  `preferences_dialog.*`, `frame_preferences.cpp`.
- `pictura-script`: one catalogue row, and its goldens.
- New Qt suite `tst_preferences_agent`.
- No new dependencies.
