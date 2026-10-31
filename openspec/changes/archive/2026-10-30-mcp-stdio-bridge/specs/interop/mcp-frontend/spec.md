## ADDED Requirements

### Requirement: MCP over stdio

`pictura --mcp` SHALL speak MCP as newline-delimited JSON-RPC on stdin and stdout,
SHALL NOT need a display, and SHALL write only protocol messages to stdout. It SHALL
implement `initialize`, `ping`, `tools/list`, and `tools/call`, and SHALL answer an
unknown method with error -32601 and an unparseable line with -32700. It SHALL NOT
listen on any network port.

#### Scenario: Initialize negotiates a version [mcp_initialize]

- **WHEN** a client sends `initialize` with a supported version, then an unknown one
- **THEN** the first is echoed, the second gets the newest supported, and tools are
  advertised

#### Scenario: Notifications get no reply [mcp_notification]

- **WHEN** a client sends `notifications/initialized`
- **THEN** nothing is written

#### Scenario: Bad input is reported [mcp_bad_input]

- **WHEN** a client sends an unparseable line, then an unknown method
- **THEN** the responses carry -32700 and -32601 and the bridge keeps running

### Requirement: Tools forward to the control server

Each tool SHALL have a `pictura_` name and an input schema and SHALL forward its
arguments to one control method. `key`, `set_unsaved_policy`, and `set_gpu_compute`
SHALL NOT be tools. The bridge SHALL enforce no permission itself. A control error
SHALL be a tool result with `isError`. A result with `base64` SHALL be image content.

#### Scenario: Tools are listed [mcp_tools_list]

- **WHEN** a client calls `tools/list`
- **THEN** every tool has the prefix and an object schema, and no excluded method appears

#### Scenario: A call forwards its arguments [mcp_call_forward]

- **WHEN** a client calls `pictura_filter` with arguments
- **THEN** the backend receives `filter` with those arguments and the result is text

#### Scenario: A control error is a tool error [mcp_call_error]

- **WHEN** the control server answers `refused`
- **THEN** the result has `isError` and names the code

#### Scenario: An unknown tool is a protocol error [mcp_unknown_tool]

- **WHEN** a client calls a tool that does not exist
- **THEN** the response carries -32602

#### Scenario: Images are image content [mcp_image]

- **WHEN** a result carries `mime` and `base64`
- **THEN** the content holds an image item

### Requirement: The bridge reaches the control server safely

By default the bridge SHALL attach to the live endpoint on the first tool call, and
SHALL name Preferences in its error if nothing is listening. On Unix it SHALL refuse
a socket or token owned by another user, or a token others can read, before sending
the token. A reply over 16 MiB SHALL answer `too_large` and leave the connection
usable. A reply with another request's id SHALL be skipped.

#### Scenario: Nothing is listening [mcp_not_running]

- **WHEN** no endpoint exists
- **THEN** the tool result has `isError` and names Preferences

#### Scenario: An exposed token is refused [mcp_token_mode]

- **WHEN** the token file is readable by others
- **THEN** nothing is sent and the result has `isError`

#### Scenario: A huge reply does not break the stream [mcp_too_large]

- **WHEN** a reply is over the limit, and the next is normal
- **THEN** the first answers `too_large` and the second succeeds

#### Scenario: A late reply is skipped [mcp_stale_id]

- **WHEN** a reply with an older id arrives first
- **THEN** it is skipped and the matching reply is returned

### Requirement: A spawned instance is restricted and short-lived

`--spawn` SHALL start a headless instance with its own socket and state directory,
at the `--tier` given or Read, with its `--root` folders. The instance SHALL exit
when its bridge disconnects. The bridge SHALL remove the temporary directory.
`--control-exit-on-disconnect` SHALL do nothing until a client has connected.

#### Scenario: Arguments are built from the options [mcp_spawn_args]

- **WHEN** `--spawn --tier edit --root /a --root /b` is parsed
- **THEN** the child arguments carry the tier, both roots, and exit-on-disconnect

#### Scenario: The default tier is read [mcp_spawn_default]

- **WHEN** `--spawn` is given alone
- **THEN** the child runs at the read tier

#### Scenario: Bad arguments are refused [mcp_bad_args]

- **WHEN** an unknown option or tier is given
- **THEN** parsing fails

#### Scenario: The last client leaving is reported [mcp_last_client]

- **WHEN** the only client disconnects
- **THEN** the server reports it, and not while another stays connected

#### Scenario: Attach and spawn work end to end [mcp_end_to_end]

- **WHEN** a client drives `pictura --mcp` against a running instance, and with `--spawn`
- **THEN** tools answer, and the spawned instance stops when stdin closes
