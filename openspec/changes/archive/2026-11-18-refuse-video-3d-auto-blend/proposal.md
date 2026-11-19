# Proposal

## Why

Issue #1110. `docs/05-layers/align-and-distribute.md` states that Auto-Blend
Layers does not work with video layers or 3D layers, but Kooka has no model for
either kind. They load as ordinary raster layers and are blended as pixels. The
engine already preserves their additional-layer-information blocks, so the
missing piece is detection.

Adobe's public PSD file-format specification documents the `PxSD` block as
"Pixel Source Data" whose payload is "raw data for 3D or video layers". That is
the verified key this change uses; the two kinds share it, and both are refused.

## What Changes

- Add a `Layer::is_video_or_3d()` predicate that reports whether the layer
  carries the preserved `PxSD` pixel-source block.
- Treat such a layer like a Background or Smart Object in Auto-Blend
  availability: selecting one disables the command and yields an empty plan.
  A video or 3D layer outside the selection does not disable it.
- Add an engine refusal test and a codec round-trip test proving `PxSD` survives
  read/write and is recognized after loading.
- No new `LayerKind` variants: the layers continue to load and save as raster
  content with their original blocks preserved.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/auto-blend-layers`: Auto-Blend availability also refuses selected
  video and 3D layers, detected by the `PxSD` pixel-source block.

## Impact

- `crates/pictura-core/src/layer_content/mod.rs`: one predicate on `Layer`.
- `crates/pictura-render/src/document_ops/layer_ops/auto_blend/mod.rs`: refusal
  path and the stale "no video or 3D refusal" note.
- Tests in `auto_blend/tests.rs` and `crates/pictura-codec/tests/layer_content.rs`.
- No docs, roadmap, dependency, or build-system changes.
- Files that predate `PxSD` or use an undocumented key remain undetected; the
  public specification exposes no other 3D/video layer key, so none is guessed.
