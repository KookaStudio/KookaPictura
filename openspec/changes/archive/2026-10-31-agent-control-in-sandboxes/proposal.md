# Proposal: Agent Control in Flatpak and Snap builds

## Why

Agent Control is refused in Flatpak and Snap builds, because an MCP client outside
the sandbox can't reach a socket inside it (#1079). The issue suggests a shared
`--filesystem` grant, which would widen the sandbox for every user of the build.
The grant isn't needed: the bridge is the app binary, so it can run inside the
sandbox and carry MCP across the boundary on stdio.

## What changes

- The live endpoint moves to a runtime directory shared by a sandbox's instances.
- The client configuration names `flatpak run <id> --mcp` or
  `snap run <snap>.pictura --mcp`.
- The Preferences page works in sandboxed builds, except one-click registration,
  which can't reach the host's clients.
- The single-server lock stops depending on PIDs, which don't compare across
  Flatpak instances.
- The packaged builds run a GUI-to-bridge round trip in CI.

## Capabilities

- Modified: `interop/agent-permissions`.

## Impact

- `pictura-app`: `agent_control.*`, `control_server.*`, `preferences_agent_page.*`.
- `.github/workflows/flatpak.yml`, `.github/workflows/snap.yml`, new
  `scripts/check-sandbox-mcp.sh`.
- Docs: agent control, install notes, roadmap.
- No new sandbox permissions and no new dependencies.
