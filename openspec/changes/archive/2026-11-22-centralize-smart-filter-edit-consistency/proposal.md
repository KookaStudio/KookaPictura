# Proposal: centralize-smart-filter-edit-consistency

## Why

Issue #1398. The codec's smart-filter edit consistency protocol — locate the
`SoLd`/`SoLE` block, validate its `soLD`/`soLE` signature and outer version,
navigate `filterFX.filterFXList`, re-serialize onto the 8-byte prefix with
4-byte padding, then store the bytes on both the tagged block and
`SmartObject.config_descriptor` — exists in two copies. `smart_filter.rs` owns
delete/reorder/clear and the flag toggles; `pictura_raw/mod.rs` re-implements
the same protocol for attach and replace, and `smart_filter.rs` inlines it a
third time inside `set_camera_raw_option_parts`. The generic `filterFX` authoring
(`author_filter_fx`, `filter_item`, `merge_filter_entry`, `group_flags`) lives
in the camera-raw adapter, and `smart_writer::serialize_sold` carries yet
another copy of the framing/padding rule.

Each copy is an independent place for unmodeled keys, padding, ordering,
enabled/blend options and typed-mirror agreement to drift, and every new
filter-specific edit must re-learn the protocol.

## What Changes

- Add one internal consistency owner to `smart_filter.rs`: open the preserved
  config (locate, signature/version validation, descriptor parse), navigate the
  filter stack, serialize with the exact prefix/pad rule, and commit to both the
  tagged block and `config_descriptor`; a typed-only layer with no preserved
  block is opened as a no-op commit.
- Rewrite the seven existing edit operations onto that owner and delete the
  protocol copy inlined in `set_camera_raw_option_parts`.
- Move the generic smart-filter operations into the edit module: attach/append,
  replace, `filterFX` authoring and entry merge. `pictura_raw/mod.rs` keeps only
  the Camera Raw adapter (constants, `Fltr` encode/decode, filter construction,
  `attach_pictura_raw_filter`, text/XMP helpers).
- Make `smart_writer::author_sold_block` call the relocated `author_filter_fx`,
  and `smart_writer::serialize_sold` reuse the shared prefix-and-pad serializer.
- Keep the public `pictura_codec` API and every existing smart-filter test name
  unchanged.
- Unify one error path: `replace_smart_filter` validates the `soLD`/`soLE`
  signature like every other edit instead of accepting any 4-byte signature.
  No accepted edit changes bytes.
- Add invariant tests: the same edit sequence agrees across preserved-block and
  typed-only storage, and a refused edit leaves block bytes,
  `config_descriptor` and the typed view unchanged.

No accepted edit changes bytes: this is a refactor of the smart-filter edit
surface; the only behavior change is the invalid-signature refusal in `replace`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is an internal refactor with no requirement change
(`skip_specs: true`). `codec/psd-smart-filters` already specifies attachment,
toggles, list editing and preservation; the new invariant tests prove those
requirements still hold through the consolidated owner.

## Impact

- `crates/pictura-codec/src/smart_filter/{mod,tests}.rs`: gains the consistency owner and
  the moved generic operations.
- `crates/pictura-codec/src/pictura_raw/mod.rs`: loses the duplicated protocol
  and generic authoring; keeps the Camera Raw `Fltr` adapter and its `Fltr`
  key map.
- `crates/pictura-codec/src/smart_writer.rs`: reuses the shared serializer and
  the relocated authoring.
- `crates/pictura-codec/src/lib.rs`: re-exports unchanged, so
  `pictura_render`, `pictura_app` and their tests need no edit.
- Tests in `smart_filter.rs` and `pictura_raw/tests.rs`; the eight tests named
  by issue #1398 stay green, three invariant tests are added.
- No new dependency, no `docs/` change, no Qt/C++ change, no tracker change
  (feature coverage does not move).
- Provenance: independent refactor from issue #1398; no Adobe behavior is
  involved.
