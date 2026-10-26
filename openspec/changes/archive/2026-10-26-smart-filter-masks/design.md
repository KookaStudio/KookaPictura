# Design: smart-filter-masks

Grounded on a Photoshop-authored PSD supplied for #956 (used as a black-box
reference, not committed). The format findings:

- A document-level `FEid` (`FilterEffects`, version 3) block in
  `Document::layer_section_extra` holds one `FilterEffect` per masked smart
  object, keyed by the object's SoLd `placed` uuid. Its body is
  `rect (4i), depth (u32), max_channels (u32)` then `max_channels + 2` channels;
  each written channel is `u32 is_written`, then a `u64`-length block of
  `u16 compression` + data. In the reference the last channel is the mask
  coverage (8-bit, `compression 3` = ZIP-with-prediction, document-sized), and
  channels 0–2 are Photoshop's rendered RGB cache.
- A document-level `FMsk` block carries the mask's display `Color` (four `u16`)
  and `u16 opacity`; it is not the coverage.
- The `SoLd.filterFXStyle` flags (`filterMaskEnable` / `Linked` /
  `ExtendWithWhite`) already round-trip.

## Reading

`smart_object.rs` gains a `collect_filter_masks(layer_section_extra, is_psb)`
that walks the global tagged blocks exactly like `collect_linked_records`,
parses `FEid` into `(uuid, rect, coverage)` and `FMsk` into `(color, opacity)`,
and `resolve_smart_objects` attaches the coverage to the smart object whose
`uuid` matches. The channels are ZIP-with-prediction decoded with the codec's
existing zlib helper. A malformed or absent block yields no mask; the bytes are
still preserved verbatim.

## Model

`SmartObject::filter_mask: Option<FilterMask>` where

```rust
pub struct FilterMask {
    pub rect: PsdRect,
    pub data: Plane<u8>,
}
```

`None` means no mask. The `filterFXStyle` flags stay as they are.

## Render

`composite_smart_filtered_source` currently blends the filtered chain output
over the canvas. With a mask it blends, per pixel, `mask/255 · filtered +
(1 - mask/255) · source`, where the source is the unfiltered smart source at
that pixel (the same buffer the chain started from). A pixel outside the mask
rect takes the mask's extend value (white by default, from
`filterMaskExtendWithWhite`). Alpha follows the source, so a masked-out pixel
is exactly the unfiltered source.

## Delete

`remove_filter_mask(layer_section_extra, uuid, is_psb)` mirrors
`remove_linked_source`: it copies every global tagged block verbatim except the
matched uuid's `FEid` entry and, when no `FEid` entry remains, the `FMsk` block;
it re-emits a rewritten `FEid` with `write_tag_document`. The app op clears
`filter_mask_enabled` in the typed view and the preserved `SoLd`, drops the
blocks, recomposites, and records one "Delete Filter Mask" state.

## Deferred

Creating a mask from a selection is not in this change: it requires authoring
Photoshop's `FEid` render-cache channels, which cannot be validated without
Photoshop.
