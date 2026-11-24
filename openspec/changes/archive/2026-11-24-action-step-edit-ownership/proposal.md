# Proposal

## Why

An action step holds both an executable payload and the form read from disk. The
writer prefers the preserved form whenever it is present, so the real contract is
"every payload-changing semantic edit must keep that form current, by invalidation
or a coordinated reference rewrite". That contract is a caller obligation
enforced by hand, and one caller already breaks it: renaming an action rewrites
Play and Conditional references in memory but leaves the preserved descriptor
naming the old action, so a save writes a reference that is stale on reload
(#1397).

## What Changes

- Make `ActionStep` itself own the relationship between a semantic edit and the
  preserved imported form: the payload and the imported form become private and
  are changed only through one edit surface. Ordinary semantic edits invalidate
  the preserved form; Play and Conditional reference rewrites coordinate both
  representations. Callers can no longer forget to keep them consistent, and
  there is no forwarding helper that still permits a bypass.
- Route Play and Conditional reference rewrites for an action rename through that
  surface. The rewrite updates both the executable reference and the preserved
  descriptor, so save and reload reproduce the renamed reference while unrelated
  imported content and display-name bytes on that step survive. Fall back to
  re-authoring when the preserved descriptor cannot be patched.
- Keep preserved forms for edits that change nothing: a parameter edit with equal
  values, a repeated Insert Path with the same path, and flag-only edits (enabled,
  with-dialog) do not discard the imported form.
- A rejected duplicate rename changes neither the in-memory library nor its
  serialized bytes.
- Cover the behavior through the same path callers use: import, edit, save, and
  reload, asserting the reloaded step rather than only the in-memory payload.

## Capabilities

### New Capabilities

- `document/action-step-edits`: the rule that a semantic action-step edit and the
  preserved imported form are updated together, including reference rewrites for
  Play and Conditional steps, no-op preservation, and unchanged serialization for
  unedited imported steps.

### Modified Capabilities

- None. The library model and `.atn`/`.psp` byte-fidelity requirements in
  `document/action-engine` and `codec/actions-file` remain as written; this change
  adds the edit-ownership invariant that makes them hold after a rename.

## Impact

- `crates/pictura-script`: `action.rs` (step type, edit surface, and descriptor
  patching), `atn.rs` (decode/write), `library.rs` (rename and edit callers),
  `player.rs`, `recorder.rs`, `batch/`.
- `crates/pictura-app`: `src/cxxqt_object/actions_engine/` reads and constructs
  steps through the new surface.
- Tests: `crates/pictura-script/tests/atn.rs` and `tests/engine.rs` gain
  import→edit→save→reload coverage and migrate existing step literals.
- No new dependency. The descriptor patching reuses `pictura-codec`'s existing
  action-descriptor read/write.
