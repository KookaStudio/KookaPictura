# Design: Agent Control in Flatpak and Snap builds

## The bridge runs inside the sandbox

The client starts the sandboxed app with `--mcp` and speaks MCP on its stdio. The
bridge and the window are two instances of one sandboxed app, so they only need a
path they both see. Nothing outside the sandbox touches the socket.

| Build | Client runs | Shared directory |
|---|---|---|
| Flatpak | `flatpak run <FLATPAK_ID> --mcp` | `$XDG_RUNTIME_DIR/app/$FLATPAK_ID/pictura` |
| Snap | `snap run <SNAP_INSTANCE_NAME>.pictura --mcp` | `$XDG_RUNTIME_DIR/pictura`, already per snap |

`snap run` rather than `/snap/bin/...`: Fedora keeps snap binaries elsewhere, while
`snap` is on `PATH`. A client that is itself sandboxed can't run either command.

Rejected: `--filesystem=xdg-run/pictura:create`. It applies to every user of the
build and breaks the minimal-sandbox requirement, and it would still leave the
client needing a host-side bridge binary.

## The lock is a kernel lock

`QLockFile` records a PID. Flatpak instances have separate PID namespaces, so a
live holder can look dead and be taken over, and a dead holder's lock can look
alive, because sandboxed processes often get the same low PID and the directory
outlives the app. POSIX builds hold `flock` on the same path instead: it works
across namespaces and goes away with the process. The lock file stays on disk,
because unlinking a locked file lets two holders exist. Windows keeps `QLockFile`.

## Files under Flatpak

The sandbox has no home access, so a host path an agent sends doesn't exist inside
it, and portal-picked folders appear under `/run/user/<uid>/doc`. No grant is added.
A person who wants host paths opts in with `flatpak override --user
--filesystem=<folder> <id>`, which affects only them. The page says so.

## Registration stays out

The sandboxed app can't read the host's dotfiles or run its `claude` CLI, so the
registration box stays hidden and the page offers the snippet to copy.

## Proof

Offscreen Qt tests cover path and command selection and lock behaviour. Only a real
sandbox proves instances meet, so `scripts/check-sandbox-mcp.sh` runs the advertised
command against the real window in the Flatpak job and under strict Snap.
