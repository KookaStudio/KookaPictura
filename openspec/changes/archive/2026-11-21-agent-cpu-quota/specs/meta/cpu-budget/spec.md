## ADDED Requirements

### Requirement: Enrolled coding agents share one user CPU quota

The repository SHALL provide an explicit Linux systemd user-slice setup for
Codex, OpenCode, and Claude Code. Setup SHALL default to 80% of the online
logical CPUs and SHALL accept a percentage from 1 to 100. There SHALL be one
quota on the parent slice across all enrolled sessions and worktrees. No user
unit SHALL be installed or enabled by routine repository use. Processes
outside the slice SHALL keep their existing CPU access.

#### Scenario: Setup generates one configurable slice unit [acq_unit]

- **WHEN** a 16-logical-CPU host requests the unit definition at the default
  percentage or at 50%
- **THEN** it specifies `CPUQuota=1280%` or `CPUQuota=800%` respectively, writes
  the definition to standard output, and leaves installation to the user

#### Scenario: Two sessions consume the same CPU quota [acq_aggregate]

- **WHEN** enrolled commands from separate worktrees run concurrent shell,
  JavaScript, or Python child processes under a low test quota
- **THEN** the children share the quota ancestor and the parent's CPU
  accounting records their combined usage and throttling

### Requirement: Launch verifies the quota before running work

The launcher SHALL use a systemd user scope and SHALL keep the working
directory, environment, terminal, arguments, and command status usable by the
harness. It SHALL check actual cgroup membership and a finite parent quota
before executing the target, including when already in the enrolled slice.
It SHALL give a nonzero result when it cannot establish that enrollment.
A status command SHALL expose the effective slice path and configured quota
from a process running inside it.

#### Scenario: The launcher uses the shared slice and preserves arguments [acq_launch]

- **WHEN** an opted-in launcher starts a supported CLI or another command
- **THEN** the invocation targets the one named user slice and delivers the
  command's arguments without shell interpretation or variable expansion

#### Scenario: Missing or ineffective enrollment is refused [acq_refuse]

- **WHEN** the slice is missing from the command's ancestry, its quota is
  missing or unlimited, or systemd cannot put the command in the slice
- **THEN** the launcher reports failure before running the target

#### Scenario: Each harness executes inside the quota [acq_harnesses]

- **WHEN** OpenCode, Claude Code, and an enrolled Codex CLI or app server execute
  local shell commands from their normal entry points
- **THEN** the actual shell reports the same quota ancestor with the status
  command, including the regular and approved local Codex command paths

## MODIFIED Requirements

### Requirement: A running sccache server is held to the slice

For legacy affinity runs on a platform with process-affinity control and
`/proc`, `scripts/cpu-budget.py` SHALL try to narrow the leader of an
addressable `sccache --server` outside the agent slice. This is best-effort and
SHALL silently skip inaccessible processes. For enrolled runs the launcher
SHALL select a dedicated per-user local sccache socket and disk-cache location
shared across worktrees. When sccache is installed, the host launcher SHALL
start or reuse the dedicated server and verify its membership before the
harness starts. Compiler children and local compilation fallbacks SHALL
inherit the quota. An enrolled build SHALL skip legacy affinity changes to
sccache servers.

#### Scenario: A pre-existing sccache server is narrowed [cb_sccache_server]

- **WHEN** an addressable `sccache --server` runs outside the agent slice and
  the legacy budget helper applies a narrower mask
- **THEN** the helper narrows the server's leader and does not restart it

#### Scenario: The enrolled launcher isolates the cache endpoint [acq_cache]

- **WHEN** an enrolled launch inherits a default sccache configuration or a
  private Qt runtime directory
- **THEN** it selects the same dedicated socket and cache directory as another
  enrolled worktree for that user, independent of the ambient cache endpoint

#### Scenario: The cache server is prepared before entering the sandbox [acq_cache_start]

- **WHEN** sccache is installed and the host launcher prepares an enrolled
  command
- **THEN** it starts a dedicated server when necessary and checks that its
  process belongs to the same quota slice before running the command

#### Scenario: Enrolled verification leaves legacy cache affinity alone [acq_cache_affinity]

- **WHEN** the verification budget helper runs inside the agent slice
- **THEN** it applies its job defaults and skips the legacy sccache
  affinity changes

#### Scenario: Cache misses compile inside the quota [acq_cache_worker]

- **WHEN** an enrolled cache client uses the dedicated server for a
  cache miss while a default server can also exist
- **THEN** the dedicated server and compiler are descendants of the quota
  slice and the default server remains outside
