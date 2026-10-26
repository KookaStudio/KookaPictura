# Proposal: smart-filter-masks

## Why

Issue #956 (follow-up to #934). CS6 lets a smart filter stack be limited by a
**filter mask**: applying a smart filter while a selection is active creates a
mask, the filter applies only where the mask is white, and the mask is deleted
from the "Smart Filters" group row's context menu. #934's context menus need
"Delete Filter Mask" to be real.

The mask is stored **document-level**, not on the layer: a `FEid`
(`FilterEffects`) block keyed by the smart object's uuid, whose last channel is
the coverage, plus a `FMsk` block (color + opacity). Both are preserved today
only as opaque bytes in `Document::layer_section_extra`; the model, the render,
and deletion do not exist.

A Photoshop-authored fixture supplied by the project owner (#956) grounds the
format; it is used only as a black-box reference and is not committed (see
`CONTRIBUTING.md`).

## What Changes

- `pictura-codec` parses the `FEid`/`FMsk` blocks out of `layer_section_extra`
  and attaches the mask to the matching smart object by uuid, preserving the
  blocks verbatim for re-save, and can drop a mask's blocks on delete (mirroring
  `remove_linked_source`).
- `pictura-core` gains `SmartObject::filter_mask` (rect + coverage).
- `pictura-render` applies the smart-filter chain through the mask: a masked-out
  pixel keeps the unfiltered source.
- `pictura-app` exposes mask presence and a delete that drops the
  `FEid`/`FMsk` blocks, clears the typed mask, and records one state.

**Deferred** (a follow-up): creating a mask from a selection, which requires
authoring Photoshop's `FEid` render-cache channels.

## Capabilities

### New Capabilities

- `codec/smart-filter-masks`: reading, preserving, and deleting the filter mask
  blocks.
- `compositing/smart-filter-masks`: rendering the smart-filter chain through the
  mask.

## Impact

- `pictura-codec` (`smart_object.rs`, `filter_effects.rs`), `pictura-core`
  (`SmartObject`), `pictura-render` (`smart_filter.rs`, `smart_source.rs`),
  `pictura-app` (`layers_smart_filters.rs`), the Layers panel, tests.
- `docs/dev/doc-code-parity-gaps.md` if a gap is closed.
