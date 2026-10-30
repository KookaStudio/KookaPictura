## MODIFIED Requirements

### Requirement: Opt-in control mode over a local socket

The application SHALL start a control server only when invoked with `--control`,
or when the person has enabled agent control. On POSIX the `--control` server
SHALL listen on a per-user Unix-domain socket whose default path is
`$XDG_RUNTIME_DIR/pictura-control.sock`; on Windows it SHALL listen on a
machine-local named pipe. The resolved endpoint SHALL be printed to stderr and
`--control-socket ENDPOINT` SHALL override it. `--state-home DIR` SHALL isolate
the session store to `DIR`. `--headless --control` SHALL run offscreen without a
display. With neither, no socket SHALL be created and startup, the self-test, and
normal runs SHALL be unchanged.

#### Scenario: Control mode starts and reports its socket

- **WHEN** the app is launched with `--control`
- **THEN** it creates the socket, prints `pictura control: <path>` to stderr, and
  a client that connects receives a `status` response

#### Scenario: No control server without the flag

- **WHEN** the app is launched without `--control` and agent control is off
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
only under `--control` or while agent control is enabled. A line or buffered
request larger than a fixed bound SHALL be rejected with `bad_request` rather
than growing without limit.

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

### Requirement: Control document paths are validated

The control server's `document` method SHALL refuse with `invalid_param`,
without opening or writing anything, an `open`, `save_as`, or `save` path
that is not absolute, contains a character below U+0020 or U+007F, or has a
`..` component. A `save_as`, or a `save` with a path, SHALL also refuse a
path whose final component is a symbolic link, and a `save` with no path
SHALL refuse the same way when the active document's path is one. Under bare
`--control` any other absolute path SHALL be accepted, so that mode SHALL NOT
confine paths to a root directory; a restricted session follows
`interop/agent-permissions`.

#### Scenario: Ambiguous paths are refused [ac_path_refused]

- **WHEN** a client opens a relative path, a path holding a newline, and an
  absolute path with a `..` component
- **THEN** each is answered `invalid_param` and no document opens

#### Scenario: A save over a symlink is refused [ac_path_save_link]

- **WHEN** a client calls `save_as` on a symlink that points at another file
- **THEN** the response is `invalid_param` and the link's target is unchanged

#### Scenario: A pathless save through an opened symlink is refused [ac_path_save_active_link]

- **WHEN** a client opens a symlink to a document, then calls `save` with no path
- **THEN** the response is `invalid_param` and the link's target is unchanged

#### Scenario: A plain absolute path still works [ac_path_accepted]

- **WHEN** a client calls `save_as` on a new absolute path in a temp directory,
  then `open` on it
- **THEN** both succeed and the file exists
