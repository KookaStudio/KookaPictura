## Why

PR #1530 made host limits and the shared queue available through an explicit executor launcher. In #1529, plain OpenCode launches kept working outside that launcher. Pictura users need project-scoped enrollment for their usual terminal command.

## What Changes

- Add a Pictura-only opt-in for the repository OpenCode plugin.
- Configure a Bash entry point that starts ordinary shell commands inside the existing managed slice and puts recognized heavy invocations into the shared queue.
- Keep the original command visible to OpenCode's permission checks and report incomplete setup before execution.

## Capabilities

### Modified Capabilities

- `meta/managed-agent-resource-guards`: OpenCode project shell enrollment.

## Impact

Linux workstation configuration, the OpenCode 1.19 plugin, managed build commands, and developer documentation.
