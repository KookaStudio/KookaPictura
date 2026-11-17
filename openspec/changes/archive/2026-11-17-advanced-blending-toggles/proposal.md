# Proposal: advanced-blending-toggles

## Why

The docs↔code parity register records Advanced Blending as missing (`LAY-1`): `clbl` (Blend Clipped Layers as Group) is parsed but never read at render, `infx` (Blend Interior Effects as Group) is stored but never composited, and `tsly` (Transparency Shapes Layer), `lmgm`/`vmgm` (Layer/Vector Mask Hides Effects) and `brst` (channel Include) have no first-class fields or compositor behaviour at all. The result is that a PSD authored in Photoshop with any of these toggles opens and re-saves losslessly but renders differently.

While grounding the fix, the existing `infx` model turned out to be inverted: the engine defaults it to `true` and writes `infx=0` when false, while Photoshop defaults it off and writes `1` when the option is on. Implementing "Blend Interior Effects as Group" on top of the current field would change every document's appearance, so the polarity correction is part of this change.

The Layer Style dialog controls for the new toggles are tracked separately (#695, #581); this change is the engine contract, and those issues add the dialog and its authoring keys.

## What Changes

- **BREAKING (model/codec contract):** correct `infx` — absent/`0` means off, `1` means "Blend Interior Effects as Group"; the `Layer::blend_interior` default becomes `false` and the writer emits `infx` only when true. Photoshop-readable output is unchanged for files this engine previously wrote.
- Add first-class `Layer` fields for `transparency_shapes` (`tsly`, default on), `layer_mask_hides_effects` (`lmgm`, default off), `vector_mask_hides_effects` (`vmgm`, default off) and `restricted_channels` (`brst`, default empty). Decode and write each tag with the Photoshop default-omission rules.
- CPU compositor:
  - **Blend Clipped Layers as Group off** — the base uses its own mode/opacity; each clipped member uses its own mode/opacity while remaining bounded by the base's content coverage.
  - **Blend Interior Effects as Group on** — Pattern/Gradient/Color Overlay, Satin and Inner Glow fold into the layer content, and the layer's mode/opacity/mask/Blend If apply once to that group; Inner Shadow, Stroke and Bevel stay on the blended result. Off keeps today's per-effect modes byte-identically.
  - **Transparency Shapes Layer off** — the effect shape becomes the whole canvas: the overlays fill the canvas, a knockout cuts the whole canvas, and the edge-derived effects (drop shadow, outer glow, stroke, inner shadow) disappear, matching Photoshop's probed behaviour.
  - **Layer/Vector Mask Hides Effects on** — the corresponding mask stops shaping effect sources and instead clips effect output; each mask switches independently.
  - **Channel Include** — an excluded channel keeps the backdrop's premultiplied value (`dst · dst_alpha / out_alpha`) after the layer's whole contribution; alpha still composites; excluding every colour channel removes the layer, effects included.
- GPU compositor declines any stack carrying a non-default value of the new flags so the CPU result is authoritative.
- Render-session digests, effect-pass cache keys and reduced-document sync carry the new fields.
- Reconcile the long-form advanced-blending docs with the implemented fold set, corrected `infx` default and channel-restriction model in a docs commit carrying `TASK-ALLOWS-DOCS`.
- Tests cover codec round-trips, every toggle's default no-op, each non-default behaviour, and the GPU declines.

## Capabilities

### New Capabilities

- `compositing/advanced-blending-toggles`: the five compositor behaviours above, their defaults, their canvas/cache/gpu consequences, and the guarantee that default documents composite byte-identically.

### Modified Capabilities

- `codec/psd-advanced-blending`: the tagged-block defaults and polarity (`infx` corrected; `tsly`/`lmgm`/`vmgm`/`brst` added), write gating, and a new channel-restrictions round-trip requirement.
- `compositing/clipping-masks`: the "group takes the base's opacity and blend mode" requirement becomes conditional on Blend Clipped Layers as Group.
- `compositing/knockout-compositing`: the "a transparent pixel does not punch through" requirement becomes conditional on Transparency Shapes Layer.
- `compositing/layer-effects`: the effect-source mask multiplication is the flag-off behaviour; with Layer/Vector Mask Hides Effects on, the mask clips pass output instead.

## Impact

- `crates/pictura-core`: `Layer` fields and defaults.
- `crates/pictura-codec`: `read.rs`, `advanced_blending.rs`, codec tests.
- `crates/pictura-render`: `BlendOverride.no_curve`, `composite_knockout.rs`, `composite_clipping.rs`, new `composite_channels.rs`, `layer_effects/*`, `layer_source/*`, `render_session.rs`, `gpu/plan.rs`, `document_ops/layer_ops/layer_style.rs`.
- `ROADMAP.md` and `docs/dev/doc-code-parity-gaps.md` (LAY-1) record the engine shipment and the remaining dialog work (#695); `docs/05-layers/blend-modes.md` and `layer-styles.md` are reconciled in a `TASK-ALLOWS-DOCS` commit.
- No new dependencies. No Qt/C++ or CMake changes in this change.
