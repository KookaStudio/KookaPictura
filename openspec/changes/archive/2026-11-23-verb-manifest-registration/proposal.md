# Proposal

## Why

Issue #1160 asked for "one bridge root" and split into two halves. The
`bridge-membranes` change shipped the include half: generated headers are
confined to `cpp/api/` and the small bridges were folded. The second half —
"verb registration is in one place" — was deferred to #1187.

The deferral hid a distinction. Adding a **verb** to an existing bridge is
already one edit. Adding a **bridge** is three: the new file, its section in
`build.rs`'s `.files([...])` list, and the membrane that owns its header. Six
bridges landed in the sixteen days after the membranes shipped (69 -> 74,
one removed), so the bridge set keeps moving; the recurring cost is bridge
registration, not verb registration. Meanwhile
nothing enumerates the 1,465 declared verbs, so "which membrane owns
`guide_count`?" costs a grep, and the 10 `#[cxx_name]` renames exist only in
generated headers.

## What Changes

- Move the bridge-file list out of `build.rs` into a checked-in generated
  artifact, `crates/pictura-app/bridge_sources.txt`, produced by a blessable
  test in `build_config` and read by `build.rs` instead of 74 hand-written
  lines. A drift test holds the list identical to the files carrying a
  `#[cxx_qt::bridge]` module, so an unlisted bridge fails the gate instead of
  silently losing its generated headers. Adding a bridge becomes: create the
  source, record its header in a membrane, bless the list — no build-script
  edit.
- Add a checked-in **verb manifest**,
  `crates/pictura-app/src/build_config/verb_manifest.tsv`: one row per declared verb with its
  kind (`free` / `qinvokable` / `qsignal`), its C++ name, its owning membrane
  or adapter header, and its bridge file. It is the single readable map from a
  verb to the header a caller should include.
- Generate that manifest with a blessable test in `crates/pictura-app/src/
  build_config.rs` — the module that already parses bridge modules — so there
  is no third declaration parser and no new script or dependency. A normal test
  run fails when the manifest and the bridge sources disagree.
- Correct the bridge-count drift the deferral left behind: the tree has 74
  bridge files, while `openspec/specs/ui/view-seam-adapters` and
  `docs/dev/{STATE.md,refactor-view-seam.md}` still say 69. The ratchet keeps
  the real number and only shrinks from there.
- **Not** a build-time generated manifest. The `actions-bundle` design
  (`openspec/changes/archive/2026-10-23-actions-bundle/design.md`, decision 2)
  chose a checked-in artifact plus a drift test over build-time generation "to
  avoid more CMake/build-order surface", and this change follows that
  precedent. No runtime behavior change either way.
- No C++ call site changes, no membrane regrouping, no per-membrane stamp, and
  no typed adapter is added.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui/view-seam-adapters`: adds the requirement that bridge registration is
  derived from the tree rather than hand-listed, and that a checked-in verb
  manifest records every verb's kind, C++ name, and owning header; the
  folded-bridge and ratchet requirements drop the stale `<= 69` budget for the
  shipped 74.

## Impact

- Rust: `crates/pictura-app/build.rs` loses its 74-entry source list and reads
  `bridge_sources.txt`, with one `rerun-if-changed` for that file. Nothing new is
  watched under `src/`, so a non-bridge edit does not rerun the build script.
  `crates/pictura-app/src/build_config.rs` gains the two writer/reader pairs and
  their drift tests; the file is over 800 lines already, so both land as
  `build_config/` submodules.
- New tracked artifacts: `crates/pictura-app/src/build_config/verb_manifest.tsv`
  (~1,465 rows) and `crates/pictura-app/bridge_sources.txt` (74 rows), both
  generated, never hand-edited.
- Specs: a delta under `ui/view-seam-adapters`, plus a small `MODIFIED`
  `meta/build-performance` delta that adds the checked-in bridge list to the
  inputs the cxx-qt build script is allowed to rerun on. The "an
  implementation-body edit does not rerun the script" guarantee is unchanged:
  the list is not derived from bodies.
- Deliberate deviation from #1187's literal text, stated here so it is
  reviewable: the issue asks for a *verb* registration point and a *build-time*
  manifest. Adding a verb is already one edit, so the recurring cost is adding a
  *bridge*; and a checked-in artifact plus a drift test is this repository's
  shipped pattern for generated tables (`gen-command-ids.py` / `command_ids.h`),
  chosen by the `actions-bundle` design to avoid build-order surface. Membrane
  generation stays out of scope: the grouping is editorial.
- Docs: `docs/dev/STATE.md` and `docs/dev/refactor-view-seam.md` gain the
  manifest and corrected bridge count; committed with `TASK-ALLOWS-DOCS`.
- Dependencies: none. Tests: Rust `#[test]` only, run by the existing
  nextest step in `verify-fast`.
