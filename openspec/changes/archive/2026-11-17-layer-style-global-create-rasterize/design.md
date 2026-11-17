# Design

## Context

Layer effects already decode from `lfx2`/`lrFX` into typed parameters and render
as reusable document-space `EffectPass` entries (`rect`, blend mode, colour,
alpha), replayed by the CPU compositor with `alpha * layer opacity`
(`crates/pictura-render/src/layer_effects/`). Style authoring edits the
descriptor through `layer_style_value` / `set_layer_style_value`, and the
commands for copy/paste/clear/scale/visibility sit in the same module. Three
pieces are missing:

- The document global light (PSD image resources 1037 angle and 1049 altitude)
  is not decoded; `use_global_angle` is carried by the decoders but unused.
- `Rasterize Layer Style` and `Create Layers` have no engine implementation, and
  the Layer-menu / panel entries are disabled.
- `Rasterize Layer Style` has no C++ command id constant and no handler; its
  catalogue row and row-menu entry exist but are disabled.

The document model already carries typed, resource-backed state through a
read-resolve / write-sync pair (`Document.guides` ↔ resource 1032), which is the
pattern this change reuses. The effect pass cache is keyed by layer content plus
the `lfx2` bytes; it does not currently key on the document global light.

## Goals / Non-Goals

**Goals:**

- One document-wide light that every `use_global_angle` effect reads, stored in
  the PSD resources and preserved byte-for-byte when untouched.
- `Rasterize Layer Style` and `Create Layers` implemented once in
  `pictura-render` and surfaced through the existing registry commands.
- The produced layer stacks recompose to the live appearance within rendering
  tolerance.
- All three commands agree in enabled/label state across the Layer menu, the
  panel's layer-style submenu and the row menu.

**Non-Goals:**

- Reproducing CS6's clipping-group shape in `Create Layers`; the pass pixels are
  already confined, and clipping would run through the compositor's isolated
  group path and change appearance on semi-transparent content.
- Converting type, shape, fill-content or smart-object layers to pixels as part
  of `Rasterize Layer Style` or `Create Layers`.
- A separate GPU shader or eligibility rule for global light: it is resolved in
  the shared pass build, so the GPU path inherits it, and the existing GPU effect
  coverage only needs a globally-lit acceptance assertion.
- `.asl` preset support.
- A `Preview` checkbox in the Global Light dialog, or preset libraries.

## Decisions

### D1. Global light is typed document state, resolved on open and emitted on save

Add `pictura_core::GlobalLight { angle: i32, altitude: i32 }` (default 30/30)
in a small `global_light` module and one field on `Document`. `pictura-codec`
resolves it from resources 1037/1049 immediately after `resolve_guides` on both
the full and lazy read paths, and emits it by chaining
`with_document_global_light` onto the existing `guide_resources::output_resources`
call in `write.rs`.

Alternatives considered:

- **Parse the resources per composite.** Rejected: `build_layer_passes` runs per
  layer per composite, and the image-resource section includes the ICC profile,
  so a naive decode copies megabytes per layer. A zero-copy targeted scan would
  work but duplicates the codec's framing logic in the render crate.
- **Store only raw resource bytes and parse in the dialog.** Rejected for the
  same cache/perf reason; a typed field also rides the history snapshot, so
  undo is free.

Resource format is a four-byte big-endian signed integer, confirmed against
psd-tools (`GLOBAL_ANGLE` 1037, `GLOBAL_ALTITUDE` 1049). Missing or short
resources fall back to 30 per value. The writer returns the section untouched
when both values match the stored pair (an absent resource counting as its
default); otherwise it writes both resources, so a partial edit cannot leave the
pair mismatched. An unedited file keeps its bytes.

### D2. Resolve the effective light when passes are built, not when effects decode

`decoded_effects` is cached by the `lfx2` bytes, so baking the global light into
the decoded struct would go stale when only the document light changes. Instead
`layer_effects::light::effective(effects, doc)` clones the decoded set and
overrides `drop_shadow.angle_deg`, `inner_shadow.angle_deg`, and
`bevel.angle_deg` / `bevel.altitude_deg` for effects whose `use_global_angle` is
true. `build_layer_passes` calls it once before computing the mask clip and the
passes. Satin keeps its stored angle: its decoder deliberately ignores `uglg`.

The pass cache gains `doc.global_light` in `pass_key_hash`; without it a light
edit would replay previously cached passes. `effects_reach` (outer paint
bounds) is already angle-independent in magnitude and needs no document. The
doc-free `paint_reach` used for dab/history padding is made conservative for
global-angle effects (worst case `ceil(distance·√2)`), because a local 0° angle
would otherwise under-pad a global 45° offset.

The effect pages read the document light through the bridge; the page control is
disabled and shows the global value without writing `lagl`, and unchecking
restores the stored local value.

### D3. Rasterize Layer Style reuses the shape rasterizer's scratch recipe

`rasterize_layer_style` builds a one-layer scratch document rendered with
`composite_rgba`, grows the layer rect to the content and effect bounds, bakes
the RGBA into the layer, removes `lfx2`/`lrFX`, and records one structural
state. The scratch carries the document state the passes resolve against:
`global_light`, `layer_section_extra` (patterns) and `is_psb`, copied exactly as
`rasterize_shape` does. The scratch copy has `opacity = 255`, `blend = Normal`,
`mask = None`, `vector_mask = None`, `blend_if = None` with its raw
`blending_ranges` cleared, and `knockout = None`, while keeping `fill`; the
result then sets `fill = 255` and keeps its opacity, blend, raster mask, vector
mask, Blend If and knockout. Baking the content at the live fill and normalising
the result to 255 preserves a Fill 50 % layer's look: content is fill-scaled
once and effect passes (which are never fill-scaled) are not scaled at all,
whereas keeping the fill would apply it twice to content and wrongly to the
baked effects. Neutralising the vector mask in the scratch avoids applying it
twice, because effect mattes already include it; the live vector mask still
applies to the baked pixels afterwards. The approximation (effect passes
resolved against transparent before the layer blend applies) is the same one the
shape path already ships.

