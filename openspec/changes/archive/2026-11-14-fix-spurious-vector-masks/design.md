# Design

## Context

See `proposal.md` — Why. The relevant current state:

- `crates/pictura-codec/src/read.rs` parses the raster mask record into
  `Layer.mask`, keeping the raw mask flags byte, and sets
  `Layer.pixel_data_irrelevant` from layer record bit 4. Non-modeled channels,
  including a `-3` real-user-mask channel, are preserved in
  `Layer.raw_channels`.
- `crates/pictura-codec/src/vector_mask.rs::resolve_vector_masks` runs once per
  read (`read.rs`, after the layer section) and assigns `Layer.vector_mask`
  from any parseable `vmsk` block, after merging the mask record's vector
  density and feather placeholder into it.
- `crates/pictura-render/src/document_ops/layer_ops/shape_layer.rs` identifies
  a shape layer from the `vmsk` block plus a decoded fill content; the fill
  compositor needs `Layer.vector_mask` to clip shape fills.
- `OpenTS_logo2.1_Hyper-V1.psd` has `Triangle` and `Chevron` as pixel layers
  (record bit 4 clear) with a `vmsk` whose bounds equal the `-2` mask rect and
  a mask flags byte with bit 3 (`user_mask_from_render`) set. A headless control
  query confirms Pictura currently reports a vector mask for both. These two
  layers are the verified reproducer; the Solid Color adjustment layer named in
  the issue is not present in that file.
- Shape layers that carry the same derived `-2` mask, such as `basic01.psd`'s
  `SoCo` shapes, have record bit 4 set because their appearance comes from the
  vector fill, not from pixels.
- psd-tools classifies a shape from record bit 4 plus a vector block; it does
  not apply Photoshop's rendering rule for the derived source path, so it
  cannot be the behavior oracle for this fix.

## Goals / Non-Goals

**Goals:**

- Make import expose the same mask set as Photoshop for a pixel layer whose
  pixel mask was rendered from a vector path.
- Keep the raw `vmsk` bytes untouched and preserve the raster mask's user
  density and feather. The suppressed path's inert vector density/feather is
  normalized on save (see Risks).
- Keep shape layers, real raster+vector composites, and ordinary vector masks
  unchanged.

**Non-Goals:**

- Decoding `vsms` shape layers (#1114) or changing shape-layer display.
- Changing the writer, the vector-mask flag semantics, or the mask record
  layout.
- Suppressing a rasterized source path on a layer whose record bit 4 is set
  (fill, adjustment, type, smart object, or shape). Such a layer's vector data
  can still be its content, so keeping the view is the conservative choice.
- Preserving a suppressed source path's vector density/feather fields in the
  mask record. Once the derived view is suppressed there is no model carrier
  for them, so the writer normalizes those inert fields to their defaults on
  save; the raster mask's user density/feather still round-trips.

## Decisions

### D1. Predicate: pixel layer + rendered mask + no real-user-mask channel

Suppress the derived view exactly when:

```
!layer.pixel_data_irrelevant
&& layer.mask.flags & FLAG_USER_MASK_FROM_RENDER != 0
&& !layer.raw_channels.iter().any(|c| c.id == -3)
```

Each clause removes a different false positive:

- **Bit 4 clear** keeps shape-layer outlines. Without it, every shape layer
  with a derived `-2` mask (a common Photoshop output) would lose the view the
  fill compositor needs and render as a rectangle.
- **Mask bit 3 set** identifies the raster mask as rendered from other data.
- **No `-3` channel** keeps a live raster+vector composite: Photoshop writes
  the real-user-mask channel when both masks apply.

Alternatives considered:

- **Mask bit 3 alone.** Simplest, but it breaks shape layers and any real mask
  whose writer set the flag conservatively. Rejected.
- **Shape detection from fill-content blocks** (`SoCo`/`GdFl`/`PtFl`/`vscg`).
  Closer to the render layer's `is_shape_layer`, but duplicates fill-block
  knowledge in the codec and still needs the bit-4 rule for shapes whose fill
  block is unmodeled. Bit 4 is the format's own answer to "does the appearance
  come from pixels", and psd-tools uses it. Rejected as the primary rule.
- **Store the source path separately from `Layer.vector_mask`.** More model
  state with no consumer. Rejected.

### D2. Implement at the one read chokepoint

`resolve_vector_masks` is the only place that assigns `Layer.vector_mask` on
read; every `read_psd*` entry point reaches it through `read_container`. Add the
predicate there and skip both the block decode and the density/feather
placeholder merge. No other reader call site, renderer, or app code changes.

The mask flag constant lives in `mask_params.rs` next to `FLAG_DISABLED` and
`FLAG_PARAMETERS`, because that module already owns the mask flags byte.

### D3. The raw block stays authoritative

Suppressing the view does not touch `Layer.extra_blocks`, so the writer emits
the `vmsk` verbatim and a round trip preserves it. A convert/save of such a file
keeps the source path without resurrecting it as a mask. The only writer change
is clearing the raster mask's render flag when a pixel layer carries a live
vector mask and no `-3` channel, so an authored mask (or one whose `-3` was
dropped by an edit) survives a reload; there is no new model field. The one
normalization is the suppressed path's inert vector density/feather (see
Risks).

### D4. Fixture and tests

- Add `derived_raster_mask.psd` to `scripts/generate-fixtures.py`: an 8×8 RGB
  document with one pixel layer, a painted `-2` mask whose
  `user_mask_from_render` flag is set, a closed-rectangle `vmsk`, record bit 4
  clear, and no `-3` channel. Regenerating the fixtures SHALL leave every
  existing fixture byte-identical.
- Unit tests in `vector_mask.rs` cover the four predicate outcomes without a
  fixture: suppressed, shape (bit 4 set), real composite (`-3` present), and
  ordinary raster mask (bit 3 clear).
- An integration test in `vector_mask_oracle.rs` reads the fixture and asserts
  the raster mask present, the vector view absent, the raw block preserved, and
  the same after a save/read round trip. A psd-tools check asserts the fixture
  really carries the `vmsk` and the `user_mask_from_render` flag; it does not
  assert Photoshop's display rule, which psd-tools does not model.

## Risks / Trade-offs

- **A live vector mask on a pixel layer whose writer set mask bit 3 but omitted
  the `-3` channel would be dropped.** Mitigation: Photoshop writes `-3` for a
  real raster+vector composite; the rule is documented as inferred from the
  format and the reference file, not oracle-proven.
- **Bit 4 is also set on adjustment, type, and smart-object layers.** Those
  layers keep the view, which preserves today's behavior; the cost is that a
  rasterized source path on such a layer is still exposed as a mask.
- **Dropping the view drops the vector density/feather placeholder.** Those
  values are inert without a live vector mask, but they are still file data:
  the writer normalizes them to their defaults on save. This is an accepted
  limitation; the raster mask's user density/feather and the raw `vmsk` block
  still round-trip.
- **The composite must not change.** The raster mask is the rendering of the
  same path (the reference file's mask rect matches the `vmsk` bbox), so the
  raster mask already carries the view's coverage and removing the duplicate
  vector mask leaves the effective mask unchanged.
- **`vsms` remains unread.** A shape layer whose outline is only in `vsms` still
  has no vector view; that is #1114's scope.

## Migration Plan

No migration. The change is derived at read time; no stored file is rewritten
except by an explicit save, and the source `vmsk` block is preserved.
