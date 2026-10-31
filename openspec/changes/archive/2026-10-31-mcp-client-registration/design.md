# Design: Client registration

## Clients

| Client | How | Where |
|---|---|---|
| Claude Code | `claude mcp add --scope user` | its CLI, run asynchronously |
| Cursor | merge | `~/.cursor/mcp.json` |
| Claude Desktop | merge | macOS Application Support, Windows `%APPDATA%`; no official Linux build |

The CLI is looked up on `PATH` and in `~/.local/bin`, because an app started from a
desktop launcher often lacks a shell's `PATH`. If it is not found, the row says so
and the person uses the snippet.

## Merge

`planRegister` is pure: it returns the file's bytes before and after, or a problem.
Nothing is written until the person accepts. Refused without writing: invalid JSON
(JSONC comments included), a non-object root, or a non-object `mcpServers`. Other
keys survive, though `QJsonObject` sorts them, so the person sees that in the change.

`applyChange` resolves a symlink so the link survives, takes a timestamped backup
beside the file, and writes through `QSaveFile`. An existing `pictura` entry that
differs asks again, naming the replacement.

## Claude Desktop on Windows

ponytail: a Microsoft Store install reads a packaged copy under `%LOCALAPPDATA%`, not
`%APPDATA%`, so registration there can land in a file that app ignores.
