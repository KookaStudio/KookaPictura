# Design

## Growth primitive

`pictura_core::layer_bounds::grow_layer_to(layer, rect, purpose)` grows one
pixel layer's rect to a superset `rect`:

- it refuses (returns `Ok(false)`) when `rect` is not a superset, the layer is
  transparency-locked, or the layer is a group/adjustment;
- it reserves the added bytes through the process governor, then allocates each
  padded plane fallibly (`try_reserve_exact`), so an allocator failure is the
  typed `MemoryError::Insufficient`, never an abort;
- it pads every 8-bit channel row into the new width, zero-filling the new area
  (transparent, black);
- it pads `source_channels` when its rect matches the old layer rect, so a
  16/32-bit store survives a grow with its precision;
- it drops `raw_channels`: those are verbatim compressed streams for channel ids
  the engine does not model, and a compressed stream cannot be padded. This is a
  deliberate ceiling (`ponytail:`), because an unmodeled channel on a grown
  layer is rare and keeping it at the old size would corrupt the save.

The rect is not clipped to the canvas: a PSD layer may extend past the canvas,
and the callers that paint or fill already clip their own work to the canvas.

## Empty new layers

`transparent_layer` stays a sized transparent layer, because Layer via Copy,
paste and the neutral-fill dialog need one. A new `empty_pixel_layer` builds the
same channel ids with empty planes and a 0×0 rect, and `add_layer` /
`add_layer_full` use it unless `neutral_fill` is set. A compositor that reads
`layer.rect` already treats a 0×0 layer as contributing nothing.

## Paint growth

`Stroke` owns the layer-local coordinate system: its `Saved` pre-stroke planes,
the coverage `scratch`/`applied` buffers and the per-dab RGBA image are all
layer-sized. Before a sample is placed, the stroke grows its target when the
sample's reach (diameter plus scatter, clipped to the canvas) leaves the rect.
Growth pads every layer-local buffer in place, preserving the pixels already
painted and the pre-stroke tiles already saved; the per-dab engines' cached
planes (colour replacement's done mask, the tone brush's applied plane, the
sampled composite images, the art-history source) are padded the same way. The
dirty rectangle and the layer path stay valid because they are remapped by the
same offset.

A per-dab engine that carries a *dab-sized* state (smudge's carried patch) keeps
it unchanged: the patch is independent of the layer rect. A growth event in the
middle of a retouch stroke is rare (the stroke has left the layer's content);
the carried state is preserved, so the stroke stays continuous.

## Fill growth

The shared `pictura_paint::fill::fill` entry grows the target to its `within`
box (the selection's box, or the canvas) before choosing the 8-bit or native
path. The app's memory gate runs before the engine, so `fork_bytes` estimates
the bands of `layer.rect ∪ bounds`; the grow helper then reserves the added
bytes itself.

## Filter growth

A committed filter grows the layer by the filter stack's apron
(`Filter::preview_apron`, already used by the preview) before applying it. A
point filter has apron 0 and grows nothing. The preview keeps the current rect;
only a commit changes the stored rect, so a cancelled preview leaves the layer
untouched. After the commit the layer is trimmed back to its content, so a blur
that spreads into transparent padding does not keep the padding forever.

## Trimming

`trim_to_content` already drops only fully transparent pixels. A Move commit
calls it for every moved pixel layer after the translation, before the history
state is captured. Free Transform already trims at session begin; its
`untrim_to` stays as the cancel path, which is still the cheapest way to restore
the pre-session extent. A layer created from a fill is trimmed at creation.

## PSD

The layer record already writes `layer.rect`; the channel encoders already accept
a zero-length plane. The work is the read side and the tests: a 0×0 record has
zero-length channel payloads, and `plan_channel_tasks` must accept a zero area
without charging a pixel budget. Existing files never reach that path, so their
byte-identical round trip is unchanged.

## Ceilings

- A layer with unmodeled `raw_channels` loses them when it grows.
- A filter preview does not grow the layer; only its commit does.
- `untrim_to` remains for Free Transform cancel; it is no longer required for a
  later edit to reach outside the rect.
- A Paint Bucket with no selection still grows to the canvas; the tiled flood
  that keeps that case small is #1214.
