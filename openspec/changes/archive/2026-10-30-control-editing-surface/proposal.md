# Proposal: Real editing tools for the control server

## Why

An agent can only run a filter at default settings. It can't read or set the
active layer, see inside a group, export without changing the document, or use
any parameterised command (#1069). That is too little for the MCP tools in #1053.

## What changes

- `filter` takes `params`, validated against the filter's table for arity,
  finiteness and range. Filters that need a resource stay out.
- New methods: `list_filters`, `list_adjustments`, `adjust_apply`, `run_command`,
  `layer_tree`, `set_active_layer`, `export_png`, `history`, `commit_crop`.
- `selection` gains an atomic `lasso` op, so a lasso is one call and one state.
- A mutator may carry `doc`, and `edit` may carry `expected_index`, so a request
  made against stale state is refused.
- An agent can commit a crop it started. A person's pending crop stays theirs.

## Capabilities

- Modified (added requirements): `interop/agentic-control`.

## Impact

- `pictura-app`: `control_server_commands.cpp`, `control_server_gate.cpp`,
  `control_server_actions.cpp`, `control_server_documents.cpp`.
- New Qt suite `tst_control_editing`.
- No new dependencies.
