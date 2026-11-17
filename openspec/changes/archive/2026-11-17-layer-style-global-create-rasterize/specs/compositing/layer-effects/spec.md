# Spec Delta

## MODIFIED Requirements

### Requirement: The object-based effects descriptor decodes a bevel and emboss

`pictura-render` SHALL derive a typed
`BevelEmboss { enabled, present, style, technique, direction, depth, size,
soften, angle_deg, altitude_deg, use_global_angle, highlight: {mode, color,
opacity}, shadow: {mode, color, opacity} }` from a layer's `lfx2`
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
(`TrnS`), edge contour (`MpgS`), contour range (`Inpr`), anti-alias
(`AntA`/`antialiasGloss`), texture (`useTexture`, `InvT`, `Algn`, `Scl `,
`Ptrn`), `useShape` and `showInDialog` keys SHALL be ignored. The decoder SHALL
keep the stored `lagl`/`Lald` as the effect's local angle and altitude; when
`use_global_angle` is true the compositor SHALL take the effective light from the
document's global light (see the global-light requirement), so the stored local
values apply only when the flag is false. A missing `lfx2` block, a missing
`ebbl`, an unknown data version, a wrong class id, a wrong-typed or non-finite
numeric value (including a finite `f64` that overflows to infinity as an `f32`),
a wrong enum typeID, a non-`RGBC` colour, or a descriptor that fails to parse
SHALL be a no-op (`None`) and SHALL NOT panic. An `ebbl` whose `present` or
`enab` is false SHALL decode but SHALL render nothing.

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

## ADDED Requirements

### Requirement: Global light drives the effects that use it

For an effect with `use_global_angle` true, the CPU compositor SHALL take its
effective lighting from the document's global light instead of the stored local
values: Drop Shadow and Inner Shadow SHALL use the global angle, and Bevel &
Emboss SHALL use the global angle and the global altitude. For an effect with
the flag false, the stored local angle and altitude SHALL apply unchanged. Satin
SHALL keep its stored angle regardless of any global-light flag. The effective
light SHALL feed both the rendered passes and the reach used to bound them, and
changing the document's global light SHALL invalidate the cached effect passes
of layers whose style bytes did not change, so the next composite repaints.

#### Scenario: A global angle reorients a shadow [gle_angle]

- **WHEN** two documents hold the same layer with an enabled Drop Shadow at a stored angle of 0 and `use_global_angle` true, and one document's global angle is 90 while the other's is 0
- **THEN** the shadow paints on opposite sides of the content in the two composites

#### Scenario: Turning the flag off restores the local angle [gle_local]

- **WHEN** the same layer's `use_global_angle` is false and the document's global angle changes
- **THEN** the composite is unchanged and the shadow keeps its stored angle

#### Scenario: A global light edit repaints cached passes [gle_cache]

- **WHEN** a document with a globally-lit effect is composited, its global angle is changed, and it is composited again
- **THEN** the second composite shows the new light rather than the cached first result

#### Scenario: Global altitude lights a bevel [gle_altitude]

- **WHEN** an enabled Bevel & Emboss has `use_global_angle` true and the document's global altitude changes from 30 to 60
- **THEN** the bevel's highlight and shadow coverage changes with the new altitude

#### Scenario: An inner shadow follows the global angle [gle_inner_shadow]

- **WHEN** two documents hold the same layer with an enabled Inner Shadow at a stored angle of 0 and `use_global_angle` true, and one document's global angle is 90 while the other's is 0
- **THEN** the inner shadow band appears on opposite edges in the two composites

#### Scenario: A bevel highlight follows the global angle [gle_bevel_angle]

- **WHEN** an enabled Bevel & Emboss has `use_global_angle` true and the document's global angle changes from 0 to 180
- **THEN** the highlighted side of the bevel swaps
