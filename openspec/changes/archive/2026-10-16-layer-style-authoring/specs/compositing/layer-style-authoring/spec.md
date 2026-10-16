## ADDED Requirements

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
unchanged value, a non-finite value, a group, the Background, or (from the layer-only setter)
a Pattern Overlay that does not yet exist. An `lfx2` left with no effect object SHALL be
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
