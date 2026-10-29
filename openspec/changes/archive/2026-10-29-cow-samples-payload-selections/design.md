# Design

## Context

See `proposal.md` for the motivation. The relevant current state:

- `Plane<T>` already provides refcounted copy-on-write storage (`Arc<[T]>` plus a
  stamp/ident and a forking `DerefMut`). It backs `PixelBuffer`, `Channel`,
  `LayerMask.data`, and `FilterMask.data`, so 8-bit pixel planes already share on
  clone.
- `Samples` holds plain `Vec<u8>`/`Vec<u16>`/`Vec<f32>`; `SourcePlanes` and
  `SourceChannels` embed it, so cloning a document deep-copies native samples.
- `SmartObject.payload` and the raw `lnk2`/`lnk3` linked-section bytes in
  `Document.layer_section_extra` are plain byte vectors. `resolve_smart_objects`
  copies the same embedded PSB bytes into both, so a clone currently copies the
  payload twice.
- `RawChannel.data`, `LayerBlock.data`, `LayerMask.extra`, `AdjustmentData.data`,
  `SmartObject.config_descriptor`, `SmartObject.crs_xmp`, `Document.source_icc`,
  `Document.document_icc`, `Document.color_mode_data`,
  `Document.image_resources`, `Document.global_layer_mask`,
  `Layer.blending_ranges`, `SmartFilter.options`, `TypeTool.text_desc`, and
  `TypeTool.warp_desc` are plain byte vectors.
- `LayerStack` wraps `Vec<Layer>`; cloning a document or a clipping base copies
  the whole tree structure.
- `Selection.data` is a document-sized `Vec<u8>`, cloned about three times per
  history capture.
- `History` stores `Document` and `Option<Selection>` and keeps a materialized
  `current` state; its public method signatures and 20-state/10-snapshot bounds
  are a spec contract. `Snapshot` keeps its shape; `Selection.data`'s public type
  changes as noted below.
- `can_free_transform` clones the document and rasterizes a smart object only to
  answer a boolean.

## Goals / Non-Goals

**Goals:**

- Every bulk store owned by `Document`, `Layer`, `Samples`, and `Selection` is a
  refcount bump on clone, and a write forks only the store being written. This
  includes the remaining preserved `Vec<u8>` fields listed above.
- History retention for native samples, payloads, raw linked-section bytes, byte
  blocks, the layer tree's nodes, and the selection scales with the data a
  capture actually changes. The layer-tree container itself may be forked while
  a capture builds its materialized states.
- Preserve byte-identical rendering, history round-trips, and PSD output.

**Non-Goals:**

- Tiled/region storage, compression, or a new history algorithm.
- GPU residency or compositor row-band redesign.
- Changing the compositor's layer-dispatch signature or threading per-field
  overrides through it. That issue proposal is superseded by making `LayerStack`
  cheap to clone; the per-band `Layer::clone` still copies the small container
  vectors (`channels`, `extra_blocks`, `raw_channels`, `smart_filters`) while
  their bulk data is shared.
- Any PSD format, golden-image, or oracle change.

## Decisions

### `Samples` variants hold `Plane<T>`

`Samples::U8(Plane<u8>)`, `U16(Plane<u16>)`, `F32(Plane<f32>)`. `Plane` already
has the required `Clone`, `PartialEq`, `Deref`, `DerefMut`, `From<Vec<T>>`, and
allocation-identity (`shares`) behavior. Making all three variants copy-on-write
also fixes the 8-bit Lab retained store, not only 16/32-bit documents.

Consequences: `SourceChannels.planes` forks per channel because each plane is a
separate `Samples`; `SourcePlanes.samples` is one flat multi-channel store, so a
write there forks the whole store. That limitation is accepted and documented;
per-channel CoW is not required for the history tests.

Alternative considered: keep `Vec` inside `Samples` and wrap
`SourcePlanes`/`SourceChannels` in `Arc`. Smaller migration, but a write copies
the entire store, so the "grows by dirty data only" requirement fails for
multi-plane stores.

