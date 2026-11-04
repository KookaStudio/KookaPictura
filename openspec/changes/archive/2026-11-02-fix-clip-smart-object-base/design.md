# Design

## Context

See proposal.md — Why. Two facts shape the approach:

- `read_psd` stores the merged image data as color planes plus document-level
  extra channels (`read.rs` `split_planes`). A transparent flattened PSD is
  therefore `composite.channels == color_channels` with the transparency plane
  in `doc.channels`.
- `layer_source::decode_embedded` prefers the stored merged composite and
  returns it as-is, so the extra transparency plane never reaches
  `smart_source::scale_smart_source`, which falls back to alpha 255.

`document_ops::materialize_merged_composite` already resolves the same
ambiguity for layer-less documents by taking `doc.channels.first()` as the
merged alpha when the composite is 1 or 3 channels. The fix follows that
precedent rather than changing the codec's composite/channel split, which is
load-bearing for lossless re-save and round-trip tests.

## Goals / Non-Goals

**Goals:**
- The decoded smart-object source keeps the merged transparency of its payload.
- One central fix; every consumer of `render_smart_source` inherits it.
- No change for a proxied smart object that renders from its proxy channels, or
  for payloads whose merged composite carries no extra plane. Source-rendering
  consumers (smart filters, Pictura Raw, lens correction, effect mattes) now use
  the source alpha.

**Non-Goals:**
- Changing the codec's document model (`composite` stays color-only; extra
  merged planes stay in `doc.channels`).
- Recovering transparency that the payload does not store.
- Recovering transparency from the source layers when the stored merged
  composite is opaque; `convert_to_smart_object` writes such a payload (opaque
  composite, alpha kept on the layer, masked today by the proxy), and
  `raster_layer_from_composite` still drops the merged alpha on Convert Smart
  Object to Layers. Both are the same root cause and are follow-ups, out of
  scope here.
- Resolving the transform/`Trnf` ceiling; placement stays nearest-neighbour in
  the layer rect.

## Decisions

### D1. Attach the merged alpha in `decode_embedded`, not in the codec

`decode_embedded` builds the source buffer for compositing. When the stored
composite has 1 or 3 channels and the decoded document has a full-size first
extra channel, return a buffer with that plane appended as alpha. A depth-1
Bitmap composite keeps its extra planes packed, so it is skipped. Alternative
considered: fold the transparency into `PixelBuffer.channels` at read time.
Rejected because it changes the read/write round-trip contract and the
`extra_selection_channel_round_trips` behavior for every document, not just
embedded sources.

### D2. The first extra merged plane is the transparency

The merged image data carries color planes then alpha planes in channel order;
the first extra plane is treated as the flattened transparency. This matches
`materialize_merged_composite`. Alternative considered: require a channel id of
`-1`. Rejected because `split_planes` renumbers merged extras from 0 and the id
is not available for merged data.

### D3. Honor `ch == 2` in `scale_smart_source`

Appending the alpha plane makes a grayscale source two channels. The current
alpha read only accepts `ch >= 4`, so grayscale sources would stay opaque.
Read alpha from plane 1 when `ch == 2`. RGB stays `ch == 4`; a three-channel
source stays opaque.

### D4. Test at the source and at the clipping group

A fast unit test asserts `render_smart_source` preserves the payload's alpha.
The issue's render test builds a proxy-less smart-object base with a transparent
margin and a full-canvas clipped layer, then asserts the clipped colour appears
only in the opaque area. A proxy-base variant pins the existing proxy coverage.
The GPU path reuses the same CPU `build_sibling_group`, so the CPU assertion is
the oracle; add a GPU parity case where the existing harness is eligible.

## Risks / Trade-offs

- A payload whose first extra plane is a saved selection or spot plane rather
  than transparency would gain a spurious alpha → matches the existing
  `materialize_merged_composite` convention; the ambiguity is inherent to the
  codec model and can be revisited if a real file shows it.
- Treating a grayscale two-channel source as gray+alpha assumes the second plane
  is alpha → same merged-plane convention; a grayscale payload whose extra plane
  is a saved selection hits the same ambiguity.
- Decode cost is one extra plane copy; the buffer gains that plane only when a
  transparency plane exists.

## Migration Plan

No data or file-format migration. Existing proxied files render unchanged;
proxy-less files with transparency start rendering the alpha they always stored.
A proxy-less payload whose first extra plane is a saved selection or spot plane
now renders that plane as alpha; this is the documented approximation, not a
format change.

## Assumption / limitation

The report carried no repro file. This change targets the code-level defect: a
proxy-less smart object whose payload's stored merged composite carries a
transparency plane in the first extra merged channel. If the reported base
carried a raster proxy, the proxy path already used the proxy's own alpha and
was self-consistent; if its merged composite was opaque, there is no plane to
recover. Either case would be a separate defect that needs the file.
