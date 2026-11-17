# Design

## Context

The renderer already has a source-over-proxy path for smart objects: a layer
without a color channel or with decodable Smart Filters renders from the
embedded payload (`composite_route::ContentRoute::SmartFiltered` /
`SmartSource`), and the GPU uploads the CPU raster. `layer_source` caches the
decoded embedded source by payload digest, and `filtered_source` applies the
Smart Filter chain over it. `SmartObject` is a typed model with a preserved raw
`SoLd`/`plLd` descriptor and a private payload. `Feature::StackModes` already
exists as an Extended-only core flag, and the command catalogue carries a
disabled `layer.smart.objects.stack.mode` placeholder row.

The embedded source is a serialized PSD. A converted single layer embeds one
layer; a placed or `Open As Smart Object` multi-layer PSD embeds all its
layers, which is the frame material a stack mode reduces. The Adobe PSD
specification documents placed-layer type `3 = image stack` but no stack-mode
key, so the mode's file representation is ours to choose.

## Goals / Non-Goals

**Goals:**
- Add the reducer and model with no new pipeline: the stack render enters where
  the embedded source already enters, so CPU composite, GPU upload, Smart
  Filters, rasterize, and thumbnails all see it.
- Bound the memory and time of reducing large, many-frame stacks.
- Keep existing proxied and stacked documents rendering byte-identically when
  no mode is active.
- Make the mode survive save/load within Pictura without guessing an
  undocumented Photoshop descriptor key.

**Non-Goals:**
- Multi-layer `Convert to Smart Object`, `Statistics`, `Load Files into Stack`.
- Reading or writing Photoshop's own stack-mode representation.
- A Properties-panel page (the docs mark it inferred).
- Exact CS6 parity for the unsourced numeric edges (entropy scaling, even
  medians, negative moments); the spec records the chosen conventions.

## Decisions

### The stack render is the embedded source, not a new composite body

`layer_source::embedded_source(so)` becomes stack-aware: when
`so.stack_mode` is `Some`, it decodes the payload's document through the shared
helper `decode_embedded` uses — which stamps every nested smart object with
`nesting = so.nesting + 1` — builds frames, reduces, and returns that buffer;
otherwise it keeps the existing decoded merged-composite path. Frames therefore
cannot bypass `MAX_EMBED_NESTING`. Everything downstream is unchanged: scaling
into the layer rect, layer properties, the Smart Filter chain (`filtered_source`
already calls `render_smart_source`), the GPU raster upload, and rasterize.

Alternative rejected: a new `ContentRoute`/`Body` for stacks. It would force a
GPU eligibility decision and duplicate the source/scale/filter plumbing for no
behavioral gain. Instead, `content_route` returns the existing
`SmartSource` route when `stack_source_applies(layer)` — a mode is active and
no enabled Smart Filter fails to decode — (unless `SmartFiltered` already
applies), and `Body::SmartSource` tries the source under the same predicate even
if a proxy exists. An enabled filter that cannot be decoded keeps the normal
proxy path when a proxy exists.

### Frames come from the existing compositor, one layer at a time, in bands

A frame is one visible top-level layer of the decoded payload rendered alone
over transparency. To get it, reuse the region compositor:

- `CompositePlan::build_for_layers(&[layer], doc, clip, effect_region)` gives a
  plan for just that layer, so groups, nested smart objects, masks, opacity,
  and effects behave exactly as they do in a normal composite.
- A new `composite_region::composite_layer_region(layer, doc, x0, y0, w, h)`
  mirrors `composite_rgba_region` with that one-layer plan and returns the
  layer over transparency for the region.

The source is processed in horizontal bands sized so that
`frame_count * rows * width * 4` stays inside a fixed budget (32 MiB, at least
one row, at most 256 rows). Each band composites every frame, calls
`reduce_stack`, and writes the output band. This keeps peak memory near one
band's frames instead of `frame_count` full RGBA copies.

Alternatives rejected: gathering raw per-layer channels (must re-implement
rect translation, masks, groups, type, nested objects), and materializing full
frames (a 20-frame 24 MP stack is ~1.9 GB).

### The reducer is a small core kernel over RGBA frames

`pictura-core::stack_mode` adds `StackMode` and
`reduce_stack(frames: &[PixelBuffer], mode: StackMode) -> PixelBuffer`. The
kernel is per pixel: collect the non-transparent samples per channel, apply the
mode, round and clamp, set alpha to the maximum sample alpha. Median sorts the
sample set; moments use the docs' `|S| - 1` denominator; entropy builds a
256-bin histogram and scales bits by `255 / 8`.

`StackMode` carries stable one-byte codes used only by the codec marker. The
renderer works on the existing 8-bit RGBA pipeline, so 16/32-bit payloads are
reduced after the existing conversion; the spec marks that approximation.

### The proxy stays pre-stack, and the mode is a private block

