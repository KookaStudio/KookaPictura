# Design: centralize-smart-filter-edit-consistency

## Context

See `proposal.md` - Why.

Current state that shapes the approach:

- Every public edit is already wrapped by `stage_content::try_edit_layer`
  (`crates/pictura-codec/src/stage_content.rs:141`): it clones `LayerParts`,
  runs the edit, and only installs the result on `Ok`. So the owner never needs
  rollback; it only needs to commit once, at the end.
- Two mirrors describe the same smart-filter stack: the preserved
  `SoLd`/`SoLE` tagged block (authoritative for save) and
  `SmartObject.config_descriptor` (typed mirror read by
  `can_bake_smart_filter_blending` and `read_placed_source`). Both must change
  together.
- Two storage shapes exist: a preserved block that is edited in place, and a
  typed-only embedded object whose `SoLd` `smart_writer` authors on save.
- `smart_object::parse_filter_fx` (`smart_object.rs:198`) is the decode-side
  twin; it stays read-only and out of this change.
- `smart_preview` in the app (`filter_tools/smart_apply.rs:245-255`) mutates the
  typed view without the descriptor on purpose. "Typed agrees with preserved"
  is therefore an invariant of accepted persisted edits, not a global one.

## Goals / Non-Goals

**Goals:**

- One owner for locate/validate/parse/navigate/serialize/commit, used by every
  smart-filter edit.
- `pictura_raw` becomes the Camera Raw `Fltr` adapter only.
- Public `pictura_codec` API and existing test names unchanged.

**Non-Goals:**

- No new filter kind, no per-filter blending (#961), no adjustment facts
  (#955), no image-resource work (#1347), no Smart Filter resource persistence
  (`FEid`/`FXid`/`FMsk`), no codec registry.
- No behavior change for accepted edits, no `docs/`, no Qt/C++ change.
- No change to the decode path (`build_smart_object`/`parse_filter_fx`).

## Decisions

### 1. Full ownership: the generic operations move into the edit module

Chosen over leaving `attach`/`replace` in `pictura_raw` and only sharing
helpers. The deletion test in issue #1398 is removing the duplicated
discovery/framing/padding/commit protocols *from authoring and mutation
callers*; keeping attach in `pictura_raw` would leave the insert path and the
list paths in different owners. After the move:

```
crates/pictura-codec/src/smart_filter.rs   (the edit module)
  Edit / Preserved          the consistency owner
  attach / append           generic insert-by-id / append
  replace                   in-place entry replacement
  delete / reorder / clear  stack structure
  enable / group / mask     filterFX flags
  set_camera_raw_option     Fltr key edit (uses the owner)
  author_filter_fx/filter_item/merge_filter_entry/group_flags
  object()/malformed() helpers

crates/pictura-codec/src/pictura_raw/mod.rs   (Camera Raw adapter)
  CAMERA_RAW_FILTER_ID / NAME
  decode_pictura_raw_settings / encode_pictura_raw_fltr
  new_filter / attach_pictura_raw_filter
  text / xmp     (unchanged)
```

The public `pictura_codec` names are unchanged, but the `pub use` statement in
`lib.rs` must take `attach_smart_filter`, `append_smart_filter` and
`replace_smart_filter` from `smart_filter` instead of `pictura_raw`;
`attach_pictura_raw_filter`, the constants and the text/XMP helpers still come
from `pictura_raw`. `pictura_render`/`pictura_app` callers compile unchanged.

### 2. The owner's interface

```rust
struct Preserved {
    block: usize,
    header: [u8; 8],      // "soLD"/"soLE" + outer version, taken from the reader
    desc: DescValue,
}

struct Edit<'a> {
    layer: &'a mut LayerParts,
    preserved: Option<Preserved>,   // None = typed-only storage
}

impl<'a> Edit<'a> {
    fn open(layer: &'a mut LayerParts) -> Result<Self, PsdError>;
    fn list_mut(&mut self) -> Result<&mut Vec<DescValue>, PsdError>;
    fn list_or_author_mut(&mut self) -> Result<&mut Vec<DescValue>, PsdError>;
    fn entry_mut(&mut self, i: usize) -> Result<&mut Items, PsdError>;
    fn fx_mut(&mut self) -> Option<&mut Items>;
    fn drop_filter_fx(&mut self);
    fn commit(self) -> Result<(), PsdError>;
}

pub(crate) fn serialize_config(header: &[u8], desc: &DescValue) -> Vec<u8>;
```

A free `serialize_config(header, desc)` owns the prefix-and-pad rule: `commit`
calls it, and `smart_writer::serialize_sold` reuses it.

- `open` locates `SoLd` (preferred) or `SoLE`, validates the `soLD`/`soLE`
  signature and reads/preserves the outer version (no reader checks its value
  today), and parses the descriptor. A typed-only layer opens with
  `preserved: None`.
- `commit` takes `header` from the parse (never `data[..8]`), appends
  `write_descriptor(desc)`, pads to a 4-byte multiple, stores the bytes on the
  block and on `config_descriptor`, and is a no-op for typed-only storage.
  Consuming `self` releases the `&mut LayerParts` borrow so the op can sync the
  typed view afterwards.
- `list_or_author_mut` builds a fresh `filterFXStyle` only on the insert path
  (same flags/defaults `pictura_raw::group_flags` produces today). Delete,
  reorder and clear keep using `list_mut`, which errors on a missing list, so
  they cannot accidentally author one.
- `entry_mut`/`fx_mut` replace the current mix of `filter_fx_mut`,
  `filter_entry_mut`, `filter_list_mut` and ad-hoc `get_object_item_mut`
  nesting.

### 3. Typed mirror is hand-synced, not derived

Alternative considered: after `commit`, replace `so.smart_filters` with the
decode parser's view of the descriptor. It would guarantee agreement by
construction, but it re-canonicalizes `options` bytes for entries the edit did
not touch and would overwrite typed-only state. `SmartFilter` has exactly four
modeled fields (`filter_id`, `name`, `enabled`, `options`), so per-op sync is
small and now lives beside the mutation. Revisit derive when #961 adds
descriptor-derived fields to `SmartFilter`.

Agreement is defined over `filter_id`, `name`, `enabled` and list order;
`options` is compared only where the edit rewrote the entry's `Fltr` (the
camera-raw option edit and an append/replace of a fresh entry). A same-id
re-attach keeps the caller's options on the typed filter while the preserved
entry keeps its unmodeled `Fltr` keys, exactly as today.

### 4. One validation rule for the signature

`replace_smart_filter_parts` currently reads and discards the signature
(`pictura_raw/mod.rs:375`), so a block that is not `soLD`/`soLE` can reach the
descriptor parser while every other edit returns `PsdError::Unsupported`.
Routing replace through `Edit::open` unifies the rule. This is an error-path
change only; no accepted edit changes bytes.

### 5. Generic error and helper locations

`malformed()` and the `object()` constructor move to the edit module; the
`"Pictura Raw filter descriptor"` prefix becomes neutral because the owner now
serves Camera Raw, lens-correction and generic records. Tests match the
`PsdError` variant, not the message.

## Operation map

| Operation | Preserved block | Typed-only |
|---|---|---|
| attach / append | rewrite list, then push/replace typed | push typed only |
| replace | rewrite entry, sync typed entry | replace typed only |
| delete | remove list entry, drop `filterFX` when empty | remove typed |
| reorder | move list entry | move typed |
| clear | drop `filterFX` | clear typed |
| per-filter enable | set entry `enab` | set typed |
| group enable / mask flag | set `filterFX` flag | set typed |
| camera-raw option | edit `Fltr` key, sync `options` | still refused (unchanged) |

`attach` keeps today's refusals for a legacy `plLd`/`PlLd` placed layer, a
non-embedded object and an embedded object with no payload
(`pictura_raw/mod.rs:159-173`).

Delete, reorder and replace validate their indices against both lists before
mutating either; the flag toggles keep today's tolerant behavior (set what
exists, error only where today errors).

