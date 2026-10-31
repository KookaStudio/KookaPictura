# Proposal: Register MCP clients from Preferences

## Why

A person enabling Agent Control still has to copy a snippet into another app's
configuration by hand (#1078). This writes to files that belong to other apps, so
it has to be careful and visible.

## What changes

- The Agent Control page lists Claude Code, Cursor, and on macOS and Windows Claude
  Desktop, with a Register or Unregister button for each.
- Claude Code is changed through its own CLI, because its config file is live.
- The others are merged under `mcpServers` only, after the person sees the change,
  with a timestamped backup and an atomic write. A file that can't be parsed is
  refused, never rewritten.

## Capabilities

- Modified (added requirements): `interop/mcp-frontend`.

## Impact

- `pictura-app`: `client_registration.*`, `client_registration_box.*`,
  `preferences_agent_page.*`.
- New Qt suite `tst_client_registration`.
- Docs: agent control, threat model, plan status.
- No new dependencies.
