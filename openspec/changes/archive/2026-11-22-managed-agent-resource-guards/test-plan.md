# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mag_limits` | Render defaults and overrides, check fixture cgroup files, and start with invalid limits. | The unit contains all limits, invalid enrollment stops before exec, and a valid launch exports the canonical guard. | `check-agent-cpu-budget::test_unit`, `check-agent-cpu-budget::test_resource_limits`, `check-agent-cpu-budget::test_launcher_exports_canonical_policy` |
| `mag_host` | Launch local OpenCode in two worktrees, check shell and cache PIDs with the status command. | All report one slice and finite CPU, memory and swap limits. | manual: requires a configured host user manager, local OpenCode, and sccache. |
| `mag_queue` | Hold the queue with a harmless first job, submit a second with disabling and ownership markers, and exercise verified nesting. | The second reports or waits for the holder and the nested job proceeds inside the same slot. | `check-agent-build::test_queue`, `check-agent-build::test_nested_job` |
| `mag_cancel` | Cancel one harmless job and simulate a disappeared client with a live job unit. | Cancellation stops the unit, and a live previous unit keeps the queue occupied. | `check-agent-build::test_cancellation`, `check-agent-build::test_abandoned_job` |
| `mag_host_queue` | Start two harmless jobs in distinct enrolled worktrees and cancel the first while it has a child. | All processes from the first unit disappear before the second begins. | manual: requires an enrolled systemd user manager with cgroup delegation. |
| `mag_policy_failure` | Invoke the actual adapter with failing, missing or unexpected mock policy outcomes. | The hook throws before shell execution. | `check-agent-command-guard::test_opencode_policy_failure` |
| `mag_policy_segment` | Submit an approved command and a compound command with an unwrapped heavy segment. | The guard allows the former and refuses the latter and common alternate spellings. | `check-agent-command-guard::test_managed_policy_segments` |
| `mag_host_opencode` | Launch OpenCode on a stale linked worktree using the canonical global plugin. Submit a harmless command through the runner and request a bare build. | The probe succeeds through the runner and the bare build is refused before execution. | manual: requires a configured local OpenCode executor and alternate checkout. |
| `cb_wrap_budget` | Run a harmless command through the portable wrapper outside the slice with the budget marker unset. | The command inherits the budget and keeps its working directory and exit status. | `check-agent-build::test_portable_wrapper_budget` |
| `cb_agent_refuse` | Submit bare heavy, wrapped, format-only, and ordinary commands in both input forms. Append a bare command after a wrapper. | Bare heavy commands and unwrapped siblings are denied. | `heavy_command_policy::self_check` |
| `cb_policy_git_root` | Refuse a command from an unrelated checkout. | The legacy hint names the policy checkout's wrapper. | `heavy_command_policy::self_check` |
| `cb_policy_quotes` | Put separators inside quoted text, then add an unquoted heavy sibling. | Only the real sibling causes refusal. | `heavy_command_policy::self_check` |
