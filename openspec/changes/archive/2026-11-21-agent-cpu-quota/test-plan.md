# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `acq_unit` | Generate a unit for a fixture with 16 CPUs at the default, a valid override, and an invalid override. | The unit uses 1280% or 800%. The invalid value is rejected. | `check-agent-cpu-budget::test_unit` |
| `acq_aggregate` | Under a temporary low quota, run two short CPU-time-bounded commands from separate worktrees through shell, Python and JavaScript. Read the parent's `cpu.stat` while the scopes are open. | Both commands resolve the same parent path and its throttling counter advances. | manual: requires an active Linux systemd user manager with CPU delegation |
| `acq_launch` | Construct a scope command with a path containing spaces and an argument containing a literal dollar sign. | The child gets the same argument vector. The systemd launch selects the user slice and disables environment expansion. | `check-agent-cpu-budget::test_scope_command` |
| `acq_refuse` | Exercise fixture cgroups with missing v2 membership, a similar slice name, an absent quota file, unlimited or invalid settings, and a working quota. Attempt to launch a program in a scope with unverified membership. | Unsafe or unavailable enrollment fails before execution. A valid ancestor and quota can be reported. | `check-agent-cpu-budget::test_quota_validation`, `check-agent-cpu-budget::test_unenrolled_execution_is_refused` |
| `acq_harnesses` | From OpenCode, Claude Code and a standalone or app-server-backed Codex session, run the status command in a real tool shell. Repeat for an approved Codex command. | The actual tool shells report the same quota ancestor. | manual: requires configured interactive harness sessions and approvals |
| `cb_sccache_server` | Run the legacy affinity self-check against a fake server process outside the agent slice. | The fake server's leader mask narrows. | `cpu-budget::self_check` |
| `acq_cache` | Build two cache environments with conflicting ambient cache and runtime settings. | Both use the same dedicated socket and disk-cache directory. | `check-agent-cpu-budget::test_cache_environment` |
| `acq_cache_start` | Simulate a cold dedicated socket and a socket with a peer outside the slice. | The launcher starts the server for the cold socket and rejects the unrelated peer. | `check-agent-cpu-budget::test_cache_startup` |
| `acq_cache_affinity` | Probe legacy budget helper paths inside and outside the agent slice. | The in-slice helper retains job setup and leaves sccache affinity alone. Legacy discovery ignores enrolled servers. | `check-agent-cpu-budget::test_legacy_cache_boundary` |
| `acq_cache_worker` | Compile a small C program through the dedicated sccache socket. Observe the server and compiler with the status command. | Both run inside the agent slice. A default server, when present, keeps its original placement. | manual: requires a live delegated slice and a Unix-socket sccache installation |
| `acq_lock` | Compare the default lock root under two runtime and temporary directories, then start two wrappers under a private fixture lock root with different runtime and temporary directories. | The default path matches and the second wrapper waits behind the first. | `check-agent-cpu-budget::test_lock_path` and the queue regression in `scripts/check-build-lock.sh` |

## Live evidence

- At a 1% quota on a 16-logical-CPU Linux workstation, Bash launched Python
  and JavaScript workers from separate worktrees. Both reported
  `cpu.max=16000 100000` and the same cgroup. The parent's `nr_throttled`
  counter advanced from 46 to 58.
- A compile through the dedicated sccache endpoint ran its server and
  compiler child in the same slice.
- All three CLIs started in the slice. Codex App Server command RPCs using
  workspace-write and full-access policies reported that same slice.
  Interactive OpenCode and Claude Code tool launches and the configured
  Codex approval path remain a reviewer check.
