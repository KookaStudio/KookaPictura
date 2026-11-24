# Design

## Context

See `proposal.md` for motivation and `specs/document/action-step-edits/spec.md`
for the behavior contract. Today `ActionStep` exposes `kind` and `imported` as
public fields, and the `.atn` writer prefers `imported` whenever it is present
(`crates/pictura-script/src/atn.rs`). Every editing path must therefore keep
`imported` current by hand; `ActionLibrary::rename_action` currently does not, so
it writes a stale reference.

## Goals / Non-Goals

**Goals:**

- Make "a semantic edit keeps the preserved form current" structural, not a
  caller convention, with no path that can bypass invalidation or a coordinated
  reference rewrite.
- Make Play and Conditional rewrites survive save and reload while keeping
  unrelated imported bytes on the rewritten step.
- Preserve the existing lossless behavior for unedited steps and for flag-only
  or no-op edits.

**Non-Goals:**

- No change to the `.atn`/`.psp` byte layout or to the codec crate.
- No new editing feature in the UI; the Actions panel keeps its current inputs.
- No change to parameter-edit re-authoring: a real parameter change still drops
  the imported form and rebuilds the descriptor from the payload.
- No new dependency.

## Decisions

### `ActionStep` owns the invariant, and its fields are private

`ActionStep::kind` and `ActionStep::imported` become private. The only mutation
surface for the executable payload is:

- `ActionStep::new(kind)` for authored steps; `ActionStep::with_imported(...)`
  is crate-private and used only by the ATN decoder.
- `kind()` and `imported()` for reads.
- `set_kind(kind)` — replaces the payload and invalidates the preserved form only
  when the payload actually differs.
- `set_params(params) -> bool` — the Command/MenuItem parameter edit; returns
  `true` whenever the step accepts parameter edits (Command or MenuItem),
  independent of whether the value changed. Invalidation is decided separately
  by value equality, so `library_ops.rs::edit_step` keeps its no-op behavior.
- `rewrite_action_reference(old, new) -> bool` — the Play/Conditional reference
  rewrite.

`flags`, `dialog_options`, and `display_name` stay public: they are header and
display state, not the executable payload, and changing them does not make the
preserved descriptor stale.

Why: the issue's deletion test is the removal of caller obligations. A public
`kind` plus a helper would still allow a caller to write `kind` and skip
invalidation, so a forwarding helper is rejected. Private fields make that
bypass a compile error rather than a review catch.

Alternative considered: keep the fields public and add helpers. Rejected — it
does not remove the obligation, only documents it.

### Reference rewrite patches the preserved descriptor, with re-author as fallback

For a Play step the preserved reference may live in the `Nm  ` item and/or a
`jsn` sidecar's `action` key (the decoder accepts either); for a Conditional step
the decoded sidecar holds `{v, condition, then, else}`. The rewrite updates the
executable payload and, when a preserved descriptor parses, updates every
matching copy of the affected top-level JSON string value. It replaces only the
value token in the original JSON text, preserving whitespace, key order,
duplicate unknown keys, number lexemes, nested values, and all unrelated bytes.
It updates every matching duplicate, so no stale duplicate survives. Unknown
descriptor items and the imported display-name bytes are left untouched.

A Conditional rewrite touches only sidecars whose `condition` decodes to the
step's current condition and whose `then`/`else` values match the decoded
branches. The first parseable sidecar used by the decoder MUST be patched
safely; otherwise the whole imported form is invalidated. Other `jsn` items
remain byte-for-byte unchanged, even when they contain similarly named keys.

The parse-failure branch is defensive: `ImportedStepForm.descriptor` is always
produced by `write_action_descriptor`, so it always re-parses. The reachable
fallbacks are a step with no preserved descriptor, no matching reference copy,
disagreeing Play copies, or a JSON member scan that cannot safely locate the
value spans. In those cases the rewrite invalidates the preserved form and lets
the writer re-author the step from the payload. That keeps the reference correct
even for steps the patch logic does not understand.

Why patch instead of always clearing: a rename is not a payload change, and
clearing would discard unknown descriptor items and non-UTF8 display-name bytes
that the source format is otherwise required to preserve. Parameter edits still
clear and re-author, which is the documented existing behavior.

Alternative considered: clear and re-author on rename. Smaller change, but it
drops preserved content the spec now requires, and it makes the rename path
indistinguishable from a parameter edit.

### No-op detection is value equality

`set_kind` compares the new payload with the current one, and `set_params`
compares `CommandParams`. Equal values leave the preserved form in place.
`ActionLibrary::insert_path` replaces an existing Insert Path through
`set_kind`, so a repeated identical path also keeps the imported form.

The ATN writer is unchanged: it still prefers the preserved form when one is
present. It now relies on the type to only leave that form present when it is
still current.

### Verification uses the library save/load path

Tests import bytes, edit through the public library or edit surface, call
`ActionLibrary::save_bytes` (or `set_bytes`), reload with `load_bytes`, and
assert on the reloaded step. Asserting only the in-memory payload is what
allowed the original defect to pass.

## Risks / Trade-offs

- Descriptor AST re-encoding still runs after a patch → JSON reference edits are
  textual span replacements, so unrelated JSON bytes, duplicate keys, number
  lexemes, and key order survive; tests also assert an unknown `Xtra` item and
  display-name bytes survive a rewrite, and any unsafe scan falls back to
  re-authoring.
- A name may exist in more than one place in a descriptor → the patch updates
  every matching Play copy (`Nm  ` and `jsn.action`) and every matching duplicate.
  Conditional sidecars must match the decoded condition and branches; unrelated
  sidecars stay untouched. When no matching reference copy is present, fallback
  re-author writes the reference from the payload.
- Private fields break every external step literal and direct read → the
  migration is mechanical and the compiler enumerates the sites; the change
  covers `pictura-script` callers, the app adapter, and tests.
- `CommandParams` equality may differ from byte equality for exotic JSON values
  → a false "changed" result only causes re-authoring, which is still correct;
  a test pins the same-value case.
- The patch path is new code on untrusted imported bytes → it only runs on
  already-decoded steps and falls back on any parse failure; no panics.

## Migration Plan

Single-crate refactor plus adapter updates. No data migration: existing libraries
and files are read as before, and behavior changes only on the next semantic
edit. Rollback is reverting the commit; no persisted format changes.

## Open Questions

None.
