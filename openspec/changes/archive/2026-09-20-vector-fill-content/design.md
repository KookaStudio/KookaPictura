## Context

The codec preserves every unmodeled per-layer tagged block in
`Layer.extra_blocks` (`crates/pictura-codec/src/read.rs` `_ => extra_blocks.push`,
re-emitted by `write_extra`), so `vscg` and `vmsk` survive open→save today. The
just-shipped `vector-mask-render` change decodes `vmsk` and clips the layer; the
fill content itself is still not decoded.

A shape layer is normally `vscg` (the fill content) plus `vmsk` (the outline).
`pictura-render` already owns typed fill decoders and generative fill
composites: `decode_solid_fill` (`composite.rs`), `decode_gradient_fill` /
`decode_pattern_fill` / `composite_solid_fill` / `composite_gradient_fill` /
`composite_pattern_fill` (`fill.rs`), all reached from `Adjustment::SolidFill` /
`GradientFill` / `PatternFill`. Layer effects gate their coverage through
`fill_coverage_matte` (`fill.rs`), which decodes the layer's adjustment block.
The GPU has no fill shader, so any `SoCo`/`GdFl`/`PtFl` layer already forces a CPU
fallback (`gpu/mod.rs::check_supported`).

### Grounding

**`vscg` block layout** (ag-psd `additionalInfo.ts:198` `vscg` handler;
psd-tools `psd/vector.py:356` `VectorStrokeContentSetting.read` / `.write`):

- A big-endian 4-byte fill key,
- then a `u32` version equal to `16`,
- then the descriptor body (classID + item count + items).

psd-tools reads `4sI` (key + version) then the descriptor body, and writes the
same; ag-psd's `vscg` reader does `readSignature` (4 bytes) then
`readVersionAndDescriptor`. The two agree.

**Nested content** (ag-psd `descriptor.ts:1684` `parseVectorContent`,
`:1727` `serializeVectorContent`): the descriptor body is the same shape as a
top-level fill block. The 4-byte key is one of `SoCo` / `GdFl` / `PtFl`, and
`parseVectorContent` dispatches on the descriptor's **content** — `'Grad' in d` →
gradient, `'Ptrn' in d` → pattern, `'Clr ' in d` → color — not on the key.

**Empirically confirmed against a real Adobe file.** A reference shape layer
(`Ellipse 1`, kind `shape`) in `assets/test_with_smart_object01.psd` and
`...02.psd` carries a `vscg` whose psd-tools type is
`VectorStrokeContentSetting` with `key == b"SoCo"`, `version == 16`,
`classID == b"null"`, and one item `Clr ` (classID `RGBC`) holding `Rd `/`Grn `/`Bl `
`Doub` `{0.0, 0.0, 0.0}`. Our `decode_solid_fill` checks only for a `Clr `
object with those three keys, so it decodes this unchanged (it does not enforce
the classID).

**Correction to the brief.** `solidColorLayer` / `gradientLayer` /
`patternLayer` are the descriptor class IDs of the **stroke** descriptor's
`strokeStyleContent` field (ag-psd `descriptor.ts:62`, `:455`), which lives in
the `vstk` block. The `vscg` fill block's own descriptor classID is `null`, and
its content is a plain `Clr ` / `Grad`+`Type` / `Ptrn` object. So the vector
stroke (`vstk`) is the piece that carries those class IDs and is the deferred
kind; the vector fill reuses the existing fill descriptors directly.

