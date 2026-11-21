## Context

The current wrapper selects affinity from the invoking process and exports
job-count variables. The build lock serializes opted-in callers and currently
uses their runtime or temporary directory. Both continue to serve their
existing roles for ordinary local verification.

## Goals / Non-Goals

**Goals**

- One opt-in, per-user CPU quota for the enrolled local harnesses and workers.
- Stable serialization for users who separately opt into the build lock.
- A way to inspect the quota from inside a real harness tool command.

**Non-Goals**

- System-wide resource limits, privileged setup, or changes to contributors'
  launchers at checkout time.
- Containment of agents allowed to reconfigure the user manager or ask an
  external service to execute work outside the slice.

## Decisions

1. Use one `pictura-agents.slice` under the systemd user manager. Generate a
   slice definition from the requested percentage and online CPU count.
   Install it explicitly as the user. Start each command in a user scope and
   validate the live cgroup v2 `cpu.max` in the child before executing it.
   Keep CPU capacity and job-count tuning independent.
2. Run OpenCode and Claude Code from the launcher. For a standalone Codex TUI
   use its no-daemon flag. For shared app-server workflows launch a local
   app server in the slice and point the client at that server's own Unix
   socket. Validate the actual shell in both execution modes. An existing
   unrelated Codex daemon stays in its current cgroup.
3. Select one dedicated sccache Unix socket in the canonical user runtime
   directory and one separate disk cache for opted-in processes. Start and
   verify the dedicated server on the host after scope entry, before a
   sandboxed client may open it. Keep the server available through idle
   periods and allow local compilation if a sandbox denies socket access.
   Enrolled verification skips legacy server affinity changes. Legacy
   verification leaves dedicated servers alone.
4. Keep the verification lock portable. Use the canonical `/run/user/<uid>`
   directory when available and a stable temporary root otherwise. A
   fixture-only environment override keeps tests independent of a real
   verification lock.

## Risks / Trade-offs

- The quota limits CPU time over a scheduling interval. RAM, disk, GPU and
  physical core reservation have independent controls.
- Local brokers and already-running services have their own cgroups.
  Verification must inspect their real command processes and include a
  dedicated sccache worker.
- Changing the configured CPU percentage means regenerating the unit.
  The warm cache for the enrolled socket is distinct from the default cache.
- A cgroup v2 user manager must delegate the CPU controller. Missing quota or
  unavailable enrollment yields an error on the opt-in startup path.
