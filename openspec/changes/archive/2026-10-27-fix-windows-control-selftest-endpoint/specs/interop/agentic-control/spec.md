# Spec Delta

## MODIFIED Requirements

### Requirement: Verification of the control server

The self-test SHALL include a `mcp_control` block that starts a control server on
a temporary endpoint, connects a client, and round-trips at least `status`,
`list_commands`, and a `dispatch_command`, asserting the response shapes and the
`bad_request`/`unknown_method` error paths, using the existing append-only check
codes and adding no new check. The temporary endpoint SHALL be
platform-appropriate: a Unix-domain socket inside a fresh private temporary
directory on POSIX, and a machine-local named pipe on Windows. The endpoint SHALL
be unique to the running self-test process on every platform, so two self-tests
running concurrently SHALL NOT share one, and the server and every client in the
block SHALL use the same endpoint.

#### Scenario: The control block proves the socket and core methods

- **WHEN** the self-test runs the `mcp_control` block
- **THEN** a client connects to the temporary endpoint and receives well-formed
  `status`, `list_commands`, and `dispatch_command` responses, and a malformed
  line yields `bad_request`

#### Scenario: Windows listens on a pipe-safe endpoint

- **WHEN** the self-test runs the `mcp_control` block on Windows
- **THEN** the server listens on a named pipe whose name carries no directory or
  path separator, the client connects to that same name, and the block reports
  pass

#### Scenario: Concurrent self-tests use distinct endpoints

- **WHEN** two self-test processes run the `mcp_control` block at the same time
- **THEN** each uses its own distinct endpoint name and each client is answered
  by the process that started it

### Requirement: Opt-in control mode over a local socket

The application SHALL start a control server only when invoked with `--control`.
On POSIX the server SHALL listen on a per-user Unix-domain socket whose default
path is `$XDG_RUNTIME_DIR/pictura-control.sock`; on Windows it SHALL listen on a
machine-local named pipe. The resolved endpoint SHALL be printed to stderr and
`--control-socket ENDPOINT` SHALL override it. `--state-home DIR` SHALL isolate
the session store to `DIR`. `--headless --control` SHALL run offscreen without a
display. Without `--control`, no socket SHALL be created and startup, the
self-test, and normal runs SHALL be unchanged.

#### Scenario: Control mode starts and reports its socket

- **WHEN** the app is launched with `--control`
- **THEN** it creates the socket, prints `pictura control: <path>` to stderr, and
  a client that connects receives a `status` response

#### Scenario: No control server without the flag

- **WHEN** the app is launched without `--control`
- **THEN** no socket path is printed and the default socket is not created

#### Scenario: A custom socket path is honored

- **WHEN** the app is launched with `--control --control-socket /tmp/x.sock`
- **THEN** the server listens on `/tmp/x.sock`

### Requirement: Local-only security

The endpoint SHALL NOT be a TCP listener, and the control surface SHALL NOT
include any arbitrary-code-evaluation method. On POSIX the socket SHALL be a
Unix-domain socket created with user-only access (owner read/write and no group
or other access; Qt's `UserAccessOption` creates it with mode `0700`). On Windows
the endpoint SHALL be a machine-local named pipe. The control server SHALL exist
only under `--control`. A line or buffered request larger than a fixed bound
SHALL be rejected with `bad_request` rather than growing without limit.

#### Scenario: The socket is user-only

- **WHEN** the control server creates its socket on POSIX
- **THEN** the socket's permissions grant access to the owning user only

#### Scenario: No eval method exists

- **WHEN** a client attempts a method that would evaluate arbitrary code
- **THEN** the response is `unknown_method`

#### Scenario: An oversized request is rejected

- **WHEN** a client sends an unterminated request that exceeds the fixed limit
- **THEN** the server responds with `bad_request` and does not grow the buffer
  without bound
