# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- `ControlServer` is already platform-aware in `defaultSocketPath()`
  (`crates/pictura-app/cpp/control_server.cpp:252`), but its `XDG_RUNTIME_DIR`
  check precedes the `_WIN32` branch, so on Windows with that variable set it
  still returns a path-shaped string. The self-test bypasses the function
  anyway with an explicit `QTemporaryDir` path.
- The self-test runs the control block unconditionally, and
  `mcp_control_socket_mode` is already skipped on Windows
  (`crates/pictura-app/cpp/selftest_control.cpp:115`). Nothing else in the block
  is POSIX-specific.
- The self-test also uses the temporary directory for its `raster.png` fixture,
  so the directory cannot simply be dropped on Windows.
- The Windows branches of `ControlServer` (`tokenPath()` under `%TEMP%`, the
  `QFile` token write, `shutdown()`) have never executed: the self-test always
  passed an explicit path, and the Qt suites that construct a server run only in
  the Linux `ctest` job. The first execution is part of this change's acceptance.
- `scripts/test-report.sh` runs two self-test invocations concurrently on Linux;
  the `windows` job runs its three sequentially.
- The `windows` job never runs for a normal PR: its `pull_request.paths` list
  excludes `crates/**`, its job `if:` only admits the release-please head, and a
  `workflow_dispatch` requires a tag and checks out the tag rather than a branch.

## Goals / Non-Goals

**Goals:**
- Give the self-test a Windows endpoint that `QLocalServer::listen` accepts, and
  make the endpoint unique per running self-test process.
- Keep the POSIX endpoint and the mode check exactly as they are.

**Non-Goals:**
- Changing `--control-socket` path handling or the POSIX endpoint.
- Making the existing CTest suites Windows-safe (they are Linux-only today; the
  same latent path bug exists there).
- Adding a permanent Windows job to ordinary PRs; the Windows run for this PR is
  a temporary, reverted edit.

## Decisions

### Derive the endpoint in the self-test, not inside `ControlServer`

The self-test is the only caller that hardcodes a path; `main.cpp` already routes
through `defaultSocketPath()`. Fixing the caller is the smallest diff.

*Alternative considered:* have `ControlServer`'s constructor map any path-like
argument to a pipe name on Windows, which would also repair the latent bug in the
CTest suites. Rejected: it needs a heuristic to tell a bare pipe name from a path
(`defaultSocketPath()` already yields a bare name, so a blanket transform would
mangle it), and no test exercises the mapping on Windows. If Windows CTest is
wanted later, that is the place to revisit.

*Alternative considered:* pass an empty path on Windows so the self-test exercises
`defaultSocketPath()` directly. Rejected: that name is scoped per user, not per
process, so two concurrent self-tests would share it — which the spec scenario
forbids.

### Move the `_WIN32` branch above the `XDG_RUNTIME_DIR` check

`defaultSocketPath()` returns `$XDG_RUNTIME_DIR/pictura-control.sock` whenever
that variable is set, on every platform. Composing the self-test name from that
result on Windows would carry `/` (and `\`/`:` for a native path), violating the
requirement that a Windows endpoint be a bare pipe name and making the
`socket=` assertion unverifiable. `XDG_RUNTIME_DIR` is a POSIX convention, so the
`_WIN32` branch moves above it and Windows always returns the temp-scoped
`pictura-control-<hash>` name. The POSIX ordering is unchanged.

*Trade-off:* this also fixes the `--control` default on Windows, which today can
be path-shaped when `XDG_RUNTIME_DIR` happens to be set. That is the same latent
bug seen from the production side, not new behavior invented for the self-test.

### Windows name: `defaultSocketPath()` + `-` + process id

Reuses the production scoping (the same per-user base a real control server
uses), stays a bare pipe name after the reorder above, and is unique because two
live processes cannot share a pid. PID reuse after a process exits is harmless:
the pipe dies with the process, `listen()` calls `removeServer`, and a stale
token file is opened with `Truncate`.

*Alternative considered:* a random suffix. Same uniqueness, but less useful when
reading a CI log; a pid also matches the issue's suggestion.

*Alternative considered:* the `QTemporaryDir` basename as the suffix. Unique, but
it needs an extra include to strip the path and hides where the value came from.

### POSIX unchanged

The private temporary directory keeps the socket and its `.token` file isolated
from the per-user runtime directory, and the mode check stats the socket inside
it. No reason to touch it.

### Concurrency is a property, not an in-process check

One self-test process cannot observe another's endpoint, and a new
`runSelfTest()` check would break the append-only budget (507) and rule 11. The
requirement is expressed as a spec scenario; verification compares the two runs'
`socket=` values and requires them to differ. On Windows a shared name does not
fail `listen()` (Qt allows two servers on one pipe and hands each connection to
either), so the observable collision symptom is an `unauthorized` reply carrying
the other process's token, not code 463.

## Risks / Trade-offs

- **The Windows `ControlServer` branches run for the first time** → the full
  self-test summary (all three Windows steps) is the acceptance, so a failure in
  the token write or `shutdown()` surfaces as a named check code rather than
  passing on the `socket=` line alone.
- **A stale `%TEMP%/<pipe>.token` can outlive a hard crash** → the per-process
  name keeps it out of other runs' way, and `QFile::open(Truncate)` replaces it.
- **No local execution of the Windows branch** → the `windows` job is enabled for
  this PR's head branch with two edits marked `# TEMPORARY (#986)` (add
  `crates/**` to `pull_request.paths`; allow head ref
  `fix/986-windows-control-server` in the job `if:`). That run is the enforcement;
  both edits are reverted before merge so `.github/workflows/windows.yml` matches
  the base.
- **`--control-socket <filesystem path>` on Windows still fails to listen** → out
  of scope, already true before this change; recorded in the proposal as a known
  divergence.

## Migration Plan

None. No persisted state, protocol, or artifact changes; rollback is reverting
the self-test and `defaultSocketPath()` reorder.
