# Proposal: Close the remaining security-limit gaps

## Why

#646 asked for safe paths and decompression-bomb guards. Most of that shipped
with #844 (`harden-untrusted-input`). Three gaps remain:

- **Embedded PNG colour profiles.** The profile probe inflates a PNG `iCCP`
  chunk with no limit, so a small file can claim gigabytes before the
  document opens. A chunk whose name ends at its last byte also panics on an
  out-of-range slice.
- **Large Ghostscript renders.** Ghostscript keeps the whole page bitmap in
  memory. A large render runs into the 8 GiB address-space limit and fails,
  when Ghostscript could have rendered it in bands.
- **Control-socket paths.** The session token limits `document` `open` and
  `save_as` to the same user, but the server takes any path string,
  including a relative path (resolved against the app's working directory),
  `..` components, control characters, and a final symlink on save.

## What changes

- The PNG profile probe inflates at most 16 MiB and never slices past the
  chunk. A profile over the limit is reported as present but unreadable.
- Ghostscript raster renders pass `-dMaxBitmap` (512 MiB). Above that,
  Ghostscript renders the page in bands instead of one full-page bitmap.
- The control server's `document` `open`, `save_as`, and `save` with a path
  refuse a path that is not absolute, holds a control character, or has a
  `..` component. A save also refuses a path whose final component is a
  symlink. Paths are not confined to a root.
- Parity register row XC-6 points at #844 and #646 and lists what is still
  open: the out-of-process sandbox and fuzzing.

## Capabilities

- Modified (added requirements): `codec/untrusted-input-limits`,
  `interop/agentic-control`.

## Impact

- `pictura-codec`: `profile_probe.rs`, `ghostscript.rs`.
- `pictura-app`: `control_server.cpp`; `tst_untrusted_input`.
- Docs: `docs/dev/doc-code-parity-gaps.md` (XC-6),
  `docs/dev/mcp-agentic-control-plan.md` (path rules), `docs/dev/STATE.md`,
  `ROADMAP.md`.
- No new dependencies.
