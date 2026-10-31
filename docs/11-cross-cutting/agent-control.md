# Agent Control

- **Spec ID:** `XC-022`
- **Status:** `Shipped`
- **Parity tier:** `Non-goal (CS6)`. CS6 has nothing like it.
- **Depends on:** `11-cross-cutting/security-and-sandboxing.md` (`XC-005`)
- **Requirements:** `openspec/specs/interop/agentic-control`, `interop/agent-permissions`,
  `interop/mcp-frontend`

Agent Control lets an MCP client such as Claude Code work in Kooka Pictura, either
in the window a person is using or in a private headless instance.

## Shape

```
MCP client --stdio--> pictura --mcp --local socket + token--> ControlServer (GUI thread)
```

All enforcement is in `ControlServer`. The bridge forwards tool calls and decides
nothing, so an agent cannot talk around a rule by reaching the socket directly.

## Sessions

| Session | Started by | Limits | When a person would be asked |
|---|---|---|---|
| Live | Edit > Preferences > Agent Control | The person's settings | Approval bar |
| Headless | `--control --control-tier …`, or `pictura --mcp --spawn` | The flags | Refused |
| Test | Bare `--control` | None | Nothing is asked |

A live session is off by default, and read-only until the person grants more. Bare
`--control` discards unsaved work and is for test instances only.

## Permissions

| Tier | Holds | Needs |
|---|---|---|
| Read | Status, layers, pixels, screenshots, history list, `document open` | Enabled |
| Edit | Filters, adjustments, selections, layers, canvas pointer | Edit grant |
| Save | `save`, `save_as`, `export_png` | Edit and Save grants |
| Admin | `key`, window `pointer`, `set_unsaved_policy`, `set_gpu_compute` | Bare `--control` |

`dispatch_command` and `run_command` run only commands on fixed lists.

## Files

Targets are canonicalized first, so a symlink is judged by where it lands. The state
directory and the endpoint directory are refused in every mode.

| Mode | Overwrites | Outside allowed folders |
|---|---|---|
| Manual | Asks | Asks |
| Auto-approve (default) | Asks, unless the agent made the file or the person approved it this session | Asks, with the same exception |
| Allow all | Never asks | Never asks |

An unanswered approval times out after 60 seconds.

## A person's work

A dirty `close` or `revert` is answered `unsaved`. A person's own Close still prompts.
Edits are answered `human_busy` while a dialog, popup, held mouse button, pending
transform or crop, or loading document is active, so leaving Preferences open pauses
agents. Crash recovery stays on.

## Connecting a client

Use the Register buttons on the Agent Control page, or add this to the client:

```json
{"mcpServers": {"pictura": {"command": "/path/to/pictura", "args": ["--mcp"]}}}
```

Add `--spawn` for a private instance, with `--tier read|edit|save`, `--root DIR`, and
`--approval allow-all`. `PICTURA_CONTROL_SOCKET` overrides the endpoint.

Registering writes to the client's own configuration, after showing the change and
taking a backup. Claude Code is changed through its CLI. A file that isn't plain JSON
is left alone.

## Limits

- Permissions constrain agents using this interface. An agent that also has a shell
  can read the token and edit the session store.
- A same-user race between a path check and its use, and hardlinks, are out of reach.
- A native portal file dialog may never reach the modal guard.
- On Windows nothing checks the pipe's peer, so another user could squat its name.
- Flatpak and Snap builds need no extra permission: the client runs `flatpak run
  <id> --mcp` or `snap run <snap>.pictura --mcp`, so the bridge is a second
  instance inside the sandbox and meets the window on a runtime path both can see.
  One-click client registration is not offered there. A Flatpak sees host files
  only through a `flatpak override` the person adds (#1079).
- The bridge implements MCP up to 2025-11-25 (#1080).
- A timed-out edit can still finish, so an agent should check state before retrying.