## Risks / Trade-offs

- **Byte identity on no-op edits** -> `reorder(from == to)` keeps its early
  return; every other op commits only on `Ok`. The descriptor writer is
  deterministic, and the new test asserts byte equality with the original; if
  equality fails, add a same-bytes early return in `commit` that stores nothing.
  The new invariant test compares prefix, length and typed view after each edit,
  and the existing `noop_edit_preserves_config_descriptor_bytes` stays in place.
- **Losing a mirror update during the move** -> the new
  `refused_edit_leaves_block_descriptor_and_typed_view_unchanged` and
  `edit_sequence_agrees_across_preserved_and_typed_only` tests cover refusal and
  agreement for insert/replace/delete/reorder/clear/toggle.
- **Borrow shape** -> `commit(self)` is deliberately last; typed sync runs
  after it on the original `layer`.
- **File size** -> `smart_filter.rs` is 916 lines today (~400 production).
  Moving ~200 production lines in plus the new owner exceeds the 800 target and
  approaches the 1200 cap. If it does, split along the seam: move the tests to
  `smart_filter/tests.rs` (test cap 1400) or the owner to `smart_filter/config.rs`.
  `pictura_raw/mod.rs` drops from 440 to about 60.
- **Preview divergence is intentional** -> `smart_preview` keeps mutating only
  the typed view; it is never saved and is out of this change.
- **Trailing padding readers** -> the 4-byte pad rule and its `ponytail:`
  comment stay exactly as they are, now with one implementation.

## Migration Plan

Single pull request: tests, owner, move, then the verification gates. No data
migration or compatibility shim. Rollback is reverting the commit; preserved
document bytes are unaffected.

## Open Questions

None.
