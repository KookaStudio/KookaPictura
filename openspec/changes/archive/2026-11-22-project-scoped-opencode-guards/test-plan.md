# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mag_project_shell` | Set the opt-in for the canonical repo and initialize the adapter in a linked worktree. | The configured executable comes from the canonical checkout and the tool command is unchanged. | `check-agent-command-guard::test_opencode_project_shell` |
| `mag_project_isolation` | Load the adapter in an unrelated repository, with opt-in unset, and inside an explicit full-executor launch. | The user's existing shell setting stays intact in each case. | `check-agent-command-guard::test_opencode_project_shell` |
| `mag_project_fail_closed` | Remove or override the configured wrapper, then exercise a wrapper launch failure. | The adapter or shell entry point reports the failure and never runs the payload. | `check-agent-command-guard::test_opencode_project_shell`, `check-agent-cpu-budget::test_opencode_shell_refuses_unavailable_slice` |
| `mag_project_queue` | Exercise the launcher with harmless command strings representing light shell work, heavy work, and an invocation already using the queue. | Each uses the shared launcher with unchanged arguments and only heavy unqueued commands get a queue wrapper. | `check-agent-cpu-budget::test_opencode_shell_routing` |
| `mag_project_host` | Start plain OpenCode from two linked worktrees and inspect managed shell status. Queue a short sleep and a harmless Cargo version probe. | Limits match and probes serialize. A separate project retains its original shell. | manual: needs an installed OpenCode with a configured systemd user slice and real shell tool calls from linked worktrees. |
