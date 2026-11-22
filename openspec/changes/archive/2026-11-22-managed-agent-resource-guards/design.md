## Context

The project already has a Linux systemd launcher with a private sccache socket and portable affinity and lock wrappers. Worktree-local hooks can lag the canonical policy. The host setup remains an explicit owner action.

## Decisions

- Reuse the common systemd slice and validate the actual cgroup files before launching. Generate conservative memory and swap defaults with explicit host knobs. The launch environment points OpenCode at the initiating checkout's policy. Install the same plugin at the OpenCode server's global plugin location for stale worktrees.
- Provide one Linux runner for managed gates. A stable host lock orders admission and a uniquely named systemd unit covers the job and its descendants. Revalidate previous job state before admitting the next job if the client ended early. Verify nested membership using the kernel cgroup and the current job record. Retain the portable behavior for ordinary shells.
- Evaluate each shell segment before allowing it. Recognize and reject common alternate spellings of heavyweight commands in managed OpenCode. Verify the exact canonical runner as the managed entry point. Shell policy covers common mistakes and the kernel caps the resources of the enrolled workload.
- Print queue ownership and job identity and retain memory-event evidence with the job where available.

## Risks and Tradeoffs

- Starting with a single slot favors desktop responsiveness. The host owner can tune limits using normal work after enrollment.
- CI can test command admission, queue state, and signal handling with harmless fixtures. Installing and verifying a real host user slice needs the machine owner's session.
