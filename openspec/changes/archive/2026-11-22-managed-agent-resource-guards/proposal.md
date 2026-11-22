## Why

On #1529 several local agent worktrees ran verification concurrently after bypassing the opt-in build lock. The kernel recorded near-exhausted swap and a memory-allocation failure. CPU affinity alone leaves memory unbounded.

## What Changes

- Add finite CPU, memory and swap containment to the managed Linux launcher.
- Queue managed verification across worktrees and tie slot ownership to the job's lifetime.
- Run the OpenCode guard from the managed checkout and refuse commands when its policy fails.
- Document adoption and a lightweight two-worktree smoke check.

## Capabilities

### New Capabilities

- `meta/managed-agent-resource-guards`: host limits, managed verification queue, and OpenCode admission.

### Modified Capabilities

- `meta/cpu-budget`: legacy command checks apply to every segment.

## Impact

Linux local tooling, systemd user units, verification entry points, agent command policy, and development documentation.