### `SmartObject.payload` and `Document.layer_section_extra` hold `Plane<u8>`

`payload()` stays `Option<&[u8]>` through `as_deref`; `set_payload(Vec<u8>)` and
`with_payload(Vec<u8>)` keep their signatures and wrap on assignment.
`payload_mut` returns `&mut Option<Plane<u8>>`; it has no callers, so the
signature change is contained. `PartialEq` still compares bytes.

`Document.layer_section_extra` is the raw `lnk2`/`lnk3` section. It carries the
same embedded PSB bytes that `resolve_smart_objects` copies into
`SmartObject.payload`, so both become refcounted planes. A document clone then
shares the typed payload and the raw section allocation; the sharing test covers
both.

### `Selection.data` holds `Plane<u8>`

This is preferred over the issue's `Arc<Selection>` in history because it keeps
the `History` method signatures, the `Snapshot { doc, selection: Option<Selection> }`
shape, and the 20-state/10-snapshot bounds unchanged. `Selection.data`'s public
field type changes from `Vec<u8>` to `Plane<u8>` (workspace-internal breaking);
the design does not claim the whole `History` API is unchanged. Every existing
`selection.clone()` becomes a refcount bump, and a selection edit forks on the
first mutable access.

Alternative considered: store `Option<Arc<Selection>>` in `Snapshot`/`Delta` and
reuse the `Arc` while bytes are unchanged. That changes the public `Snapshot`
type, needs an equality check per capture to decide reuse, and does not make
selection edits cheaper.

### `LayerStack` is backed by `Arc<Vec<Layer>>`

`LayerStack` keeps its slice-like API. Reads go through the `Arc`; mutating
methods and `IndexMut` call `Arc::make_mut`. The by-value and mutable-iteration
paths need explicit handling:

- `retain_recursive` uses
  `Arc::unwrap_or_clone(std::mem::take(&mut self.0))` before consuming the old
  layers.
- `IntoIterator for LayerStack` uses `Arc::unwrap_or_clone(self.0).into_iter()`.
- `IndexMut`, `get_mut`, `first_mut`, `last_mut`, `iter_mut`, `normalize`, and
  `normalize_tree` call `Arc::make_mut`.
- Every structural mutator (`insert`, `remove`, `move_node`, `splice`, `group`,
  `reverse`, `truncate`, `set_clipping`, `promote_background`) calls
  `Arc::make_mut` on the container it changes.

`container_mut` calls `get_mut` at each level, so a nested edit forks each
ancestor container; the tree is still O(nodes) shallow work, not O(pixels).
`splice` clones a backup before mutating, so its first edit forks even when the
tree was unique. That is existing behavior and is accepted.

This makes `Layer::clone` and `Document::clone` cheap for the tree and makes the
compositor's three `..base.clone()` sites cheap without touching
`composite_layer_inner` or its callees.

Alternatives considered:

- Thread opacity/blend/fill/children overrides through `composite_layer_inner`.
  Matches the issue's proposal but reaches `content_route`, `effect_passes`,
  `composite_pixels`, `composite_adjustment`, `composite_canvas`,
  `blend_into_row`, and text/effect paths. High blast radius for the same
  measured outcome.
- Special-case only the proxy at `composite_clipping.rs:73` to avoid the
  children clone. Leaves the `:49` and `:89` clones copying the subtree.

### Preserved byte blocks hold `Plane<u8>`

`RawChannel.data`, `LayerBlock.data`, `LayerMask.extra`, `AdjustmentData.data`,
`SmartObject.config_descriptor`, `SmartObject.crs_xmp`,
`Document.source_icc`, `Document.document_icc`, `Document.color_mode_data`,
`Document.image_resources`, `Document.global_layer_mask`,
`Document.layer_section_extra`, `Layer.blending_ranges`, `SmartFilter.options`,
`TypeTool.text_desc`, and `TypeTool.warp_desc` become `Plane<u8>`. These are
preserved verbatim and are usually read-only, so sharing on clone is the natural
fit. Construction sites wrap with `.into()`; sites that need an owned `Vec` use
`.to_vec()`.

