# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- `Profile::to_icc` (`crates/pictura-color/src/lib.rs:158`) is the **single
  chokepoint** that turns an in-memory `cmsHPROFILE` into ICC bytes for the rest
  of the engine. Everything that stores or writes profile bytes goes through it.
- Little CMS stamps wall-clock time at profile creation
  (`cmsCreateProfilePlaceholder` → `_cmsGetTime`); it exposes a creation-date
  *getter* but deliberately no setter (Little-CMS #71, Debian #814883), so the
  field can only be controlled by patching the produced bytes. The header date is
  the 12-byte `dateTimeNumber` at bytes 24..35 (ICC.1:2022 §7.2.8); the fixture
  confirms the layout (bytes 24..36 exclusive hold it, bytes 36..40 are `acsp`).
- Consumers already mask the field by hand today: `undated_icc` in the app tests
  (`crates/pictura-app/src/cxxqt_object/tests.rs:13`) and an explicit zero loop in
  `crates/pictura-color/examples/dump_adobe_rgb.rs:15-21`, which is why the
  committed `psd_icc_rgb.icc` fixture already carries zeroed date bytes. This
  change moves that masking to the producer and deletes the consumer copies.
- Loaded profiles are not affected by this path: the PSD writer re-emits a
  preserved profile's stored resource-1039 bytes verbatim, so only profiles the
  engine *synthesizes* (the built-ins) are serialized through `to_icc`.
- This repo never computes the ICC Profile ID (no `md5`/`profile_id` call
  anywhere in `crates/`, `cpp/`, or `scripts/`), so the header's ID field is
  already all zeros and zeroing the date cannot invalidate a hash.

## Goals / Non-Goals

**Goals:**

- Repeated serialization of a built-in profile is byte-identical, now and across
  processes.
- The whole class of time-flaky `to_icc()` byte comparisons is removed at its
  source, not masked test by test.
- The consumer-side masking (`undated_icc`, the example's zero loop) is deleted.
- Loaded/preserved profile bytes stay exactly as read.

**Non-Goals:**

- Changing what bytes are written for a profile loaded from a file.
- Preserving a real creation timestamp on synthesized built-ins.
- Supporting `SOURCE_DATE_EPOCH` or any environment-gated behavior.
- Adding a date getter/setter to the `Profile` API.

## Decisions

### Zero the header date inside `to_icc`, not in the tests

Patch the produced bytes at the chokepoint: after `self.0.icc()`, zero bytes
24..36 — **guarded on length** so the documented panic-free empty fallback
(`to_icc` is `self.0.icc().unwrap_or_default()`) still holds for a hypothetical
short buffer, mirroring the example's existing `if bytes.len() >= 36`.

Alternatives rejected:

- **Mask in each test** (the pattern the app tests already use via
  `undated_icc`). Rejected: it leaves the class open for every future comparison,
  duplicates the mask logic across consumers (tests *and* the example), and puts
  the fix in the consumer instead of the producer — the opposite of upstream's
  own guidance ("access the header directly").
- **Memoize the built-in profiles** so repeated constructions share one
  timestamp. Rejected: `lcms2::Profile` is `!Sync` and not `Clone` (only
  `Debug`/`Drop`/`Send`), so this would mean caching bytes and re-parsing on
  every call — more code for less coverage.
- **`SOURCE_DATE_EPOCH`-gated chokepoint** (the dasch-swiss/sipi ADR design).
  Rejected here: it adds env parsing and a branch, and determinism then depends
  on every test/CI invocation exporting the variable; nothing in this product
  consumes a built-in profile's creation time, so the split buys nothing.

### Zero rather than a fixed constant date

Zero is the same sentinel Little CMS leaves when its own time call fails, it is
what the masks already applied, and external readers (PIL/lcms2, and ImageMagick
when present) accept and name a profile with a zeroed date. A fixed constant date
(e.g. epoch) would be equally deterministic but adds a magic value for no gain.

### Zeroing is unconditional; the built-in-only invariant is the caller's

`to_icc` cannot tell a synthesized profile from a parsed one (lcms2 exposes no
date setter to scope it), so it zeroes always. This is safe because every
production caller passes a built-in (`impl_core.rs` `builtin_profile`), and the
only other serializer is the read path, which re-emits stored bytes verbatim.
The consequence — a parsed profile serialized through `to_icc` loses its original
creation date — is recorded rather than encoded, because no caller needs it; a
future need would be met with an explicit setter, not by reverting determinism.

### Delete the redundant consumer-side masking

With serialization deterministic, `undated_icc` in the app tests and the zero
loop in the example are no-ops. Remove both so the app tests become plain
byte-equality checks against a freshly serialized profile (a strictly stronger
assertion) and the example is a bare `to_icc` dump.

## Risks / Trade-offs

- **A built-in profile embedded on Assign/Convert loses a real creation date.** →
  Accepted: the built-ins are synthesized at runtime, so their "creation time" is
  meaningless build time, not a real profile's origin. Loaded profiles are
  untouched (they are never re-serialized through `to_icc` in production).
- **A parsed profile passed to `assign_document_profile` would lose its date.** →
  No such caller exists; documented above as the caller's invariant.
- **An ICC validator could warn that a zero date is not a calendar date.** →
  lcms2, PIL, and ImageMagick accept it; the fixture already ships this way and
  the PIL/ImageMagick oracle tests exercise an external read.
- **The ImageMagick oracle does not run without `magick`.** → It self-skips by
  convention; PIL covers the external-read case in CI where `magick` is absent.

## Migration Plan

None. No API or on-disk format version changes; only the date bytes of newly
synthesized built-in profiles change, and they remain valid ICC profiles.