The command refuses rather than converts when the base is not a raster pixel
layer (type, shape, fill, smart object), is a group or the Background, when no
effect is enabled and present, when the effect master switch is off, when the
layer has a customized Blend If or a non-None knockout, or when the base owns a
clipped run above it (baking effects into a clip base would widen the clip
coverage). Photoshop's exact behaviour for those cases is undocumented; refusing
avoids silent data loss and keeps the first slice exact.

### D4. Create Layers materialises the existing effect passes, without clipping

For each enabled family, the same `build_*` calls that the compositor uses are
run individually, with `light::effective` applied first so a globally-lit effect
explodes at the document light, and each `EffectPass` is materialised into a
layer-sized RGBA buffer with `Canvas::new_region` + `planes::apply`. The
materialiser is a `pub(crate)` helper in `layer_effects` because `planes` and the
`build_*` functions are private there; `document_ops` only consumes the returned
buffers. Produced layers take the pass blend mode, the base layer's opacity (the
pass alpha is stored unscaled and multiplied by layer opacity at replay), opaque
fill, and no mask (the matte already includes it). Bevel & Emboss emits one pass
per slot pair: two for Inner/Outer/Emboss/Stroke and four for Pillow, so those
styles produce two and four layers. Below-content passes are inserted beneath
the base and above-content passes above it at increasing offsets (equivalently,
in reverse draw order), so the final bottom-to-top stack follows the
compositor's draw order. The base keeps everything but the style.

Clipping is deliberately not used. The compositor renders a clipping group in
isolation (`composite_clipping.rs`), so clipped interior-effect layers would
blend against the base alone rather than base-over-backdrop, changing pixels on
semi-transparent content. Plain layers above/below reproduce the live composite
exactly; the docs' "some effects become clipped layers" is treated as CS6's
structural shape, not a visual requirement, and the docs task corrects it.
Create Layers refuses a base that is itself clipped or that owns a clipped run
above it: the former would break the clip chain, and a new non-clipped layer
above a clip base would become the run's new base. Rasterize refuses a base that
owns a clipped run above it because baking effects into a clip base widens the
clip coverage.

### D5. One registry command per action, dispatched from every surface

`LayerRasterizeLayerStyle` joins `const_names.rs` (regenerated into
`command_ids.h`), and all three handlers are registered in
`wireLayerStyleMenu`. The row action table gains a `RasterizeStyle` dynamic case
with a fact for "styled raster base with a renderable effect"; the panel's
layer-style submenu replaces its hand-built disabled stubs with
`registry_->detachedAction` + `dispatch`, so it takes the same label and enabled
state as the menu. Task 4.4 adds `rasterizeStyle` to the row-menu text-override
list so the row keeps the label "Rasterize Layer Style" instead of the registry's
"Layer Style", and the panel label changes from "Create Layer" to the registry's
"Create Layers" so the surfaces agree.

### D6. Undo and damage

Global Light is a document-level change that can move every styled layer's
pixels, so the app records it with `Edit::Full`. Create Layers and Rasterize
Layer Style are structural edits; Create Layers uses the existing structural
edit with no damage bound (it inserts and modifies sibling layers), and
Rasterize uses the before/after influence pattern already used by the other
rasterize commands.

## Risks / Trade-offs

- **Stale passes after a light edit** → `pass_key_hash` includes the document
  light, and a render test composites before and after the edit.
- **Under-padded dab refresh** → `paint_reach` uses the √2 worst case for
  globally-lit shadows.
- **Byte churn on save** → the writer leaves the section untouched when the
  stored values match, including absent resources at their defaults.
- **Create Layers differs structurally from CS6** (no clipping group, bevel as
  one layer per pass (two, or four for Pillow), layer names derived from effect
  names) → accepted; appearance equality is the acceptance criterion, and the
  file header records the ceiling.
- **Quantisation in materialised passes** (f32 alpha/colour to RGBA8) → tests
  compare recomposites within a small tolerance rather than bytewise.
- **Existing tests assert the disabled stubs** → update
  `tst_layer_style::menuCommands`, `tst_layers_effects::rowMenuListsLayerStyleCommands`
  and `tst_layers_row_model::unimplementedRowsAreDisabled` in the same change
  rather than deleting them.
- **File-size caps** → new logic goes into new modules (`global_light`, `light`,
  `materialize`, dialog) so `pictura-core/src/lib.rs` (1190 lines) and
  `layer_effects/mod.rs` (1138) stay under their 1200-line caps.
- **Global light changes unchanged-file rendering** when a file sets `uglg`
  true but omits the resources (now defaults to 30/30, Photoshop's default).
  Existing test helpers that build `uglg` true (`ShadowSpec` in
  `crates/pictura-render/src/tests/layer_effects.rs:245`, and authored styles
  whose `useGlobalLight` defaults on in `layer_style_fields.rs:56`) must be
  audited: set the flag false (or the test document's light) where a local-angle
  assertion is intended, and add explicit global-light tests for the override.

## Migration Plan

No stored-state migration. Files without the resources keep their bytes until
the light is edited; files with them keep them when unchanged. Undo restores the
previous light through the normal history snapshot. No dependency additions.
