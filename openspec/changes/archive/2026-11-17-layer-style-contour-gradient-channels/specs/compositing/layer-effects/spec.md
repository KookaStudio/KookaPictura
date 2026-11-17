# Spec Delta

## MODIFIED Requirements

### Requirement: The object-based effects descriptor decodes a bevel and emboss

`pictura-render` SHALL derive a typed
`BevelEmboss { enabled, present, style, technique, direction, depth, size,
soften, angle_deg, altitude_deg, use_global_angle, gloss_contour, edge_contour,
edge_contour_on, edge_range, edge_anti_alias, highlight: {mode, color, opacity},
shadow: {mode, color, opacity} }` from a layer's `lfx2`
(`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block. The block SHALL be read as a
`DescriptorBlock2` (a `u32` version and a `u32` data version, then a version-16
descriptor body) and the top-level object whose class id is **`ebbl`** SHALL be
decoded. The decoder SHALL read `enab` as `enabled`, `present` as `present`,
`bvlS` (typeID `BESl`) as `style`, `bvlT` (typeID `bvlT`) as `technique`, `bvlD`
(typeID `BESs`) as `direction`, `srgR` as `depth` in percent, `blur` as `size` in
pixels, `Sftn` as `soften` in pixels, `lagl` as `angle_deg` in degrees, `Lald` as
`altitude_deg` in degrees, `uglg` as `use_global_angle`, and the pairs
`hglM`/`hglC`/`hglO` and `sdwM`/`sdwC`/`sdwO` (typeIDs `BlnM` and `RGBC`) as the
highlight and shadow mode/colour/opacity. The `hglM`/`sdwM` blend values SHALL be
the capitalized `BlnM` descriptor vocabulary (`Nrml`/`Mltp`/`Scrn`/…), not the
layer blend key vocabulary. The style values SHALL be
`InrB`/`OtrB`/`Embs`/`PlEb`/`strokeEmboss`, the technique values
`SfBL`/`PrBL`/`Slmt`, and the direction values `In  `/`Out `; a missing or
unknown value SHALL decode to its Photoshop default (`Inner`, `Smooth`, `Up`).
Numeric values SHALL be accepted as either a unit float or a `doub`. A missing
`hglC`/`sdwC` SHALL decode to white/black and a missing `hglM`/`sdwM` to
`Screen`/`Multiply` (the Photoshop defaults); absent numeric keys SHALL take the
Photoshop UI defaults (depth 100, size 5, soften 0, angle 120, altitude 30,
highlight/shadow opacity 75, `uglg` true). A finite value outside its documented
range SHALL be clamped (`depth` `0..=1000`, `size` `0..=250`, `soften`
`0..=250`, `altitude_deg` `0..=90`, both opacities `0..=100`). The gloss contour
(`TrnS`) and edge contour (`MpgS`) SHALL decode to point lists: the `ShpC`
object's `Crv` list of `CrPt` objects, each point's `Hrzn` and `Vrtc` clamped to
`0..=255` and ordered by input. A missing or unusable contour SHALL decode to
the linear list `(0,0) (255,255)` and SHALL NOT reject the bevel. `useShape`
SHALL decode to `edge_contour_on` (default false), `Inpr` to `edge_range` in
percent (default 50, clamped `1..=100`), and `AntA` to `edge_anti_alias`
(default false); the anti-alias flag SHALL be carried but SHALL NOT change the
render. The texture keys (`useTexture`, `InvT`, `Algn`, `Scl `, `Ptrn`) and
`showInDialog` SHALL remain ignored; the effective angle and altitude SHALL be
the stored `lagl`/`Lald`. A missing `lfx2` block, a missing `ebbl`, an unknown
data version, a wrong class id, a wrong-typed or non-finite numeric value
(including a finite `f64` that overflows to infinity as an `f32`), a wrong enum
typeID, a non-`RGBC` colour, or a descriptor that fails to parse SHALL be a
no-op (`None`) and SHALL NOT panic. An `ebbl` whose `present` or `enab` is false
SHALL decode but SHALL render nothing.

#### Scenario: A bevel and emboss descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains an `ebbl` object with `enab` true, `present` true, `bvlS` `BESl`/`InrB`, `bvlT` `bvlT`/`SfBL`, `bvlD` `BESs`/`In  `, `srgR` 250, `blur` 7, `Sftn` 3, `lagl` 120, `Lald` 30, `uglg` false, `hglM` `BlnM`/`Scrn` with a non-white colour and `hglO` 80, and `sdwM` `BlnM`/`Mltp` with a non-black colour and `sdwO` 70
- **THEN** `decode_bevel_emboss` returns a `BevelEmboss` whose `enabled` and `present` are true, `style` is `Inner`, `technique` is `Smooth`, `direction` is `Up`, `depth`/`size`/`soften`/`angle_deg`/`altitude_deg` are the stored values, `use_global_angle` is false, and the highlight and shadow carry the stored colours, opacities and modes

#### Scenario: Missing bevel and emboss keys take Photoshop defaults

- **WHEN** an `ebbl` object carries only `enab` true and `present` true
- **THEN** the decoded `BevelEmboss` has style `Inner`, technique `Smooth`, direction `Up`, depth 100, size 5, soften 0, angle 120, altitude 30, `use_global_angle` true, a Screen/white/75 highlight and a Multiply/black/75 shadow

#### Scenario: The style, technique and direction enums decode every value

- **WHEN** an `ebbl` carries `bvlS` `BESl`/`OtrB`, `bvlT` `bvlT`/`Slmt`, and `bvlD` `BESs`/`Out `, and separately `InrB`, `PrBL` and `In  `
- **THEN** `decode_bevel_emboss` maps the first to `Outer`/`ChiselSoft`/`Down` and the second to `Inner`/`ChiselHard`/`Up`

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** an `ebbl` carries `srgR` 1e30, `blur` 1e30, `Sftn` 1e30, `Lald` 1e30, `hglO` 1e30 and `sdwO` 1e30
- **THEN** `decode_bevel_emboss` returns a `BevelEmboss` with `depth` 1000, `size` 250, `soften` 250, `altitude_deg` 90 and both opacities 100, and rendering it does not panic
- **AND WHEN** an `ebbl` carries `blur` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_bevel_emboss` returns `None` and rendering does not panic

#### Scenario: A disabled bevel and emboss decodes but is inert

- **WHEN** an `ebbl` object has `present` true and `enab` false
- **THEN** `decode_bevel_emboss` returns a `BevelEmboss` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent bevel and emboss effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks an `ebbl`, has an unknown data version, an `ebbl` with a wrong class id, a wrong-typed numeric, a wrong enum typeID, a non-`RGBC` colour, or fails to parse
- **THEN** `decode_bevel_emboss` returns `None` and does not panic

#### Scenario: A bevel with custom contours decodes them [bevel-contour-decode]

- **WHEN** a bevel's `TrnS` and `MpgS` carry custom `Crv` point lists and `useShape` is true
- **THEN** `decode_bevel_emboss` returns both point lists, `edge_contour_on` true, and the stored `Inpr` and `AntA`

#### Scenario: Missing contour keys take the linear default [bevel-contour-defaults]

- **WHEN** a bevel object carries no `TrnS`, no `MpgS`, no `useShape`, no `Inpr` and no `AntA`
- **THEN** both contours are linear, the edge contour is off, the range is 50, and the anti-alias flag is false

#### Scenario: A malformed contour falls back to linear [bevel-contour-malformed]

- **WHEN** a contour's `Crv` is empty, has one point, has a non-finite coordinate, or is the wrong type
- **THEN** that contour decodes as linear, the rest of the bevel still decodes, and decoding does not panic

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
1 pixel, `ChiselHard` not at all. When `edge_contour_on` is true, it SHALL remap
the rounded height through the edge contour before deriving the normal: the
input SHALL be `height * 100 / range` clamped to `0..=1` and the output SHALL be
the contour's 256-step lookup table; `range` SHALL be clamped to `1..=100`. A
linear (identity) edge contour SHALL skip both the remap and the range scaling,
so it is byte-identical to the edge contour off. It SHALL derive the surface
normal from the height gradient scaled by `size` and `depth`, build the light
vector from `angle_deg`/`altitude_deg` with the same sign convention as the
shipped shadows, and take the Lambertian shading as the signed deviation of the
normal from flat. It SHALL remap the signed shading through the gloss contour
before the highlight/shadow split: `t = (s + 1) / 2` clamped to `0..=1`, then
`s = 2 * lut(t) - 1`. A linear (identity) gloss contour SHALL skip the remap and
SHALL be byte-identical to the same document without it. The signed shading
SHALL be negated when `direction` is `Down`; its positive
and negative parts SHALL each be blurred by `soften` pixels separately, so they
spread rather than cancel. The positive part tinted by the highlight colour,
opacity and blend mode and the negative part tinted by the shadow colour,
opacity and blend mode SHALL each be multiplied by the style's coverage and composited above the
content, with the shadow drawn before the highlight: `Inner` by `M` (every pixel
outside `M` byte-identical to the same document without the effect), `Outer` by
`1 - M` (every pixel inside the content byte-identical), `Emboss` by full
coverage, and `Pillow` by `M` plus `1 - M` with the shading negated outside.
The `Stroke` style (which needs the Stroke effect's band) SHALL be a no-op.
`depth` 0 or both effect opacities 0 SHALL be a no-op. A zero-area or fully off-canvas rect or an empty matte SHALL be a
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
- **THEN** the bevel is absent where the mask is zero and present where it is 255

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

#### Scenario: A gloss contour redistributes the light and shade [bevel-gloss-contour]

- **WHEN** the same bevel is composited with a linear gloss contour and again with a contour that darkens the mid-tones
- **THEN** the two composites differ inside the bevel band while pixels outside the band are byte-identical

#### Scenario: An edge contour reshapes the profile [bevel-edge-contour]

- **WHEN** a bevel with the edge contour on is composited with a linear edge contour and again with a bent one
- **THEN** the two composites differ inside the bevel band

#### Scenario: The edge contour switch turns the contour off [bevel-edge-contour-off]

- **WHEN** the same bent edge contour is present once with `useShape` false and once with it true
- **THEN** the false composite is byte-identical to the same document with a linear contour and the true composite differs

#### Scenario: The contour range selects the input band [bevel-contour-range]

- **WHEN** a bent edge contour is rendered at range 100 and at a smaller range
- **THEN** the two composites differ and each stays bounded to the content rect plus the bevel reach

#### Scenario: Linear contours are byte-identical [bevel-contour-linear-noop]

- **WHEN** a bevel with linear `TrnS` and `MpgS`, the edge contour on, and the default range is compared with the same document with the edge contour off
- **THEN** the two composites are byte-identical
