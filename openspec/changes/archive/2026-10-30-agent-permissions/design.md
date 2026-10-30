# Design: Live control sessions and agent permissions

## Ownership

`frame.h` is at its size ceiling, and the server does not need to be a frame
member. `AgentControl` is a `QObject` parented to the window, created in `main()`
for an interactive launch. `saveSession()` loads the store and patches it, so
`AgentControl` can write its own keys without the frame knowing them.

Bare `--control` keeps its process-wide Discard policy in `main()`: such a process
has no person relying on prompts. A live session never touches those globals.

## Request gate

`handleLine` ends in `serve()`:

1. `admit()`: tier, command allowlist, dirty `close`/`revert` (`unsaved`), and the
   human-busy gate.
2. For `document` `open`, `save`, `save_as`: canonicalize the target and call the
   pure `judgePath()`.
3. Refuse, park for approval, or run.

Parking stops `pump()`, so later requests wait and order holds. After Allow,
every check runs again; only the approved target skips approval.

## Paths

`pathProblem` still answers malformed spellings as `invalid_param`. Otherwise the
target is canonicalized (the parent plus leaf for a write), so a symlinked
directory is judged by where it lands. Roots match at a separator boundary, and
case-insensitively on Windows and macOS. The state directory and the endpoint
directory are denied in every mode.

ponytail: a same-user race between check and use, and hardlinks, are out of
reach of a path check. So is an agent that also has a shell.

## Approval modes

`known` is a file the agent made, or one the person approved, this session.

| Mode | Asks when |
|---|---|
| Manual | outside the roots, or overwriting |
| Auto-approve | as Manual, unless `known` |
| Allow all | never |

Headless cannot ask, so Ask becomes Refuse. `--control-approval allow-all` opts out.

## Live input

`key` and window `pointer` are Admin, so agents can't reach the permission
controls. `pointer` in image space is Edit.

## Deferred open

Over a socket, `open` calls `startOpenDocumentAtPath(path, false)` and parks until
`documentOpenFinished`. `interactive = false` raises no dialog, so no `ControlScope`
is needed and a person's dialog is left alone. In-process `dispatch()` still blocks.
