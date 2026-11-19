# Design

## Context

See `proposal.md` — Why.

A layer record's additional-layer information is preserved verbatim as
`LayerBlock`s. `PxSD` is already in `PSB_BIG_KEYS` (`crates/pictura-codec/src/common.rs`),
so both the PSD `u32` tag length and the PSB `u64` tag length are read and
written correctly. The block's own leading `u64` payload length stays inside the
preserved bytes.

Auto-Blend's availability is decided by `can_auto_blend`, which refuses a
selection when any selected path resolves to a `refused` layer. `refused`
currently covers Background, Smart Object, and clipped layers; `eligible`
separately filters out groups, adjustments, fills, type, smart objects, hidden
layers, and empty layers.

## Goals / Non-Goals

**Goals:**

- Detect the two layer kinds from a verified PSD block and disable Auto-Blend
  when one is selected.
- Keep detection honest and local: the layer still behaves as the raster content
  Kooka already renders, and its `PxSD` bytes still round-trip.
- Prove detection from bytes that have gone through the codec, not only from a
  hand-built layer.

**Non-Goals:**

- Modeling video or 3D content, rendering it differently, or adding
  `LayerKind::Video`/`LayerKind::ThreeD`.
- Guessing undocumented CS6-era keys. `PxSD` is the only key the public
  specification ties to 3D or video layers.
- Refusing these layers in other commands. The issue is scoped to Auto-Blend.

## Decisions

### D1. Detect `PxSD` only

The public Adobe PSD specification names `PxSD` ("Pixel Source Data", Photoshop
CC 2015) as raw data for 3D or video layers. The older `PxSc` block is a
descriptor form with no documented tie to 3D/video in the same specification, so
it is not used. The predicate lives on `Layer` so the key is written once and
the render crate does not duplicate PSD knowledge.

### D2. Refuse, do not skip

`refused` is the right set: Photoshop disables Auto-Blend when a selected layer
is a video or 3D layer, just as it does for a Smart Object or Background. Adding
the predicate to `refused` also gives the required "outside the selection does
not matter" behavior for free, because only the selected paths are checked.
Adding it to `eligible` instead would silently skip a selected video layer and
still blend the rest, which the contract does not allow.

### D3. Keep the raster model

No new `LayerKind` variant is introduced. A video or 3D layer is still
`LayerContent::Pixel` with its `PxSD` block preserved; only the availability
predicate changes. This keeps the change reversible and avoids threading a new
kind through every exhaustive match.

## Risks / Trade-offs

- CS6-era 3D/video files that do not carry `PxSD` remain undetected. The public
  specification offers no verified alternative key, so this change does not
  invent one. Such files continue to blend as raster pixels, as before.
- A future full model would replace the predicate with a real kind; the call
  site in Auto-Blend stays the same.

## Migration Plan

None. Revert the code and test diff and remove the archived change folder.

## Open Questions

None.
