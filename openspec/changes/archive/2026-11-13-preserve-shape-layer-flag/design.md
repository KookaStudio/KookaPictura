# Design

## Context

See `proposal.md` for motivation and the delta specs for required behavior.

The codec reads the layer record's flags byte in
`crates/pictura-codec/src/read.rs` and uses bit 0 for the legacy
transparency-protection lock and bit 1 for visibility; bits 3 and 4 are
discarded. `write_layers.rs::write_record` rebuilds the byte from `visible` and
the transparency lock only, so neither bit can survive a save.

`pictura_core::Layer` (`crates/pictura-core/src/lib.rs`) has no field for the
bit. `pictura_render::add_shape_layer`
(`document_ops/layer_ops/shape_layer.rs`) creates shapes as a solid-color fill
with an authored `vmsk`/`vogk`, but never marks them. `rasterize_shape`
(`document_ops/layer_ops/rasterize.rs`) keeps an imported shape's `vscg` block
while dropping `vmsk`/`vogk`; `rasterize_fill_content` keeps an authored shape's
`vmsk`. Both bake pixels into the layer but leave its record flags untouched.

Read-only inspection confirms the classification rule and the fixtures:

- `vector_fill.psd`: `Shape` flags `0x18`, kind `shape`; `Base` flags `0x08`,
  kind `pixel`.
- `vector_mask.psd`: both vector-masked solid-fill layers have flags `0x08`,
  kind `solidcolorfill`; the generator does not set bit 4 for them.
- `scripts/generate-fixtures.py` already sets
  `rec.flags.pixel_data_irrelevant = True` for the `vector_fill.psd` shape.
- The external reference PSD has a shape layer whose flags byte is `0x38`
  (bits 3, 4, and an undocumented bit 5); the committed `vector_fill.psd`
  shape copy is `0x18` (bits 3 and 4).
- psd-tools 1.19's layer factory classifies a shape from
  `record.flags.pixel_data_irrelevant` plus a vector block; it does not gate on
  bit 3.

Graph coverage is unavailable for this worktree; the observations above were
checked against current source, fixtures, the installed psd-tools source, and
the reference file.

## Goals / Non-Goals

**Goals:**

- Preserve bit 4 through PSD and PSB save/read and write bit 3 with it so
  Photoshop treats bit 4 as valid.
- Make app-authored shapes carry bits 3 and 4 without reclassifying unrelated
  vector-masked layers.
- Clear the flag when Rasterize Shape or Rasterize Fill Content bakes appearance
  into pixels.
- Keep default documents byte-identical to the current writer output.

**Non-Goals:**

- Preserve undocumented record-flag bits 5-7, or a source bit 3 on a layer whose
  bit 4 is clear. The writer emits `0` for them as today.
- Derive the flag from vector blocks on save.
- Change the layer-kind predicate used by the app, the Layers panel, or
  `is_shape_layer`; those already identify shape layers from content.
- Change `drop_type` or `rasterize_smart_object`: the external reference and the
  committed fixtures do not put bit 4 on type or smart-object layers, and
  psd-tools's type/smart-object branches take precedence over the shape
  condition.

## Decisions

### Store one boolean and derive the validity bit

Add `pixel_data_irrelevant: bool` (default `false`) to the layer model, set it
from `flags & 0x10` in the reader, and write `0x08 | 0x10` into the record flags
when it is true. Bit 3 means "flags valid"; Photoshop uses it to decide whether
bit 4 is meaningful. A stored clear bit 4 leaves both bits clear, so documents
whose layers never carry bit 4 still write flags `0` and the existing goldens
are unchanged.

The reader stores only bit 4. Bit 3 is derived on write, not preserved from the
source: a source bit 3 on a layer whose bit 4 is clear remains unpreserved, as
it is today.

Alternatives rejected:

- **Preserve the raw flags byte.** It would also round-trip source bit 3 on
  bit-4-clear layers and undocumented bits 5-7, but duplicates the visible and
  lock bits the model already owns and changes the written bytes of every
  psd-tools-authored fixture. The validity bit is the only extra bit this bug
  needs.
