# Design

## Context

`#1200` (commit `65c37a2c`) flipped the PSD raster-mask flag polarity: bit 0 set
means unlinked, clear means linked. It updated `add_layer_mask`, vector-mask
rasterization, and Auto Blend, but not
`cxxqt_object/helpers_composite.rs::default_layer_mask`, whose reveal-all branch
still writes `flags: 1` from the pre-#1200 spelling of "linked". Every default
fill/adjustment mask with no selection therefore starts unlinked.

## Goals / Non-Goals

**Goals:**

- Make every `default_layer_mask` branch produce a linked mask.
- Keep the PSD flags byte authoritative; no new model field.

**Non-Goals:**

- Rewriting flags in existing documents. The fix applies only to masks created
  after it lands.
- Changing the default-mask preference or when a fill/adjustment layer gets a
  mask.

## Decisions

### Use the linked default instead of a raw flag value

Drop `flags: 1` from the reveal-all branch; `..Default::default()` supplies
`flags: 0`, matching the selection branch and `add_layer_mask`. A comment names
`MASK_FLAG_UNLINKED` so a future reader does not reintroduce a raw literal.
Callers (`add_fill_layer`, the adjustment-layer path, `impl_layers_rasterize`)
inherit the fix without changes.

## Risks / Trade-offs

- **A mask imported from an older Pictura save stays unlinked.** → Intended:
  stored flags are not rewritten; the user can link it from the glyph.
