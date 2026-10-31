# Design: MCP stdio bridge

## Shape

`main()` branches on `--mcp` before `QApplication`, so no display is needed and
nothing but the protocol reaches stdout. `main.cpp` computes the endpoint and token
path with the same C++ the server uses, so Rust never repeats that logic.
`PICTURA_CONTROL_SOCKET` overrides the endpoint.

The crate has three parts: `server` (pure JSON-RPC handling over a `Backend`
trait), `client` (the control socket), and `spawn` (a headless child).

## Protocol

Newline-delimited JSON-RPC: `initialize`, `notifications/initialized`, `ping`,
`tools/list`, `tools/call`. `initialize` answers with the client's version if it is
supported, otherwise the newest supported.

## Tools

One static tool per control method, prefixed `pictura_`, with arguments passed
through; `ControlServer` validates them. There are no per-filter tools, which keeps
`tools/list` small. `key`, `set_unsaved_policy` and `set_gpu_compute` are not
exposed. A tool error is a result with `isError`, so the model can read it.

Results are JSON text. A result with `base64` becomes MCP image content.
`initialize` tells the client that document text is untrusted data. The permission
layer, not this wording, is the defence.

## Socket reads

A reply over 16 MiB is read to its newline and discarded, answering `too_large`,
so the stream stays in step. A timeout keeps the connection, and a late reply is
skipped by id. Timeout is 300 s, above the approval timeout, because a large filter
can be slow.

## Trust

On Unix the bridge checks that the socket and token belong to this user and that
the token is not readable by others before sending it.

ponytail: on Windows another user could squat the pipe name; nothing checks the
peer. Closing that needs a Win32 call.

## Spawn

The child runs `--headless --control` with a temp socket and `--state-home`, the
tier, roots and approval, and `--control-exit-on-disconnect`. The default tier is
read. The bridge connects once at start and keeps the connection, so a killed bridge
also ends the child.
