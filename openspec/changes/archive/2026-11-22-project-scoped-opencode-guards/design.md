## Context

OpenCode 1.19 loads repository plugins and lets a configuration hook select an absolute shell executable. Its shell tool checks permissions before invoking that executable. Pictura already provides a systemd launcher, dedicated cache and shared verification queue.

## Decisions

- Use `PICTURA_OPENCODE_MANAGED=1` in Pictura's existing user environment file as an explicit project opt-in. Verify the current directory belongs to the plugin checkout's Git common directory. Resolve the canonical checkout from that common directory so linked worktrees share the same scripts. Leave the default OpenCode shell and the existing full-executor enrollment mode usable.
- Configure an executable Bash entry point in the OpenCode configuration hook. Keep original tool arguments intact. Pre-execution hooks guard against an incomplete or overridden shell selection; the shell entry point checks actual slice limits through the existing launcher.
- Run all selected shell invocations inside the slice. Feed Bash command arguments unchanged to that shell. Classify recognized heavy commands with the managed command policy and submit them through the existing queue. Other commands run inside the same resource-limited slice. An already approved queue invocation can enter through that runner directly.
- Prove configuration scope, the launch contract, and queue entry with harmless fixtures. The installed OpenCode and real delegated user slice provide the final smoke boundary.