Applying a mode does not rewrite the layer's channels. That preserves the
pre-stack pixels for `None` and for readers that do not understand the mode.
Only applying a stack mode leaves the proxy untouched; an existing filter apply
such as `apply_pictura_raw` still bakes its filtered current source into the
proxy, as before. The mode is written as a `PiSt` layer block: version `1` and
a mode code (`1` Maximum, `2` Minimum, `3` Mean, `4` Median, `5` Summation, `6` Range,
`7` Variance, `8` Standard Deviation, `9` Skewness, `10` Kurtosis, `11`
Entropy). The reader consumes a recognized block into
`SmartObject.stack_mode` (so it is not duplicated as an unknown block); an
unrecognized version or code stays in `extra_blocks` and re-emits verbatim. The
writer authors the block from the model, and clears it when the mode is
`None`. It emits at most one `PiSt` per layer: a preserved unrecognized marker
is re-emitted only when no mode is modelled, and is replaced when the writer
authors one.

Alternatives rejected: a guessed `SoLd` descriptor key (undocumented; a wrong
key could be misread by Photoshop), baking the reduced pixels into the proxy
(then `None` could not revert), and memory-only state (loses the
non-destructive edit across save).

### Caching, routing, and rasterize

- `embedded_key` gains the stack mode, and `chain_hash` gains it too, so the
  embedded source, the filtered source, the composited raster, and the damage
  path all key on the active mode. If the stack cannot be built, the build
  closure falls back to `decode_embedded` inside the same key, so a failed
  stack never caches `Missing` and masks the fallback.
- `content_route` returns `SmartSource` when `stack_source_applies(layer)`;
  that shared predicate is used by the route and `Body::SmartSource`. It holds
  only when a mode is active and no enabled Smart Filter fails to decode; an
  undecodable enabled filter keeps the normal proxy path. `Body::SmartSource`
  prefers the source over a proxy only when the predicate holds. The GPU keeps
  its existing `SmartSource` handling.
- `rasterize_smart_object` materializes from `render_smart_source` when
  `stack_source_applies(layer)`, even if a proxy exists; otherwise a proxied
  layer keeps its proxy and a channel-less layer falls back to the embedded
  source. It then drops the object and the marker.

Nested smart objects inside frames are decoded through the shared decode helper,
so their `nesting` is `so.nesting + 1` and the `MAX_EMBED_NESTING` check applies
before any frame composition; they resolve through their own cache keys (payload,
nesting depth, and their own mode). The existing "a build never waits on itself"
rules cover cache re-entry.

### The command surface is 12 catalogue leaves

Replace the `layer.smart.objects.stack.mode` placeholder with 12 rows under
`Layer / Smart Objects / Stack Mode`. Their path-derived ids are stable
(`...stack.mode.none`, `.entropy`, ..., `.standard.deviation`). They are
checkable and `implemented: true`; `command_edition` maps every id to
`Edition::Extended`, so `addDefaultCommands` omits them in Standard. The C++
wiring lives in `menu_smart_object.cpp`, with one table of
`{command id, mode code, label}`: each leaf gets a handler, an enabled
provider, and a checked provider. The cxx-qt bridge exposes the current mode
code, eligibility, and one apply operation that sets/clears the mode,
recomposites, and records `Stack Mode: <label>`; a refusal records nothing.

## Risks / Trade-offs

- **Unsourced numeric conventions** (entropy scaling, even median, negative
  skew) → encode the chosen conventions in the spec and tests; do not claim
  Photoshop-verified parity beyond the documented formulas.
- **Large stacks can be slow** (median sorts per pixel) → the band budget
  bounds memory; the result is cached per mode; no new parallelism is required
  for correctness.
- **Photoshop interop** → Photoshop preserves the unknown `PiSt` block but
  displays the proxy/merged content rather than the stack render. Record this
  in the parity register; a future change can read Photoshop's real key.
- **Band edges and effects** → compositing each band through
  `composite_rgba_region`-equivalent region semantics keeps effects exact at
  band boundaries; a test compares a banded stack render with a small
  whole-source stack.
- **Frames with clipping or blend interactions** → the spec defines a frame as
  the layer rendered alone over transparency; clipping-pair semantics are not
  part of this change.
- **Equality and undo** → `SmartObject::PartialEq` must include the mode or a
  mode change would look like a no-op to history snapshots.
- **Command-id churn** → removing the placeholder and adding 12 rows requires
  regenerating `command_ids.h`; the placeholder had no handler or const name.
- **Extended gating scope** → `command_edition` must list all 12 ids or the
  submenu leaks into Standard; the edition test checks absence.
- **Core file-size headroom** → `crates/pictura-core/src/lib.rs` is 1190 lines
  against the 1200 hard cap in `scripts/check-file-size.sh`; keep the core edit
  to the field, the `Default` arm, the `PartialEq` arm, and module/export lines,
  or split `SmartObject` into its own module before adding to it.

## Migration Plan

The block and field are additive. Existing documents carry no `PiSt` block and
report `None`, so their composite is unchanged. A document saved with a mode
reopens in Pictura with the mode; an older Pictura build preserves the block as
unknown and renders the proxy. Rolling back is safe: the block is ignored by
readers that do not know it, and the proxy is never rewritten.
