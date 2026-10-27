# Proposal

## Why

The `windows` release job fails the application self-test with
`pictura self-test: FAIL core mcp_control_server 463 control server listen`,
blocking the release PR (#790). The self-test builds a control endpoint from a
`QTemporaryDir` path (`<temp>/control.sock`). That is a Unix-domain socket path;
on Windows a `QLocalServer` name is a machine-global named pipe, so `listen()`
fails and the first control check fails before any other control check runs. The
production path already routes through `ControlServer::defaultSocketPath()`;
only the self-test bypasses it, and the Windows branch of that function is
shadowed by the `XDG_RUNTIME_DIR` check.

## What Changes

- Derive the self-test control endpoint per platform: the private temp-dir Unix
  socket on POSIX (unchanged), and on Windows a pipe-safe, per-process name
  built from `ControlServer::defaultSocketPath()` plus the process id.
- Move the `_WIN32` branch of `ControlServer::defaultSocketPath()` above the
  `XDG_RUNTIME_DIR` check, so a Windows call always returns the temp-scoped bare
  pipe name (`pictura-control-<hash>`) and never a path-shaped string; the POSIX
  ordering is unchanged.
- Use the derived name for both the `QLocalServer` and every `QLocalSocket`
  connect in the block (already a single variable), and make the pass line
  report `server.socketPath()`.
- Add an OpenSpec scenario (a property, not a new `runSelfTest()` check: the
  `ST_BEGIN` budget is exactly 507 and AGENTS.md rule 11 forbids growth) that
  requires two concurrent self-tests to use distinct endpoint names, with each
  client answered by the process that started it.
- Leave the `mcp_control_socket_mode` Windows skip as it is; every other control
  check runs unchanged on Windows.

No change to `--control-socket`, the protocol, the token scheme, or POSIX
behavior. The `--control` default is unchanged on POSIX; on Windows it is now
always a pipe name. No new Qt or OS dependency.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `interop/agentic-control`: three requirements change.
  - "Verification of the control server" becomes platform-appropriate: a
    temp-dir Unix-domain socket on POSIX, a per-process named pipe on Windows,
    unique per self-test process so concurrent runs use distinct endpoints.
  - "Opt-in control mode over a local socket" scopes the Unix-socket default to
    POSIX and names the Windows default endpoint a machine-local named pipe.
  - "Local-only security" scopes the Unix-domain-socket mode-`0700` clause to
    POSIX and records the Windows endpoint as a machine-local named pipe.

## Impact

- `crates/pictura-app/cpp/selftest_control.cpp` — endpoint derivation, and the
  first execution of `ControlServer::tokenPath()`, the `QFile` token write, and
  `shutdown()` on Windows. Acceptance is the full self-test summary
  (`SUMMARY passed=<n> failed=0`), not only the `socket=` line.
- `crates/pictura-app/cpp/control_server.cpp` — move the `_WIN32` branch of
  `defaultSocketPath()` above the `XDG_RUNTIME_DIR` check.
- `openspec/specs/interop/agentic-control/spec.md` — three modified
  requirements (delta in this change).
- The existing `meta/windows-release-build` requirement ("SHALL pass its
  headless self-test on Windows") already demands this outcome; its text is
  unchanged.
- Verification: the `windows` job is deliberately enabled for this PR's head
  branch with a temporary `# TEMPORARY (#986)` edit, and its three self-test
  steps are the execution of the Windows branch; the edit is reverted before
  merge. Linux CI (`lint`, `test`, `qt-headless`, `guards`, `oracles`) covers
  the POSIX branch.

Known existing divergence deliberately left out of scope (not introduced by this
change, and not observable in any current test): a `--control-socket` filesystem
path still fails to listen on Windows.
