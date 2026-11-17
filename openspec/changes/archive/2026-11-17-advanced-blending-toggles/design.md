# Design

## Context

See `proposal.md` for motivation. The current state that shapes the approach:

- `Layer` already carries `knockout`, `blend_clipping`, `blend_interior` and the typed Blend If view (`crates/pictura-core/src/lib.rs:1093-1103`), and the codec round-trips `knko`/`clbl`/`infx` (`crates/pictura-codec/src/read.rs:1013-1015`, `advanced_blending.rs:68-77`). `clbl` and `infx` are never read by the renderer.
- `tsly`, `lmgm`, `vmgm` and `brst` currently fall into `extra_blocks` and are re-emitted verbatim; the engine neither models nor renders them.
- The CPU pipeline is `composite_layer_inner` (`crates/pictura-render/src/composite.rs:234`) with below effects, a body dispatch, then above effects. Clipping groups are isolated in `composite_clipping.rs:42-104`; knockout replaces covered pixels in `composite_knockout.rs:92-131`; effect passes are prebuilt in `layer_effects` and replayed (`layer_effects/planes.rs`).
- Every effect builder multiplies its source matte by both layer and vector masks (`layer_effects/mod.rs:1096`); that is the flag-off behaviour the new mask-hide switches reuse.
- The GPU planner declines a stack per layer in `gpu/plan.rs:331-338`; render-session backdrops are keyed by `layer_digest` (`render_session.rs:332-345`).
- The Layer Style dialog is out of scope (#695, #581). This change adds only the scalar authoring keys the dialog will bind to.

## Goals / Non-Goals

**Goals:**

- Make the five toggles first-class model, codec and CPU-render behaviour.
- Keep every default document byte-identical: `clbl` on, `infx` off, `tsly` on, both mask-hide flags off, no restricted channels.
- Keep the GPU result authoritative by declining any non-default stack.
- Keep the new render seams small enough for the existing module size caps.

**Non-Goals:**

- No Layer Style dialog or any other Qt/C++ change.
- No Photoshop parity claim. The specs pin the documented/probed behaviour; there is no local Photoshop oracle.
- No group-effect rendering (groups still carry the flags but their effects stay unrendered).
- No CMYK/Lab channel-restriction rendering; the indices round-trip but are inert.
- No legacy `lmgm`/`vmgm` link-status fallback for files that omit the tags.

## Decisions

### D1. Correct `infx` before implementing the grouped path

Absent or `0` means off; byte `1` means "Blend Interior Effects as Group"; the writer emits `infx` only when true. `Layer::blend_interior` defaults to `false`.

Why: Photoshop, psd-tools (`== 1`), the repo docs ("Off by default") and an externally calibrated implementation all agree. Alternate considered: keep the field default and invert its meaning. Rejected because implementing the grouped path against a default-true field would silently change every existing document.

### D2. Model the new fields with PSD-shaped names

`transparency_shapes: bool` (default true), `layer_mask_hides_effects: bool` and `vector_mask_hides_effects: bool` (default false), `restricted_channels: Vec<u32>` (default empty; preserved in file order on read, written sorted and deduped ascending). Add `BlendOverride.no_curve` for the grouped blend.

Why `Vec<u32>` rather than `[bool; 4]`: `brst` may name channels the compositor does not render (spot channels, CMYK indices), and the repo preserves what it does not model. A list round-trips them; the renderer maps indices 0/1/2 to R/G/B and index 0 to gray.

### D3. Channel Include is a restore-once wrapper

At the sibling entry (`composite_layer`) and around the clipping-group merge: save the pre-contribution canvas, composite the layer (content + below/above effects + knockouts), then for each restricted channel set `out[c] = dst[c] * dst_alpha / out_alpha`. If every rendered colour channel is restricted, skip the layer entirely (effects and alpha included). When `out_alpha` is zero the layer has no contribution and the pixel is left unchanged. Non-RGB documents keep the list for round-trip but apply no exclusion. The knockout-base path also goes through the wrapper so a restricted background is reflected in the base.

Why once at the end: applying the same formula after each pass telescopes to the same result while the channel is never modified, so one wrapper covers content, interior effects, exterior effects and adjustments with a much smaller diff than per-kernel gating. The engine stores straight colour, so the formula is evaluated on straight values. Alternate considered: per-pass gating in `blend_into_row_ctx` plus every effect replay; rejected because effect passes bypass `BlendCtx` and the wrapper is the single choke point.

The wrapper uses a full-region canvas clone only when the list is non-empty; a `ponytail:` note records the ceiling.

### D4. `tsly` off makes the effect shape the whole canvas

`effect_source_rect(layer, doc)` returns the document canvas when `tsly` is off and the layer rect otherwise. `content_matte` fills `1.0` over the requested region in that mode. Overlay coverage widens to the canvas; drop shadow, outer glow, stroke and inner shadow are suppressed; satin, inner glow and bevel contribute no pixels because the canvas matte has no interior edge. The knockout cover becomes the whole canvas. The layer and vector masks keep their role from D5; `tsly` changes only the pixel-alpha shape. `paint_bounds` returns the canvas when `tsly` is off and an effect or knockout is active, so style-edit refresh and damage cover the whole canvas.

Why: this is the behaviour calibrated against Photoshop probes. Alternate considered: layer bounds as an opaque matte (the docs-literal reading). Rejected by the user in the exploration; it would produce a rect-shaped shadow/overlay instead of the vanishing effects Photoshop shows.

Ceiling: a partial-region composite can see an artificial edge at the requested region boundary when `tsly` is off. Record it as a limitation; region composites of this toggle are not a parity target.

### D5. Mask-hide flags switch a mask's role

`apply_mask` consults the flags: a mask whose hide flag is on no longer multiplies the source matte. The effect source box must also ignore that mask: `clip_rect`/`fill_content_box` narrow a fill or shape layer to its confining mask, so the box helper must skip a hide-flagged mask. After passes are built, `apply_output_masks` multiplies each pass alpha by that mask over the pass rectangle. The two masks switch independently. Folded effect outputs are output-masked before folding; the content is masked in the inner body, the group blend disables the mask, and the fold outputs are already output-masked.

Why the role switch rather than "shape always, clip additionally": the calibrated probe shows the on state removes the source shaping too (no stroke at the cut, nothing beyond it). Alternate considered: source shaping always plus output clip; simpler but observably different at hard mask cuts.

Cache: `transparency_shapes` and both hide flags join `effect_digest`, because they change the built passes.

### D6. Clipped layers keep their own blending when `clbl` is off

In `composite_siblings_range`, when the base's flag is false: composite the base with its own context, compute its content coverage, then for each visible member composite onto a canvas copy with the member's own context and lerp back by coverage. Destructive clipped adjustments keep the existing recolour-inside-coverage treatment.

Why: this is the repo docs' contract ("each layer keeps its own mode and appearance"). Alternate considered: Patchy's observed Photoshop behaviour where the base's inheritance is largely retained. Rejected as the primary model because the docs are the repository contract and the external probe leaves the non-Normal-base case unprobed. Record the divergence as a ceiling.

### D7. `infx` on groups content with a recursive `skip_effects` pass

Split `LayerPasses` into `below`, `fold`, `above`; fold is Pattern/Gradient/Color Overlay, Satin, Inner Glow; above is Inner Shadow, Stroke, Bevel. That split matches the existing build order, which already emits the fold set as a contiguous prefix. The split is cross-cutting: update the GPU backend replay, the GPU dissolve scan, the GPU pass metric, the region fast path and `LayerPasses::size` so a default layer's fold passes keep replaying in order.

Grouped path for content bodies only — pixel, fill, shape, type and smart-object content:

1. Render the body into a derived canvas with `skip_effects = true`, using a proxy override (`Normal`, opacity 255, Blend If off; fill, mask and text curve retained). This reuses `composite_layer_inner` instead of extracting the body match.
2. Apply the fold passes with the inherited effect scale (not the layer opacity).
3. `composite_canvas` the result onto the parent with the real mode/opacity/Blend If and a proxy override (`fill 255`, `no_mask`, `no_curve`).
4. Apply the above passes to the parent at `effect_scale * opacity`, so Inner Shadow/Stroke/Bevel stay on the blended result.

The guard excludes `skip_effects` canvases, groups and destructive adjustments (`is_destructive_adjustment`), so the recursive call cannot re-enter the grouped path and adjustment layers keep today's composition. The no-fold case is algebraically the same as direct compositing and is pinned by a byte-identity test. The inner body applies the text curve; the final blend uses `no_curve`, and a type-layer test pins that the curve is applied once.

Why recursive reuse: smaller and safer than extracting the ~140-line body match out of a file already at 1093 lines. Alternate considered: a separate grouped body dispatcher; rejected as duplication.

### D8. Integration

- GPU: extend `own_attribute_decline` with `!blend_clipping`, `blend_interior`, `!transparency_shapes`, both hide flags and non-empty `restricted_channels`; update the advanced-blending error text. The gate applies to a visible layer at any depth, not a hidden layer.
- Render session: hash the new fields and restricted list; copy `blend_clipping`, `blend_interior`, `transparency_shapes`, both hide flags and `restricted_channels` in `sync_targets`, with a reduced-preview test.
- Authoring: the existing `blending.blendInterior` key changes meaning with the default flip; new keys stay with #695/#581.
- Layer-style copy/paste: `LayerStyle`, `copy_layer_style` and `paste_layer_style` carry the new fields.
- Damage: `paint_bounds` returns the canvas when `tsly` is off and an effect or knockout is active; style-edit refresh and structural edits use it.

### D9. Module layout

New `composite_channels.rs` (wrapper + formula), declared in `crates/pictura-render/src/lib.rs`. The grouped `infx` path stays in `composite.rs` (which remains under the 1200-line cap), and `composite.rs` also gains the fold/above split. The effect source-rect helper stays in `layer_effects/mod.rs`; the canvas matte branch and the output-mask helper live in `layer_effects/planes.rs`.

## Risks / Trade-offs

- **No local Photoshop oracle** → the specs say "documented/probed", tests pin our defined behaviour, and the PR does not claim verified parity.
- **`tsly` whole-canvas is calibrated against external PS 2026 probes, not CS6** → mark the limitation in the design and tests; keep the flag off by default.
- **`clbl` off may diverge from Photoshop's observed inheritance** → follow the repo docs; note the external-probe uncertainty in the parity register/PR.
- **`infx` grouping can regress effect blending** → default off, content bodies only, and the no-fold identity test.
- **The fold split is a cross-cutting `LayerPasses` change** → update the GPU backend/plan/metrics, region fast path and `size` in the same task, and add a fold-only region test.
- **`tsly` off widens damage/paint bounds to the canvas** → `paint_bounds` returns the canvas when the flag is off and an effect or knockout is active; add a `layer_paint_bounds` test.
- **Mask-hide on a fill or shape layer still narrows the source box** → make `rect_for_effect_source`/`fill_content_box` skip a hide-flagged mask, not just `apply_mask`.
- **The grouped text curve can be applied twice** → `no_curve` on the final blend and a type-layer on/off test.
- **The long-form docs describe the old fold/`infx`/channel model** → reconcile them in a `TASK-ALLOWS-DOCS` commit.
- **Channel wrapper clones the canvas per restricted layer** → only when the list is non-empty; `ponytail:` note names the upgrade path.
- **Old files with unlinked masks and no `lmgm`/`vmgm`** → no link-status fallback; default false may differ from Photoshop's legacy rule. Recorded as a ceiling.
- **CMYK/Lab channel exclusion is modelled but inert** → preserved for round-trip, checkbox/render work stays with the colour-mode issues.
- **Groups** → `infx` is inert because group effects are not rendered; flags still round-trip.

## Migration Plan

- The codec change is Photoshop-compatible in both directions: absent/`0` reads as off, `1` as on, and the writer emits only non-defaults. Files written by the old engine that omitted `infx` now read as off, which is what Photoshop already did. No rewrite is required.
- Land in four commits: (1) core model + codec + GPU decline + digests; (2) `tsly` and mask roles; (3) channel Include; (4) `clbl` + `infx`. Each commit keeps default documents byte-identical.
- Rollback is revert-by-commit; the flags are default-off, so a revert does not strand documents.
- No data migration, no dependency change.