Sites that need an owned `Vec` include `Samples::to_bytes`/`narrow_to_u8`/
`to_u16`, `color_mode.rs`'s `Vec<Vec<u8>>::concat()` paths,
`write_layers.rs:359`'s verbatim `RawChannel` stream, `fill_native.rs`'s
`Vec<u16>`/`Vec<f32>` test helpers, `clipboard.rs`/`image_match_color.rs`'s
`Option<Vec<u8>>` coverage helpers, and `mask_refine.rs`'s `Vec<u8>` conversion.
Byte-block assignments in `smart_object.rs`, `smart_filter.rs`, `crs_xmp.rs`,
`pictura_raw/mod.rs`, `vector_masks.rs`, `shape_layer.rs`, `read.rs`,
`mask_edit.rs`, and `LayerMask::default` need `.into()`. The 8-bit
`auto_native` path moves the plane back with `*data = buf.data;`, not
`buf.data.to_vec()`.

### `can_free_transform` uses a read-only predicate

Add a `pictura-render` predicate that answers
`layer.smart_object.is_some() && render_smart_source(so, layer.rect, layer.rect).is_some()`
for a layer that has no channel id `0`, without cloning the document. The
transform session calls it instead of cloning and rasterizing. The predicate
still decodes and rasterizes the layer source; it avoids only the document clone
and the channel split. Caching the decode/raster is out of scope.

Tests cover a decodable payload (true), an empty or undecodable payload (false),
and a channel-less non-smart layer (false). Existing shape/lock tests remain.

### Mechanical migration

The type changes are compile-driven. Constructor sites use
`Plane::from(vec)`/`.into()`; pattern sites rely on `Deref`/`DerefMut`; functions
that return an owned `Vec` from a store use `.to_vec()`. Public method
signatures are preserved except `SmartObject::payload_mut`.

Churn estimate: roughly 300 `Samples` references, ~60 `Selection` sites, and
~50 `LayerStack` field uses (including tests and pattern sites).

## Risks / Trade-offs

- **Mechanical churn** (~300 `Samples` references, ~60 `Selection` sites, ~50
  `LayerStack` field uses) → keep each edit local and let the compiler drive;
  run the existing unit and oracle suites.
- **`Plane<f32>` equality with NaN** → same semantics as the current `Vec<f32>`
  (NaN never equals NaN); no regression, and no golden relies on NaN equality.
- **`LayerStack::IndexMut`/`get_mut` now fork the tree when shared** → one copy
  on the first write, then in place. Structural edits are user-paced; the
  layer-stack invariant tests cover behavior.
- **`hollow`/`private_copy` fork the tree container** because they mutate tracked
  planes through `iter_mut` → O(nodes) shallow work per materialized state, not
  O(pixels). The spec permits the tree container fork while requiring the nodes'
  bulk stores to stay shared.
- **`splice` clones a backup before mutating** → the first edit forks even when
  the tree was unique. Existing behavior, accepted.
- **`SourcePlanes` is one flat store** → a write forks all document composite
  planes. Accepted and documented; per-channel sharing applies to
  `SourceChannels`.
- **A write through `DerefMut` on a shared `Plane` copies that store** → the
  intended copy-on-write cost, paid once per fork.
- **Memory assertion tests can be flaky if based on RSS** → assert allocation
  identity (`Plane::shares` / `Arc::ptr_eq`, or `as_slice().as_ptr()` for a
  `LayerStack` outside its module) instead of process memory.
- **The per-band `Layer::clone` remains** → it copies small container vectors
  (`channels`, `extra_blocks`, `raw_channels`, `smart_filters`) while their bulk
  data is shared. Eliminating it needs the compositor override redesign and is
  out of scope.

## Migration Plan

No persisted data or serialized format changes; the change is workspace-internal
and ships in one pull request. Rollback is a revert. Existing PSD round-trip and
golden oracles must pass unchanged before merge.

## Open Questions

None blocking. An optional ignored allocation profile for the clipping
compositor can be added if a later regression suggests the remaining per-band
`Layer` allocations matter.
