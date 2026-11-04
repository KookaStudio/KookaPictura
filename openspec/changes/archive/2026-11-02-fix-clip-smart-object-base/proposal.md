# Proposal

## Why

A clipping mask does not confine a clipped layer to a smart-object base's alpha:
the clipped layer stays visible over the base's transparent margin (issue
#1103). A proxy-less smart object decodes its embedded source from the payload's
stored merged composite, but the reader splits that composite into color planes
plus extra channels, so the merged transparency plane is dropped. The source is
then fully opaque, the base paints opaque, and clipping coverage is 1 across the
whole base rect. This change is scoped to that defect: a proxy-less smart object
whose payload's stored merged composite carries a transparency plane. The report
carried no repro, so a base that carried a raster proxy or an opaque merged
composite would be a separate defect that needs the file.

## What Changes

- Preserve the embedded source's merged transparency when decoding a smart
  object without a raster proxy, so the source (and the base that shows it)
  carries the stored merged transparency plane.
- Read grayscale-plus-alpha sources as alpha, not only four-channel RGBA.
- Add a render test for a full-canvas clipped layer over a smart-object base
  with a transparent margin, plus a source-alpha unit test. Cover the GPU path
  where the existing parity harness is eligible.
- A proxied smart object without an enabled smart-filter chain keeps rendering
  from its proxy channels; that path already computes coverage from the proxy
  alpha and does not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/smart-object-rendering`: the stored merged composite is preferred
  together with its merged transparency plane, and a grayscale source's alpha
  plane is honored; the decoded source keeps the stored merged composite's
  merged transparency plane.
- `compositing/clipping-masks`: a clipping base rendered from an embedded smart
  object confines its clipped layers to the base's rendered alpha, including a
  transparent margin inside the base's rect.

## Impact

- `crates/pictura-render/src/layer_source/mod.rs` (`decode_embedded`)
- `crates/pictura-render/src/smart_source.rs` (`scale_smart_source`)
- `crates/pictura-render/src/tests/smart_object_alpha.rs` and
  `crates/pictura-render/src/tests/clipping.rs` (or the GPU parity suite)
- Consumers of `render_smart_source` inherit the fix: compositing, the
  smart-filter chain, rasterize, transform materialization, Pictura Raw, lens
  correction, effect mattes, and the GPU source upload.
- Out of scope: `raster_layer_from_composite`
  (`document_ops/layer_ops/smart_object.rs`) still drops the merged alpha on
  Convert Smart Object to Layers; same root cause, separate follow-up.
- No dependency, codec-format, or public-API change.
