# Proposal: MCP stdio bridge

## Why

The control server speaks its own protocol. MCP clients such as Claude Code need a
standard one (#1053, #1073).

## What changes

- `pictura --mcp` speaks MCP over stdio and forwards tool calls to the control
  server. It enforces nothing: permissions stay in `ControlServer`.
- It attaches to a running window by default. `--spawn` starts a headless instance
  restricted by `--tier`, `--root` and `--approval`.
- `--control-exit-on-disconnect` stops a spawned instance when its bridge goes.

This replaces the planned separate `pictura-mcp` binary on `rmcp` and `tokio`. The
crate is a library in the main binary, so there is nothing new to package on any
platform and no new dependency. The cost is keeping a small protocol layer current.

## Capabilities

- New: `interop/mcp-frontend`.

## Impact

- New crate `pictura-mcp` (std, `serde_json`, `libc` on Unix, all already locked).
- `pictura-app`: `mcp_entry.rs`, `main.cpp`, `control_server.*`.
- `scripts/verify-control.sh`, `.github/workflows/windows.yml`.
