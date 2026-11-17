# Spec Delta

## ADDED Requirements

### Requirement: Global Light command

`pictura-render` SHALL expose a document-level global light setter,
`set_document_global_light(doc, angle, altitude)`, which stores the angle
normalised into `0..=359` and the altitude clamped to `0..=90`. It SHALL return
false, changing nothing, when neither value differs from the stored light. The
app SHALL record one "Global Light" history state when the setter reports a
change and SHALL recomposite the whole canvas.

#### Scenario: The light is normalised and recorded once [lsa_global_set]

- **WHEN** the global light is set to angle 370 and altitude 95
- **THEN** the document reads angle 10 and altitude 90, and exactly one "Global Light" state is recorded

#### Scenario: Setting the same light records nothing [lsa_global_noop]

- **WHEN** the global light is set to the values it already holds
- **THEN** the setter returns false and no history state is added

### Requirement: Create Layers command

`pictura-render` SHALL expose `can_create_style_layers(doc, path)` and
`create_style_layers(doc, path) -> Vec<String>`. Create Layers SHALL materialise
one ordinary pixel layer per enabled and present effect pass: the below-content
passes (Drop Shadow, then Outer Glow) SHALL be inserted beneath the base layer,
and the above-content passes SHALL be inserted above it at increasing offsets
(equivalently, in reverse draw order) so the final bottom-to-top stack follows
the compositor's draw order (Pattern Overlay, Gradient Overlay, Color Overlay,
Satin, Inner Glow, Inner Shadow, Stroke, Bevel & Emboss). Each produced layer
SHALL carry the effect's blend mode, the base layer's opacity, opaque fill, and
the pass pixels; it SHALL NOT copy the base layer's mask, because the pass matte
already includes it. The passes SHALL resolve the document's global light
exactly as the live canvas does. Bevel & Emboss SHALL produce one layer per pass
(its shadow and highlight modes are separate): two for the Inner, Outer, Emboss
and Stroke styles, and four for Pillow. Layer names SHALL be unique and derived from
the effect names. The base layer SHALL keep its pixels, kind, mask and Blending
Options and SHALL lose its layer style, and the produced stack SHALL composite
to the same appearance as the live style within rendering tolerance.

The command SHALL refuse, changing nothing, on an unstyled layer, a group or the
Background, a layer with no raster pixel channels, a layer with no enabled and
present effect, a layer whose effects are hidden by the master switch, a clipped
base layer, and a base that owns clipped layers above it. A successful run SHALL
record one "Create Layers" history state.

#### Scenario: The created layers recompose to the live appearance [lsa_create_layers]

- **WHEN** a styled pixel layer with a Drop Shadow, a Bevel & Emboss and overlapping above-content effects is exploded with Create Layers and the document is recomposited
- **THEN** the composite matches the pre-command composite within tolerance, the base layer has no style, and its new sibling layers carry the effects' pixels and modes

#### Scenario: The produced layers inherit the base state [lsa_create_inherit]

- **WHEN** a styled base at opacity 128 with a raster mask is exploded
- **THEN** each produced layer has opacity 128, opaque fill, no mask and a unique name, and the base keeps its mask and fill

#### Scenario: Above-content layers keep the draw order [lsa_create_order]

- **WHEN** a layer styled with Pattern Overlay, Color Overlay, Inner Glow and Stroke is exploded
- **THEN** the four new layers above the base read Pattern Overlay, Color Overlay, Inner Glow, Stroke from bottom to top

#### Scenario: A globally-lit effect explodes with the document light [lsa_create_global]

- **WHEN** a Drop Shadow with `use_global_angle` true is exploded on a document whose global angle is 90
- **THEN** the recomposite matches the live canvas rendered at 90, not the effect's stored local angle

#### Scenario: Bevel becomes one layer per pass [lsa_create_bevel]

- **WHEN** a layer whose only effect is an enabled Bevel & Emboss is exploded
- **THEN** an Inner bevel produces two new pixel layers above the base, one for the shadow pass and one for the highlight pass, and a Pillow bevel produces four

#### Scenario: Create Layers refuses what it cannot convert [lsa_create_refuse]

- **WHEN** Create Layers is asked for an unstyled layer, a group, the Background, a layer with only disabled effects, a clipped base layer, or a base that owns a clipped run
- **THEN** it creates no layers, removes no style, and records no state

### Requirement: Rasterize Layer Style command

`pictura-render` SHALL expose `can_rasterize_layer_style(doc, path)` and
`rasterize_layer_style(doc, path)`. Rasterize Layer Style SHALL render the
layer's content and its enabled and present effects through the CPU compositor,
bake the result into the layer's pixel channels over the content's and effects'
combined bounds, and remove the `lfx2` and `lrFX` style blocks. It SHALL keep the
layer's name, position, blend mode, opacity, layer mask, vector mask, Blend If
and knockout, and SHALL set the layer's fill to 255 after baking the content at
the layer's prior fill. It SHALL leave the layer's adjustment, shape, type, or
smart-object data untouched by refusing such a layer instead of converting it.
It SHALL refuse, changing nothing, on an unstyled layer, a group or the
Background, a layer with no raster pixel channels, a layer with no enabled and
present effect, a layer whose effects are hidden by the master switch, a layer
with a customized Blend If (typed non-default or a raw `blending_ranges` block),
a layer with a non-None knockout, and a base that owns clipped layers above it.
A successful run SHALL record one "Rasterize Layer Style" history state.

#### Scenario: A styled layer is baked and cleared [lsa_rasterize]

- **WHEN** a pixel layer with an enabled Drop Shadow is rasterized with Rasterize Layer Style
- **THEN** the layer's pixels include the shadow as it showed on the canvas, `has_layer_style` is false, and the composite matches the pre-command composite within tolerance

#### Scenario: Fill is baked and normalised [lsa_rasterize_fill]

- **WHEN** a styled layer at Fill 50 % is rasterized
- **THEN** the composite matches the pre-command composite and the layer's fill reads 255

#### Scenario: A globally-lit effect bakes at the document light [lsa_rasterize_global]

- **WHEN** a layer with a Drop Shadow at `use_global_angle` true is rasterized on a document whose global angle is 90
- **THEN** the baked pixels carry the shadow at 90, matching the pre-command composite

#### Scenario: A pattern overlay bakes the document pattern [lsa_rasterize_pattern]

- **WHEN** a layer with an enabled Pattern Overlay is rasterized
- **THEN** the baked pixels show the document's pattern rather than the placeholder

#### Scenario: Rasterize refuses what it cannot bake [lsa_rasterize_refuse]

- **WHEN** Rasterize Layer Style is asked for an unstyled layer, a fill layer, a type layer, a group, the Background, a layer with only disabled effects, a layer with a customized Blend If or knockout, or a base that owns a clipped run
- **THEN** it changes nothing and records no state

#### Scenario: Undo restores the live style [lsa_rasterize_undo]

- **WHEN** a baked layer is undone
- **THEN** the style block returns and the pre-raster pixels are restored in one step
