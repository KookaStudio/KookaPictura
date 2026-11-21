## Why

Agents can exceed the intended shared CPU budget when wrapped commands inherit different affinity masks or delegate work to an existing daemon. A private Qt runtime directory can also split the build lock across worktrees.

## What Changes

- Offer a Linux systemd user slice with one configurable CPU quota for enrolled Codex, OpenCode, and Claude Code workloads across worktrees.
- Provide a launcher and an enrollment check. Document standalone and app-server Codex launches, isolated compiler caching, and how to verify aggregate throttling and roll back.
- Use one stable location for the existing opt-in build lock across temporary-directory changes.

## Capabilities

### Modified Capabilities

- `meta/cpu-budget` gets an opt-in aggregate quota and isolates compiler caching for enrolled agents.
- `verification/verification-harness` gives the opt-in lock a stable per-user location.

## Impact

The new mode requires a Linux systemd user manager with the cgroup v2 CPU controller. The existing affinity and hook workflow remains available to developers and CI without enrollment. No application or build dependency changes.