- **Derive the bit on write from `vmsk`/`vscg`/`vogk` plus fill content.** It
  needs no model field, but it would promote `vector_mask.psd`-style fill layers
  to shapes and silently rewrite unmodeled source data, breaking the
  lossless-re-save stance.

psd-tools ignores bit 3 during classification, so the oracle must assert the raw
flags byte `0x18` in addition to `kind`; Photoshop's use of bit 3 remains
inferred from the format documentation and the reference file, not proven by
the oracle.

### Mark only layers created by the shape tools

Set the new field in `add_shape_layer`, next to the name and path writes. It is
the single creation choke point: `add_solid_fill` alone stays an ordinary fill
layer, `set_layer_shape_paths` only edits existing shape layers, and copies and
history snapshots carry the field through the existing `Layer` clone.

### Clear the flag when rasterization bakes appearance into pixels

Set `pixel_data_irrelevant = false` at the end of `rasterize_shape` and
`rasterize_fill_content` after the composite is baked into the layer channels.
`rasterize_shape` keeps an imported `vscg` block, and `rasterize_fill_content`
keeps an `vmsk`; either stale vector block plus a stale bit 4 makes psd-tools
report a shape for a layer the app now treats as pixels. The app's
`Layer > Rasterize > Layer` on an authored shape routes through
`rasterize_fill_content`, while `Rasterize All Layers` reaches `rasterize_shape`,
so both functions must clear the flag.

`rasterize_vector_mask` is not changed: it keeps the layer's fill content, so the
layer remains a fill layer and bit 4 is still correct. `drop_type` and
`rasterize_smart_object` are out of scope for the reasons in the Non-Goals.

### Reuse the existing independent fixtures

`vector_fill.psd` is the positive case (bits set) and `vector_mask.psd` is the
negative case (vector-masked fill layers with bit clear); no fixture or
generator change is needed. A new focused integration test file,
`crates/pictura-codec/tests/layer_record_flags.rs`, covers reading, PSD/PSB
round trips with the PSB container flag normalized, the no-inference case, the
no-gain case, psd-tools classification of written copies, and the rasterized
save. `crates/pictura-render/src/tests/rasterize.rs` gains one test per
rasterization path.

The existing `psd_tools_reads_an_authored_shape_layer` test uses a **separate
script constant** that reports the layer kind and raw record flags, so the
shared `SCRIPT` and the expected rows of `psd_tools_reads_vector_mask_fixture`
stay unchanged.

## Risks / Trade-offs

- **A new field breaks every fully spelled `Layer { ... }` literal.** → Add the
  field and use `..Default::default()` at the remaining sites; the compiler
  enumerates any missed literal.
- **Setting the bit too broadly would turn fill layers into shapes.** → The
  negative fixture test and the no-gain test assert clear stays clear in the
  model and in psd-tools.
- **The validity bit could change default bytes.** → Bit 3 is written only with
  bit 4, so default documents keep flags `0`; the existing golden tests guard
  the default path. Written flagged records assert the raw byte `0x18`.
- **A rasterizer could leave a stale flag.** → Both shape and fill-content
  rasterizers clear it, with render and save/oracle tests. Type and
  smart-object rasterization are out of scope per the Non-Goals.
- **The independent decoder's classification could drift across versions.** →
  Assert both the raw `0x18` byte and `kind`, keep the existing availability
  self-skip, and rely on CI's pinned oracle job. Photoshop behavior is inferred,
  not oracle-proven.
- **Committed golden bytes could change.** → The flag defaults to clear and
  existing golden tests (`default_document_matches_rle_golden`,
  `existing_files_stay_byte_identical`) guard the default path.

## Migration Plan

No migration or dependency change. Ship the model field, reader, writer,
authored-shape marker, and rasterization clearing together so a layer cannot be
read with the flag and written without it, or rasterized into a stale shape
record. Rollback restores the old semantic downgrade; no stored file is
rewritten except through an explicit save.
