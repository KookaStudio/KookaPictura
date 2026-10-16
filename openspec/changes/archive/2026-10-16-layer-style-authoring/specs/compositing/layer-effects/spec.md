## MODIFIED Requirements

### Requirement: A bevel and emboss composites inside the layer content

An enabled `BevelEmboss` SHALL be composited as a lit bevel whose side of the
content edge follows its style. On a visible non-group, non-destructive-adjustment layer with
`present` true the CPU compositor SHALL composite it **after** the layer's own
content. It SHALL build a coverage matte `M` from the layer's content alpha — a
pixel layer's alpha channel, a solid fill's payload alpha, an opaque gradient or
embedded smart-object coverage, or a pattern fill's resolved pattern alpha —
multiplied by the layer mask. It SHALL form a height field in floating point from
the matte's signed Euclidean distance to its edge (positive inside, sub-pixel on
anti-aliased edge pixels), as a chamfer rising from 0 to 1 over `size` pixels:
inside the edge for `Inner`, outside it for `Outer`, and straddling it (half
each side) for `Emboss` and `Pillow`. The technique SHALL round the chamfer
with a Gaussian blur: `Smooth` by `max(size / 3, 1.5)` pixels, `ChiselSoft` by
1 pixel, `ChiselHard` not at all. It SHALL derive the surface normal from the
height gradient scaled by `size` and `depth`, build the light vector from
`angle_deg`/`altitude_deg` with the same sign convention as the shipped shadows,
and take the Lambertian shading as the signed deviation of the normal from flat.
The signed shading SHALL be negated when `direction` is `Down`; its positive
and negative parts SHALL each be blurred by `soften` pixels separately, so they
spread rather than cancel. The positive part tinted by the highlight colour,
opacity and blend mode and the negative part tinted by the shadow colour,
opacity and blend mode SHALL each be multiplied by the style's coverage and composited above the
content, with the shadow drawn before the highlight: `Inner` by `M` (every pixel
outside `M` byte-identical to the same document without the effect), `Outer` by
`1 - M` (every pixel inside the content byte-identical), `Emboss` by full
coverage, and `Pillow` by `M` plus `1 - M` with the shading negated outside.
The `Stroke` style (which needs the Stroke effect's band) SHALL be a no-op. The
`ChiselHard` and `ChiselSoft` techniques SHALL render as `Smooth` (a stated
approximation). `depth` 0 or both effect opacities 0 SHALL be a no-op. A zero-area or fully off-canvas rect or an empty matte SHALL be a
no-op. A layer with no effect, or with a disabled or not-present effect, SHALL
composite byte-identically to the same document without the effect. The bevel
SHALL be clipped to the canvas, SHALL NOT panic, and SHALL NOT add a dependency.

#### Scenario: The bevel is interior and leaves exterior pixels unchanged

- **WHEN** an opaque square layer over an opaque backdrop has an enabled and present Inner/Smooth bevel with a non-zero `size`, `depth` and colour, and the composite is compared with the same document without the effect
- **THEN** every pixel outside the square's coverage is byte-identical, and at least one interior pixel differs from the un-beveled content

#### Scenario: Angle and altitude shape the highlight and shadow

- **WHEN** the same bevel is rendered with two different `angle_deg` values, and separately with two different `altitude_deg` values
- **THEN** the composites differ, showing that the lit and darkened interior edges follow `angle_deg` and `altitude_deg`

#### Scenario: Size, soften and depth shape the shading

- **WHEN** the same bevel is rendered with `size` 0 and with a larger `size`, with `soften` 0 and with a larger `soften`, and with `depth` 0 and with a larger `depth`
- **THEN** the larger `size` spans at least as many shaded interior pixels as the smaller, the larger `soften` spans at least as many transition pixels, the larger `depth` increases the shading magnitude, and `depth` 0 is byte-identical to no effect

#### Scenario: Direction flips the bevel

- **WHEN** the same bevel is rendered once with `direction` `Up` and once with `Down`
- **THEN** the two composites differ, with the highlight and shadow exchanging places

#### Scenario: Highlight and shadow colour, opacity and mode shape the bevel

- **WHEN** a bevel with a non-white highlight and a non-black shadow is composited over an opaque content layer, and once more with different highlight/shadow opacities and non-`Normal` blend modes
- **THEN** lowering an opacity approaches the un-beveled content, and the different colour and blend-mode composites differ

#### Scenario: The layer mask shapes the matte

- **WHEN** a beveled layer's mask hides half the canvas
- **THEN** the bevel is absent where the mask is zero and present where the mask is 255

#### Scenario: The bevel is bounded to the content rect plus reach

- **WHEN** a small content layer with a large `size` and `soften` is composited on a much larger canvas
- **THEN** every pixel outside the content rect padded by the blur reach is byte-identical to the same document without the effect

#### Scenario: Each style lights its side of the edge

- **WHEN** a red square over white is beveled with each of `Inner`, `Outer`, `Emboss` and `Pillow`
- **THEN** `Inner` changes only pixels inside the square, `Outer` only pixels outside it, `Emboss` and `Pillow` both, and `Pillow`'s outer ring differs from `Emboss`'s

#### Scenario: A deferred style or technique is a no-op

- **WHEN** an enabled and present bevel has the `Stroke` style, the only style still deferred
- **THEN** the composite is byte-identical to the same document without the effect

#### Scenario: Each technique rounds the chamfer differently

- **WHEN** the same bevel is rendered with `Smooth`, `ChiselHard` and `ChiselSoft`
- **THEN** the three composites differ from each other

#### Scenario: A disabled or absent bevel is a no-op

- **WHEN** a layer carries no `lfx2` block, or an `ebbl` with `enab` false, or an `ebbl` with `depth` 0
- **THEN** the composite is byte-identical to the same document without the effect

## ADDED Requirements

### Requirement: Interior effects stack in CS6's order

The effects composited after the layer's content SHALL be drawn in CS6's
stacking order, the Layer Style list read bottom-up: Pattern Overlay, Gradient
Overlay, Color Overlay, Satin, Inner Glow, Inner Shadow, Stroke, then Bevel &
Emboss on top, so an opaque overlay does not hide the effects listed above it.

#### Scenario: Satin and bevel show over an opaque overlay

- **WHEN** a square has a 100 % blue Color Overlay, and Satin and Bevel & Emboss are switched on
- **THEN** the composite differs from the overlay alone, and switching Satin and Bevel off again restores it exactly

### Requirement: Shadow and glow noise

The Drop Shadow, Inner Shadow, Outer Glow and Inner Glow SHALL decode `Nose`
(percent, `0..=100`, default 0) and multiply the effect's alpha at canvas
`(x, y)` by the grain factor `clamp(1 + 2·n·(2r − 1), 0, 1)`, with `n` the
noise fraction and `r` the compositor's fixed splitmix hash of `(x, y)` in
`[0, 1)`; noise 0 SHALL leave the effect unchanged. The grain is a stated
approximation; Adobe's noise model is unpublished. Legacy `lrFX` effects carry
no noise.

#### Scenario: Noise breaks a drop shadow into a fixed grain

- **WHEN** a hard drop shadow is rendered at noise 0 and at noise 82, twice
- **THEN** the noisy shadow differs from the clean one with some shadow pixels lightened, and both noisy renders are identical
