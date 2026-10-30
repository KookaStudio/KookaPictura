# Design: Close the remaining security-limit gaps

## D1. The PNG profile inflate limit

`probe_png_profile` inflates through `take(MAX_PNG_PROFILE + 1)` and refuses a
result longer than `MAX_PNG_PROFILE`, as `read_bounded` does. The limit is
16 MiB. That is the most a JPEG (255 APP2 segments) can carry, and no other
container has a format limit to match. A refused profile is `present` with no
`embedded` bytes, the same as a profile that fails to parse, so the open path
already handles it by warning. The `nul + 2` slice uses `get`, so a chunk
that ends inside the keyword or method byte reads as present but unreadable.

## D2. Ghostscript banding

`png_args` adds `-dMaxBitmap=536870912`. Ghostscript renders any page bitmap
larger than this in bands, from a command list, instead of allocating the
whole page. Address space then grows with band size, not page size, and the
8 GiB `RLIMIT_AS` from #844 is no longer what limits large renders. 512 MiB
holds an RGBA page of about 11 500 × 11 500 in one buffer (an A4 page at
1 000 ppi), so ordinary renders stay unbanded and keep their speed.

ponytail: the value is fixed. A cgroup or seccomp sandbox for the child is
still not built (`security-and-sandboxing.md`, out-of-process helper).

## D3. Control document paths

A file-local `pathProblem(path, writes)` in `control_server.cpp` returns an
empty string or the reason for a refusal, and the call site answers
`invalid_param` with that reason. The checks:

1. `QDir::isAbsolutePath`. A relative path depends on the app's working
   directory, which the client does not control.
2. No code point below U+0020 and no U+007F. NUL truncates at the OS
   boundary; other control characters spoof log and dialog text.
3. No `..` component, after `QDir::fromNativeSeparators`. An absolute path
   never needs one, and it is the classic way to name a place other than the
   one that looks named.
4. For a write (`save_as`, `save` with a path): `QFileInfo::isSymLink` on the
   target refuses. The atomic rename would replace the link rather than
   follow it, but the Qt-side exporters do not all go through that path, and
   a client meaning to save over a link's target can name the target.
   `open` keeps the path as given, so a `save` with no path applies the same
   symlink check to the active path. Only that check: the other three would
   refuse a path the app opened by other means.

The symlink check is advisory. A link planted between the check and the write
is still followed, which the same-user model accepts: a process that can plant
it already has the session's file access.

No root confinement, by owner decision: the token already limits callers to
the same user, and scripted workflows open and save anywhere. Reading through
a symlink stays allowed, because a user's own folders often contain them.

## Alternatives rejected

- A `pictura-security` crate with `Limits`/`SafePath`, as the spec's module
  map proposes. Rejected because each would have one caller (AGENTS rule 3).
  The budget stays in `pictura-codec/src/budget.rs`, and the path check stays
  next to its one call site.
