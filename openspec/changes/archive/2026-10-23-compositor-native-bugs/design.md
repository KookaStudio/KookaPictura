# Design: Compositor, layer and native colour bugs

## Approach

### #846 Pass Through group vector mask

Add `layer.vector_mask.is_none()` to the pass-through fast-path guard in
`composite.rs` (`composite_layer_inner`) and `gpu/backend.rs`
(`composite_layer`). The isolated fallback already multiplies the group result
by the vector-mask coverage through `mask_alpha_unit`, and the GPU already
uploads a CPU-computed vector mask (`mask_writer`), so no new mask path is
needed.

### #847 opacity and fill semantics

- Effects draw through `blend_parts`; add an `effect_scale` to `Canvas`
  (default 1.0, copied by `with_cover_from`), multiply it in `blend_parts`, and
  set it to `layer.opacity/255` around `composite_layer_effects` and
  `composite_layer_effects_above`. Content already scales in `blend_into_row`.
- `blend_into_row` applies `fill` to every non-group. Adjustment layers and
  fill-content layers both carry an adjustment block, so the distinction is the
  decoded variant: `SolidFill`/`GradientFill`/`PatternFill` keep Fill; every
  other adjustment uses fill 255. Implement the override once in
  `composite_native::composite_adjustment` (a proxy layer passed to
  `gate_adjusted` and the 8-bit loop), not per pixel.
- The GPU adjustment branch also multiplies `params.fill`; dispatch `255` there
  (or ignore `fill` when `adj_kind != 0`) so the GPU agrees with the CPU oracle.
  Fill-content layers stay on the CPU path.
- The Layers panel keeps its current Fill eligibility. Disabling it for
  adjustments needs a new row signal and is a separate UI change.

### #848 effect geometry

- Extract one padding helper from the renderer constants
  (`dilate_radius = round(spread / 100 × size)`, `blur_support`) and use it in
  both `effects_reach` and the drop-shadow/outer-glow renderers. `effects_reach`
  currently omits the spread dilation, which is the damaged band.
- In `content_matte`, when there is no `-1` channel and the layer has an
  embedded smart object, render the source over the requested region with
  `render_smart_source` and use its alpha. Keep the `1.0` fallback when the
  source cannot be rendered; the existing malformed-payload test depends on it.
  ponytail: this decodes the embedded source once per effect; cache the
  rendered alpha if a profile shows it.

### #849 Merge Down in a clip run

`bake_inputs` composites exactly the scratch inputs through
`composite_siblings`, which starts a clipping run from a sibling without
consulting that sibling's own `clipping` flag. For a clipped target that is the
bug: `[lower.clipping, upper.clipping]` makes `lower` its own base, so the real
base's coverage is lost. Adding the real base to the scratch is not the fix: it
would bake the base's pixels into the node and then composite the still-present
base again.

- When the target (`lower`) is clipped, clear `clipping` on both scratch copies
  so the pair composites as ordinary layers, and keep `lower.clipping` on the
  result. The real base stays below the result, so the compositor applies its
  coverage exactly once.
- When the target is the base, leave the clipped source's flag in the scratch
  (the base's coverage is baked because the base is replaced) and leave the
  result unclipped.
- Pass the resulting `clipping` flag through to `pixel_result` (a new parameter;
  `flatten` and the other merge scopes pass `false`); do not hard-code `false`.
- Delete `fold_clipping` from `merge_clipping_mask`: with the compositor's
  clipping pass it applies the base coverage twice, and its comment is stale.
  The scratch of `[base, clipped…]` already yields the clipped composite.

The failing test comes first and pins both the target-is-clipped and
target-is-the-base cases; a partial-alpha Merge Clipping Mask case guards the
fold removal (a 0/1 coverage test cannot tell the double application apart).

### #861 Convert to Profile at native depth

- Add 32-bit float formats to `pictura_color::formats` (`GRAY_FLT`, `RGB_FLT`,
  `RGBA_FLT`); the existing byte-slice transform already accepts any pixel
  format, so no new public API is needed.
- In `convert_document`, after the 8-bit composite/layer conversion, convert
  `doc.source_planes`' three color planes and every layer's (recursively)
  `source_channels` planes with ids `0..3` at the store's own depth (`16` or
  `32`; leave an 8-bit Lab store alone), using the document size for
  `source_planes` and the layer rect for a layer store. Interleave the planar
  planes, pack in the order lcms2's `RGB_16`/`RGB_FLT` expects (native-endian
  `u16`/`f32`, not the big-endian PSD `Samples::to_bytes` order), call
  `pictura_color::convert`, and unpack into the same `Samples` variant. Leave
  `-1`, `-2`, and document extra planes untouched.
- Overwrite each converted channel's 8-bit working plane (composite by plane
  index, layer by channel id) with `narrow_to_u8()` of the converted native
  samples. `write_psd` re-emits a retained native plane only when narrowing it
  equals the working plane; a separately converted 8-bit plane can differ by a
  rounding step and would silently fall back to 8-bit on save.
- Do not let the conversion clamp float samples: lcms2 receives out-of-range
  values clamped to its `[0, 1]` domain, and their originals are restored
  afterwards. The codec test asserts the saved file carries more than 256 levels
  at 16-bit and that out-of-range 32-bit samples are not flattened to `[0, 1]`.
- Layers whose native store is partial (any of `0..3` missing, or no matching
  8-bit working channel) are left unchanged, matching `convert_layer`'s 8-bit
  rule.

## Order

#846 → #847 → #848 → #849 → #861 (highest risk last).

## Risks

- These are golden-image surfaces. Update no baseline without a before/after
  composite assertion; GPU and CPU must agree.
- The 32-bit float transform relies on lcms2 preserving extended-range samples.
  The codec test is written first; if lcms clamps, the fix must convert the
  in-range domain and carry the out-of-range values explicitly rather than
  silently narrowing. The 8-bit working copy is derived from the narrowed native
  result so the writer's retained-plane check holds.
- Removing `fold_clipping` changes `Merge Clipping Mask` pixels when the base
  alpha is partial; `merge_clipping_mask_applies_coverage_once` is the guard
  (the existing split-alpha test is 0/1 and cannot see the double application).
- A clipped target baked without the real base keeps whole-document equality
  for a `Normal` target (the tested case). A non-`Normal` target's blend needs
  the base context, which a single baked node cannot carry; merging the whole
  clipping run with its base is the upgrade path. The scratch composite remains
  exact for the target-is-base case because the base is one of the inputs.

## Constraints

- Follow `docs/dev/testing-conventions.md`: Qt Test for GUI, std `#[test]` for engine.
- Do not weaken or delete an existing test; update a baseline only with a before/after assertion.
- Keep each file under its size cap; split along the seams named above.
