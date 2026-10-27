# Tasks

All implementation and verification happens through CI; no local build, test, or
gate runs.

## 1. Endpoint derivation

- [x] 1.1 In `crates/pictura-app/cpp/control_server.cpp`, move the `_WIN32`
  branch of `defaultSocketPath()` above the `XDG_RUNTIME_DIR` check so a Windows
  call always returns the temp-scoped bare pipe name. Verify: the PR's
  temporarily enabled `windows` job compiles the C++ shell.
- [x] 1.2 In `crates/pictura-app/cpp/selftest_control.cpp`, derive `socketPath`
  per platform in `runControlChecks`: on Windows
  `pictura::ControlServer::defaultSocketPath()` plus `-` plus
  `QCoreApplication::applicationPid()` (composed with `QString::arg`), on POSIX
  the unchanged `dir.path() + "/control.sock"`. Pass that one value to the
  `ControlServer` and to both `connectToServer` calls. Verify: the PR's `windows`
  self-test steps report `mcp_control_server` PASS.
- [x] 1.3 Add `#include <QtCore/QCoreApplication>` and keep
  `<QtCore/QTemporaryDir>` (still used by the fixture raster and the POSIX
  endpoint). Verify: the PR's `windows` job and the `qt-headless` job build the
  shell.
- [x] 1.4 Change the `mcp_control_server` pass line to print
  `server.socketPath()` instead of the local variable, so the log names the
  endpoint the server bound. Verify: the `socket=` line in the PR's `windows`
  self-test log is the value checked by 3.2.
- [x] 1.5 Add no `ST_BEGIN`. Verify: the PR's `guards` job runs
  `check-selftest-budget.sh` at 507/507 and `check-file-size.sh` passes.

## 2. Temporarily enable the Windows job for this PR

- [x] 2.1 In `.github/workflows/windows.yml`, add `crates/**` to
  `pull_request.paths` and extend the job `if:` to allow head ref
  `fix/986-windows-control-server`; mark both edits `# TEMPORARY (#986)`.
- [ ] 2.2 After 3.2 and 3.3 pass, revert both edits and verify
  `git diff origin/master -- .github/workflows/windows.yml` is empty, so the
  workflow file matches the base.

## 3. Verification (CI-only)

- [ ] 3.1 The PR's `lint`, `test`, `qt-headless` (both self-test steps, CTest,
  Qt test report), `guards`, and `oracles` jobs pass. POSIX: the
  `mcp_control_socket_mode` self-test check still reports a user-only mode.
- [ ] 3.2 The temporarily enabled `windows` job (staged, portable archive, and
  installed binary): all three self-test steps print
  `SUMMARY passed=<n> failed=0` and `mcp_control_server` reports `socket=` with a
  value containing no `/`, `\`, or `:`.
- [ ] 3.3 Reviewer check (no CI job starts concurrent Windows self-tests):
  compare the `socket=` values across two concurrent
  `pictura.exe --headless --self-test` runs. They must differ, and each client
  must be answered by the process that started it; a shared name surfaces as
  `unauthorized` (the other server's token), not code 463.
- [ ] 3.4 The PR's `guards` job passes `check-test-plan.py`,
  `check-file-size.sh`, `check-selftest-budget.sh` (507/507),
  `openspec validate --all --strict`, and, after archiving,
  `check-openspec-archived.sh`.

## 4. Archive

- [ ] 4.1 Run `openspec archive fix-windows-control-selftest-endpoint` and commit
  the synced `interop/agentic-control` spec in the same pull request. Verify: the
  PR's `guards` job `check-openspec-archived.sh` passes.
