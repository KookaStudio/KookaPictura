# Test Plan

Every row comes from a scenario in
`openspec/changes/fix-windows-control-selftest-endpoint/specs/interop/agentic-control/spec.md`
(the requirements "Verification of the control server", "Opt-in control mode over
a local socket", and "Local-only security").

The three verification scenarios cover behavior this change touches. The six
opt-in and security scenarios are pre-existing requirements reproduced verbatim
in the delta (only the POSIX/Windows scope is stated explicitly), so their
behavior is unchanged. The changed branch lives inside `runControlChecks`, which
no Qt Test or Rust test can reach, and the Windows branch needs a Windows runner;
the POSIX endpoint stays covered by the existing self-test run.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| The control block proves the socket and core methods | Read the `mcp_control_*` lines and the closing summary of the `qt-headless` and `windows` self-test runs. | Every `mcp_control_*` check reports PASS (or SKIP where the platform has no analogue) and the summary reports `failed=0`. | manual: the block is the application self-test, not a Qt Test or Rust test; the `qt-headless` self-test steps run it on Linux and the `windows` self-test steps run it on Windows |
| Windows listens on a pipe-safe endpoint | Read the `mcp_control_server` line in the three `windows` self-test logs. | `mcp_control_server` reports PASS with a `socket=` value that carries no `/`, `\`, or `:`, and the summary reports `failed=0`. | manual: the `windows` job is temporarily enabled for this PR's head branch; its three self-test steps run the staged, portable, and installed binary |
| Concurrent self-tests use distinct endpoints | Compare the `socket=` values from two concurrent `pictura.exe --headless --self-test` runs. | The two endpoint names differ and each client is answered by the process that started it. A shared name surfaces as `unauthorized` (the other server's token), not code 463. | manual: no CI job starts concurrent Windows self-tests; the reviewer runs the pair once and compares the logged names |
| Control mode starts and reports its socket | Run `scripts/verify-control.sh`. | The server starts, prints its endpoint, and answers `status`. | manual: unchanged POSIX behavior; the `mcp_control_server` self-test check and `scripts/verify-control.sh` cover it |
| No control server without the flag | Read the existing self-test run, which never passes `--control`. | No control endpoint is created and the self-test passes. | manual: unchanged behavior; the `qt-headless` Control e2e step covers the flag path and the self-test covers the default |
| A custom socket path is honored | Run `scripts/verify-control.sh`, which passes `--control-socket`. | The server listens on the supplied path. | manual: unchanged POSIX behavior; `scripts/verify-control.sh` covers `--control-socket` |
| The socket is user-only | Read the `mcp_control_socket_mode` line in the `qt-headless` self-test log. | The check reports a user-only mode (`mode=700`). | manual: unchanged POSIX behavior; the `mcp_control_socket_mode` self-test check asserts it |
| No eval method exists | Read the `mcp_control_unknown_method` line in the self-test log. | The check passes with `unknown_method`. | manual: unchanged behavior; the `mcp_control_unknown_method` self-test check covers it |
| An oversized request is rejected | Read the `mcp_control_oversized` line in the self-test log. | The check passes with `bad_request`. | manual: unchanged behavior; the `mcp_control_oversized` self-test check covers it |

Automated tests the plan names: none. AGENTS.md requires a runnable check for
new non-trivial logic, but the only new logic is the platform branch inside
`runControlChecks`; it cannot be reached from a Qt Test, and the Windows half
requires a Windows runner. The `windows` job, temporarily enabled on this PR, is
the enforcement point.