**Fixture authorability.** psd-tools' typed `VectorStrokeContentSetting(key=…,
version=16, items=…)` writer plus `VectorMaskSetting` round-trips byte-stably
(`scripts/generate-fixtures.py` style), and psd-tools' own compositor renders the
authored shape layer (inside the `vmsk` rectangle: the fill colour; outside: the
base), so the fixture is ground-truthable by an independent implementation.

## Goals / Non-Goals

**Goals:**

- Decode the preserved `vscg` block into the existing `Adjustment::SolidFill` /
  `GradientFill` / `PatternFill` variants, reusing the fill decoders unchanged.
- Composite a shape layer's vector fill through the said generative fill path,
  clipped by the already-shipped vector-mask coverage.
- Keep the raw `vscg` block as the serialization source of truth; no writer or
  model change.
- Prove the decode with psd-tools and ag-psd over a committed fixture, and prove
  the clip with a render test.

**Non-Goals:**

- The vector stroke (`vstk`, `StrokeDescriptor`, `strokeStyleContent`), `vogk`,
  `vsms`, noise gradients, boolean ops beyond union, text, antialiasing.
- A new `Adjustment` variant, a core model field, a codec module, or an encoder
  for `vscg`. Authoring and editing a shape layer are deferred.
- Rasterizing a `vscg` shape layer.
- A GPU fill shader.

## Decisions

### D1. Decode `vscg` in `pictura-render`, not in the core model or the codec

Unlike `vmsk` coverage (sampled per pixel, so the vector-mask change put the
flattened view in `Layer.vector_mask`), a `vscg` fill is decoded once per layer
composite, and `pictura-render` already reads `extra_blocks` directly for layer
effects (`layer_effects/*.rs`). `pictura-core` has no dependencies, so a typed
derived view there would need a new `pictura-adjust` edge; a codec-module decode
could not reuse the renderer's fill decoders without inverting the crate
dependency. Decoding in render is the smallest change that reuses the existing
decoders.

Alternative rejected: a `Layer.vector_fill` derived view in core. It adds a field
to every exhaustive `Layer { … }` literal (~32 sites) and a new model parse for
no per-pixel benefit. Alternative rejected: decode in the codec and store an
`AdjustmentData`. Same model churn, and the decoder still cannot live in the
codec.

### D2. `decode_vector_fill` skips the 4-byte key and reuses the fill decoders

`decode_vector_fill(d: &[u8]) -> Option<Adjustment>` slices `d[4..]` (the key is
a redundant tag; ag-psd ignores it on read), parses it with
`pictura_codec::read_descriptor`, and dispatches on the descriptor's content to
`gradient_params_from_desc` / `pattern_params_from_desc` / `decode_solid_fill` —
the same helpers the top-level blocks use, so the layouts cannot drift. Content
dispatch, rather than key dispatch, mirrors the one independent decoder
(`ag-psd parseVectorContent`) we use as an oracle. A version other than 16, a
non-object descriptor, or none of `Grad`/`Ptrn`/`Clr ` is `None`. Never panics.

`decode_solid_fill` is made `pub(crate)` so `fill.rs` can call it; its behavior
is unchanged.

### D3. One helper decides a layer's fill, used by composite and effect coverage

`decode_layer_fill(&Layer) -> Option<Adjustment>` returns
`layer.adjustment.as_ref().and_then(decode_adjustment)`, else
`layer.extra_block(b"vscg").and_then(|b| decode_vector_fill(&b.data))`. The
compositor's layer dispatch and `fill_coverage_matte` both call it, so a shape
layer's fill renders and its effects are gated by the same content. A modeled
adjustment block always wins; `vscg` is consulted only when the layer has no
adjustment block, so existing files are unchanged. An adjustment layer with an
undecodable key stays a no-op and is never treated as pixels (the dispatch keeps
that branch).

Alternative rejected: inline `layer.extra_block(b"vscg")` at each site. Two call
sites would have to agree on the block layout; one helper is the codebase's
established "single decoder" pattern (`is_fill_content_layer` and
`rasterize_fill_content` already share `decode_adjustment`).

### D4. Clipping comes free from the shipped vector-mask coverage

The vector fill composites through `composite_solid_fill` /
`composite_gradient_fill` / `composite_pattern_fill`, which blend each in-rect
pixel through `blend_into`; `blend_into` multiplies the source alpha by
`mask_alpha`, which already folds in the `vmsk` vector coverage. So a shape
layer's fill is clipped by its outline with no new geometry. With no `vmsk` or a
disabled/no-closed-subpath mask, coverage is 255 and the fill covers the layer
rect — the documented fallback. The fixture proves the clip.

### D5. GPU falls back to the CPU for a vector fill

`check_supported` currently rejects a layer whose adjustment block does not map to
a GPU shader. A `vscg` shape layer has no adjustment block, so the GPU would run
it as a channel-less pixel layer and disagree with the CPU. `check_supported`
gains a check that a visible layer carrying a `vscg` block returns
`UnsupportedAdjustment`, forcing the CPU fallback exactly as the shipped fill
layers do. No shader is added; the CPU remains the oracle (rule 6).

### D6. Fixture authored with psd-tools' typed writers; two independent oracles

`scripts/generate-fixtures.py` gains `vector_fill()`: an 8×8 RGB document with a
`Base` pixel layer and a document-sized `Shape` layer whose `vscg` is a `SoCo`
solid fill and whose `vmsk` is a closed `(1,1)-(5,5)` rectangle. `vscg` is a
`VectorStrokeContentSetting(key=b"SoCo", version=16, items={b"Clr ": Descriptor({
b"Rd ":…, b"Grn ":…, b"Bl ":…}, classID=b"RGBC")})` attached under
`Tag.VECTOR_STROKE_CONTENT_DATA`; the `vmsk` uses the existing `_closed_rect_path`
recipe under `Tag.VECTOR_MASK_SETTING1`. The inner `Clr ` **must** carry classID
`RGBC` or psd-tools' own compositor rejects it (`KeyError: b'null'`); our decoder
does not require it. The fixture registers as `vector_fill.psd`, is byte-stable
across regeneration, and leaves the existing fixtures unchanged.

The psd-tools oracle (`crates/pictura-codec/tests/psd/vector_fill_oracle.rs`, new;
`oracle.rs` is 1398/1400 lines) asserts the shape layer's `vscg` key/version and
`Clr ` components, that the layer kind is `shape`, that the asserted pixels match
psd-tools' composite, and that a whole-`Document` round trip preserves the block.
The ag-psd test (appended to `crates/pictura-codec/tests/psd/agpsd_oracle.rs`)
asserts `layer.vectorFill` decodes to the solid colour. Both self-skip when
`node`/`ag-psd` is absent.

The gradient path is proven without a second committed fixture: a render unit
test builds a `vscg` payload as the 4-byte key `GdFl` followed by
`encode_gradient_fill(…)`'s bytes and asserts it decodes and composites, so the
gradient reuse is covered by the existing encoder/decoder.

### D7. No app change and no self-test code

The app renders through `composite_rgba`/`composite_active`, so the fill appears
with no bridge, CPP, panel, or self-test change. The runnable check is the render
test (rule 2), so no `ST_FAIL` code is consumed and `CMakeLists.txt` is untouched.

## Risks / Trade-offs

- **The brief's nested-content names were wrong.** `solidColorLayer` etc. are the
  stroke descriptor's class IDs, not the `vscg` fill's. Mitigation: grounded the
  real block three ways (ag-psd, psd-tools, a real CC file) and recorded it here;
  the fill reuses the existing decoders.
- **A wrong-but-parseable descriptor renders a wrong fill** where it previously
  no-op'd. The existing decoders' type/range checks bound the damage and the
  fixture pins the known value; the behavior matches the shipped top-level fills.
- **A hand-built `vscg` extra block is decoded only through `read_psd`/the
  fixture**, since it lives in `extra_blocks`. Tests decode through the fixture
  (or build `Layer.extra_blocks` explicitly), matching the layer-effects tests'
  posture.
- **Effects coverage changes for shape layers.** An effect on a `vscg` shape
  layer is now gated by the shape's fill instead of the whole layer rect.
  Mitigation: this is the intended "content" semantics; recorded here.
- **The GPU must not silently render a channel-less vector fill.** Mitigation:
  D5's one-line `check_supported` rejection keeps the CPU/GPU parity contract.
- **A version-other-than-16 or unknown-content descriptor is a no-op.** Proven
  preserved and inert by a malformed-block unit test.
- **No Photoshop pixel parity claim.** The oracle proves structure and the
  psd-tools composite; rasterization parity is not claimed.

## Migration Plan

None for documents: existing files read as before, and a `vscg` that previously
no-op'd now renders. No golden fixture changes (the new fixture is additive) and
no rollback beyond reverting the commit.

## Open Questions

- **`vstk` (vector stroke) scope.** Confirmed present as a distinct block and
  deferred; whether a CS6 shape's outline stroke ever needs it rather than an
  `lfx2` stroke is unresolved.
- **`operation` and holes in real shape paths.** Inherited from the vector-mask
  slice: all closed subpaths contribute under one fill rule; holes encoded via
  subtract operations do not cut.
- **Whether a shape layer can carry both an adjustment-block fill and `vscg`.**
  The design prefers the adjustment block; the real fixture carries only `vscg`.
  A CS6 capture would settle it.
