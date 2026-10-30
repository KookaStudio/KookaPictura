# Proposal: Live control sessions and agent permissions

## Why

#1053 lets an agent work in the window a person is editing in. The control
server only runs under `--control`, which assumes nobody is watching: it discards
unsaved work, turns crash recovery off, and leaves paths unconfined. An agent can
also be misled by the content it reads, so what it may do has to be enforced by
the app, not by the agent (#1063).

## What changes

- `AgentControl` owns a control server in the running app, off by default, and
  can start and stop it at runtime. Settings persist in the session store.
- `ControlPolicy` has three forms. Live comes from settings and can ask the
  person. Headless (`--control --control-tier …`) refuses instead of asking. Bare
  `--control` stays unrestricted, as today.
- Every method has a tier: Read, Edit, Save or Admin. The check is in the server.
- `dispatch_command` is default-deny outside bare `--control`.
- Allowed folders and three approval modes: Manual, Auto-approve, Allow all.
- Approval uses a non-modal bar, so the GUI never blocks on an agent's request.
- Unsaved prompts are decided per request, so a person's Close and Quit still ask.
- Socket `document open` is deferred, so a person's dialog during a load is safe.
- Edits are refused while a person is mid-interaction.

The Preferences page is the next PR; until then the feature stays off unless the
session store enables it.

## Capabilities

- New: `interop/agent-permissions`.
- Modified: `interop/agentic-control`.

## Impact

- `pictura-app`: `agent_control.*`, `control_policy.*`, `control_approval_bar.*`,
  `control_server_gate.cpp`, `control_scope.*`, `session.*`, `dialogs.*`, `main.cpp`.
- New Qt suites `tst_control_policy`, `tst_control_approval`, `tst_control_live`.
- No new dependencies.
