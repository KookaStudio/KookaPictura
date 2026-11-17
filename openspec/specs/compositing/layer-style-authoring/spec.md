# compositing/layer-style-authoring Specification

## Purpose
Reads and writes layer-style values through the lfx2 descriptor so styles
round-trip with a PSD.

## Requirements

### Requirement: Layer-style values read and write the lfx2 descriptor

`pictura-render` SHALL expose `layer_style_value(layer, key)` and
`set_layer_style_value(layer, key, value)` over the layer's `lfx2` descriptor,
where `key` is `"<effect>.<field>"` for the ten effects (`bevel`, `stroke`,
`innerShadow`, `innerGlow`, `satin`, `colorOverlay`, `gradientOverlay`,
`patternOverlay`, `outerGlow`, `dropShadow`). Values SHALL be the dialog's
units: percent, pixels and degrees as shown, a colour packed `0xRRGGBB`, a blend
mode as its index in `BlendMode::LAYER_MODES`, a choice as its option index, and
a flag as `0` or `1`; numeric values SHALL be clamped to the field's range. A
field of an absent effect, or an absent key, SHALL read as CS6's default.
Writing a field of an absent effect SHALL add the effect, off (`enab` and
`present` false), with every table field at its default. `<effect>.on` SHALL
read as `enab` and `present` both true and SHALL write both; the read-only
`<effect>.exists` SHALL report whether the effect object is present. Every
authored effect SHALL decode through the shipped `lfx2` decoder with the values
written. A write SHALL return false, changing nothing, for an unknown key, an
unchanged value, a non-finite value, the Background, or (from the layer-only setter)
a Pattern Overlay that does not yet exist. A group accepts the `blending.*` keys;
its effect fields remain refused until the compositor renders group effects. An `lfx2` left with no effect object SHALL be
removed.

#### Scenario: An absent effect reads its defaults

- **WHEN** a plain pixel layer is asked for `dropShadow.on`, `dropShadow.opacity`, `dropShadow.mode` and `outerGlow.color`
- **THEN** they read `0`, `75`, the Multiply index `3`, and `0xFFFFBE`, and an unknown field reads `None`

#### Scenario: An authored drop shadow decodes and renders

- **WHEN** a red square's `dropShadow.on` is set to 1, opacity 100, angle 90, distance 3, size 0 and colour blue
- **THEN** `decode_drop_shadow` returns those values with Multiply, and the composite shows a blue band below the square and white above it

#### Scenario: Every effect round-trips

- **WHEN** every effect but Pattern Overlay is switched on and one field of each is changed
- **THEN** each shipped decoder reads the changed value, and every one of the 27 blend modes written to an effect decodes as itself

### Requirement: Blending options write the layer record

`blending.mode`, `blending.opacity`, `blending.fillOpacity` (percent),
`blending.knockout` (0 None, 1 Shallow, 2 Deep), `blending.blendInterior` and
`blending.blendClipped` SHALL read and write the layer's blend mode, opacity,
fill, `knko` knockout, `infx` and `clbl` fields.

#### Scenario: Blending options change the layer

- **WHEN** `blending.mode` 3, `blending.opacity` 50, `blending.knockout` 2 and `blending.blendInterior` 0 are set
- **THEN** the layer is Multiply at opacity 128 with a Deep knockout and Blend Interior Effects off, and an out-of-range mode index is refused

### Requirement: Layer Style commands

`pictura-render` SHALL provide: `copy_layer_style`, which captures the layer's
`lfx2` and `lrFX` blocks together with its Blending Options; `paste_layer_style`,
which replaces the style and Blending Options of every styleable target;
`clear_layer_style`, which removes `lfx2` and `lrFX`; `scale_layer_effects`,
which multiplies every pixel-sized field (not Spread or Choke) and the overlay
scales of the enabled effects by a percentage, clamped to range, with the
stroke size kept whole; and `set_all_effects_visible`, which sets every
layer's `masterFXSwitch`. Each SHALL return how many layers changed and skip
groups and the Background.

#### Scenario: Copy and paste replace the target's style

- **WHEN** a layer with a stroke and Fill Opacity 0 is copied and pasted onto another layer
- **THEN** the target has the stroke and Fill Opacity 0, and pasting again changes nothing

#### Scenario: Scale Effects scales sizes but not percentages

- **WHEN** a drop shadow at distance 10, size 5, spread 20 and a 5 px stroke are scaled by 50 %
- **THEN** the distance is 5, the size 2.5, the spread still 20, the opacity still 75, and the stroke 3 px

#### Scenario: Clear removes the style

- **WHEN** Clear Layer Style runs on a layer with an overlay
- **THEN** the layer has no `lfx2` and a second clear changes nothing

### Requirement: The master switch hides every effect

`decode_layer_effects` SHALL return no effects for a layer whose `lfx2`
`masterFXSwitch` is false, so the compositor, the GPU path and the effect reach
all treat its effects as hidden, while the effects themselves stay stored.

#### Scenario: Hide and show all effects

- **WHEN** a blue colour overlay's layer is hidden with `set_all_effects_visible(false)` and then shown
- **THEN** the content shows unstyled while hidden with `colorOverlay.on` still 1, and blue again once shown

### Requirement: An authored style survives save and reopen

An authored style SHALL be written by the PSD encoder and read back unchanged,
compositing identically.

#### Scenario: Save and reopen

- **WHEN** a layer with an inner glow (size 9) and a gradient overlay (first stop `0x123456`, Color mode) is saved as PSD and reopened
- **THEN** the reopened layer reads the same values and its composite equals the original's

### Requirement: Pattern Overlay uses the built-in patterns

`set_document_layer_style_value(doc, path, key, value)` SHALL behave as the
layer setter and additionally handle what needs the document.
`patternOverlay.pattern` SHALL pick one of the eight built-in patterns (the
Pattern Stamp's set, `layer_style_pattern_names`) by index: it SHALL embed that
pattern in the document's `Patt` block under a stable id when not already
there, and point the layer's Pattern Overlay at it, adding the overlay (off)
when absent. Switching on an absent Pattern Overlay SHALL first give it the
first pattern. Reading `patternOverlay.pattern` SHALL give the built-in index,
or -1 for a pattern that is not built in. `pictura-codec` SHALL provide
`encode_rgb_pattern` and `add_document_pattern`, which write an RGB pattern
record the pattern decoder reads back and keep every other global block.

#### Scenario: A Pattern Overlay embeds its pattern and survives a save

- **WHEN** an absent Pattern Overlay is switched on, then its pattern set to 3, and the document saved as PSD and reopened
- **THEN** it starts on pattern 0 with one embedded pattern and a changed composite, holds two embedded patterns after the switch, refuses an unchanged or out-of-range index, and the reopened layer reads pattern 3 with an identical composite

#### Scenario: Pattern records round-trip

- **WHEN** two RGB patterns are added to a document with another global block, and one is added again
- **THEN** the decoder returns both with their pixels, the other block is unchanged, the repeat is refused, and a PSD save and reopen keeps them

### Requirement: Group Blending Options writes

`set_layer_style_value(layer, key, value)` SHALL accept the `blending.*` keys for
a group and update the layer record. It SHALL continue to refuse effect keys on a
group until the compositor renders group effects.

#### Scenario: A group's opacity is written

- **WHEN** `blending.opacity` is set to 50 on a group
- **THEN** the group's opacity is 128 and `dropShadow.on` remains refused

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
