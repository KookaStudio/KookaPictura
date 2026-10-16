# layer-effects Specification

## Purpose
Object-based layer effects, from drop shadow to the glow and shadow family, that composite behind or inside layer content.

## Requirements

### Requirement: The object-based effects descriptor decodes a drop shadow

`pictura-core`'s `Layer` SHALL expose its preserved additional-layer-information
blocks by key, and `pictura-render` SHALL derive a typed `DropShadow { enabled,
present, blend_mode, color, opacity, angle_deg, distance, spread, size,
use_global_angle, knocks_out }` from a layer's `lfx2`
(`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block. The block SHALL be read as a
`DescriptorBlock2` (a `u32` version and a `u32` data version, then a version-16
descriptor body) and the top-level `DrSh` object SHALL be decoded. The decoder
SHALL read `enab` as `enabled`, `present` as `present`, `Md  ` (typeID `BlnM`,
the capitalized effect blend vocabulary (`Nrml`/`Mltp`/`Scrn`), not the layer blend key) as `blend_mode`, `Clr ` (an `RGBC` object
whose `Rd `/`Grn `/`Bl  ` values are on the `0..=255` scale) as `color`,
`Opct` as `opacity` in percent, `uglg` as `use_global_angle`, `lagl` as
`angle_deg` in degrees, `Dstn` as `distance` in pixels, `Ckmt` as `spread` in
percent, `blur` as `size` in pixels, and `layerConceals` as `knocks_out`.
Numeric values SHALL be accepted as either a unit float or a `doub`. A missing
`Md  ` SHALL decode to `Normal`, a missing colour SHALL decode to black, and
absent numeric or boolean keys SHALL take the Photoshop defaults (opacity 75,
angle 120, distance 5, spread 0, size 5, use-global-angle true, knocks-out true).
A finite value outside its documented range SHALL be clamped (`opacity` and
`spread` `0..=100`, `distance` `0..=30000`, `size` `0..=250`). A missing `lfx2`
block, a missing `DrSh`, an unknown data version, a wrong-typed or non-finite
numeric value (including a finite `f64` that overflows to infinity as an `f32`),
a wrong `Md  ` typeID, a non-`RGBC` `Clr `, or a descriptor that fails to parse
SHALL be a no-op (`None`) and SHALL NOT panic. A `DrSh` whose `present` or `enab`
is false SHALL decode but SHALL render nothing.

#### Scenario: A drop shadow descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains a `DrSh` with `enab` true, `present` true, `Md  ` value `Mltp`, a non-black `Clr `, `Opct` 60, `uglg` false, `lagl` 45, `Dstn` 8, `Ckmt` 20, `blur` 12, and `layerConceals` false
- **THEN** `decode_drop_shadow` returns a `DropShadow` whose `enabled` and `present` are true, `blend_mode` is `Multiply`, `color` is the stored colour, and `opacity`, `angle_deg`, `distance`, `spread`, `size`, `use_global_angle`, and `knocks_out` are the stored values

#### Scenario: Missing drop shadow keys take Photoshop defaults

- **WHEN** a `DrSh` object carries only `enab` true and `present` true
- **THEN** the decoded `DropShadow` has blend mode `Normal`, black colour, opacity 75, angle 120, distance 5, spread 0, size 5, use-global-angle true, and knocks-out true

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** a `DrSh` carries `blur` 1e30 and `Dstn` 1e30
- **THEN** `decode_drop_shadow` returns a `DropShadow` with `size` 250 and `distance` 30000, and rendering it does not panic
- **AND WHEN** a `DrSh` carries `blur` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_drop_shadow` returns `None` and rendering does not panic

#### Scenario: A disabled drop shadow decodes but is inert

- **WHEN** a `DrSh` object has `present` true and `enab` false
- **THEN** `decode_drop_shadow` returns a `DropShadow` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks a `DrSh`, has an unknown data version, or fails to parse
- **THEN** `decode_drop_shadow` returns `None` and does not panic

### Requirement: A drop shadow composites behind the layer content

For an enabled `DropShadow` on a visible non-group layer with `present` true, the CPU compositor SHALL composite the shadow into the running canvas immediately before the layer's own content, so the shadow lies over the layers below and behind the layer content. It SHALL build a coverage matte from the
layer's content alpha — a pixel layer's alpha channel, a solid fill's payload
alpha, an opaque gradient or embedded smart-object coverage, or a pattern fill's
resolved pattern alpha — multiplied by the layer mask. It SHALL offset the matte
by `distance` pixels along the effect angle using `dx = -distance·cos(angle)` and
`dy = +distance·sin(angle)` in screen coordinates (y down), dilate it by a
spread radius derived from `spread` (a max filter), Gaussian-blur it by a radius
of `size` pixels using the crate's existing blur, and multiply it by
`opacity/100`. It SHALL composite
the result with the effect's `blend_mode` and `opacity`, without applying the
layer's own opacity or fill a second time. A zero-area, fully off-canvas, or
empty matte, `size` 0 with no offset, or `opacity` 0 SHALL be a no-op. A layer
with no effect, or with a disabled or not-present effect, SHALL composite
byte-identically to the same document without the effect. The shadow SHALL be
clipped to the canvas, SHALL NOT panic, and SHALL NOT add a dependency.

#### Scenario: The shadow is offset along the angle

- **WHEN** an opaque square layer over an opaque backdrop has a drop shadow with `distance` 4, `blur` 0, `spread` 0, opacity 100, and `angle_deg` 0
- **THEN** the backdrop pixel four pixels left of the square is darkened and the pixel four pixels right of the square is unchanged

#### Scenario: Distance scales the shadow offset

- **WHEN** the same layer is rendered with `distance` 2 and with `distance` 6 at `angle_deg` 0
- **THEN** the darkened region for `distance` 6 extends farther from the content than for `distance` 2

#### Scenario: Size softens the shadow

- **WHEN** the same layer is rendered with `size` 0 and with `size` 6
- **THEN** the blurred shadow transitions across more pixels than the hard shadow and both darken the backdrop near the content

#### Scenario: Opacity and colour shape the shadow

- **WHEN** a shadow with `opacity` 50 and a red `color` is composited over a white backdrop
- **THEN** the shadowed pixels are a partially transparent red tint rather than black, and lowering `opacity` toward 0 approaches the backdrop

#### Scenario: Spread dilates the shadow

- **WHEN** the same layer is rendered with `spread` 0 and with a positive `spread`
- **THEN** the positive-spread shadow covers no fewer pixels than the unspread shadow, and `spread` 100 with a non-zero `size` produces a harder edge than `spread` 0

#### Scenario: The layer mask shapes the matte

- **WHEN** a shadowed layer's mask hides half the canvas
- **THEN** the shadow is absent where the mask is zero and present where the mask is 255

#### Scenario: The shadow composites behind the content

- **WHEN** a shadow is offset so that it overlaps the layer's own opaque content
- **THEN** the overlapping pixel carries the content colour and not the shadow colour

#### Scenario: A disabled or absent effect is a no-op

- **WHEN** a layer carries no `lfx2` block, or a `DrSh` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes an outer glow

`pictura-render` SHALL derive a typed `OuterGlow { enabled, present, blend_mode,
color, opacity, spread, size, technique }` from a layer's `lfx2`
(`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block. The block SHALL be read as a
`DescriptorBlock2` (a `u32` version and a `u32` data version, then a version-16
descriptor body) and the top-level `OrGl` object SHALL be decoded. The decoder
SHALL read `enab` as `enabled`, `present` as `present`, `Md  ` (typeID `BlnM`,
the capitalized effect blend vocabulary (`Nrml`/`Mltp`/`Scrn`), not the layer blend key) as `blend_mode`, `Clr ` (an `RGBC` object
whose `Rd `/`Grn `/`Bl  ` values are on the `0..=255` scale) as `color`,
`Opct` as `opacity` in percent, `Ckmt` as `spread` in percent, `blur` as `size`
in pixels, and `GlwT` (typeID `BETE`, values `SfBL` Softer or `PrBL` Precise) as
`technique`. Numeric values SHALL be accepted as either a unit float or a
`doub`. A missing `Md  ` SHALL decode to `Screen`, a missing colour SHALL decode
to the Photoshop default `#FFFFBE`, a missing `GlwT` or an unknown technique
value SHALL decode to Softer, and absent numeric keys SHALL take the Photoshop
defaults (opacity 75, spread 0, size 5). A finite value outside its documented
range SHALL be clamped (`opacity` and `spread` `0..=100`, `size` `0..=250`). A
missing `lfx2` block, a missing `OrGl`, an unknown data version, a wrong-typed or
non-finite numeric value (including a finite `f64` that overflows to infinity as
an `f32`), a wrong `Md  ` typeID, a wrong `GlwT` typeID, a non-`RGBC` `Clr `, or
a descriptor that fails to parse SHALL be a no-op (`None`) and SHALL NOT panic.
An `OrGl` whose `present` or `enab` is false SHALL decode but SHALL render
nothing.

#### Scenario: An outer glow descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains an `OrGl` with `enab` true, `present` true, `Md  ` value `Scrn`, a non-default `Clr `, `Opct` 60, `GlwT` value `PrBL`, `Ckmt` 20, and `blur` 10
- **THEN** `decode_outer_glow` returns an `OuterGlow` whose `enabled` and `present` are true, `blend_mode` is `Screen`, `color` is the stored colour, `opacity` is 60, `technique` is `Precise`, `spread` is 20, and `size` is 10

#### Scenario: Missing outer glow keys take Photoshop defaults

- **WHEN** an `OrGl` object carries only `enab` true and `present` true
- **THEN** the decoded `OuterGlow` has blend mode `Screen`, colour `#FFFFBE` (255, 255, 190), opacity 75, technique `Softer`, spread 0, and size 5

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** an `OrGl` carries `blur` 1e30 and `Ckmt` 1e30
- **THEN** `decode_outer_glow` returns an `OuterGlow` with `size` 250 and `spread` 100, and rendering it does not panic
- **AND WHEN** an `OrGl` carries `blur` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_outer_glow` returns `None` and rendering does not panic

#### Scenario: A disabled outer glow decodes but is inert

- **WHEN** an `OrGl` object has `present` true and `enab` false
- **THEN** `decode_outer_glow` returns an `OuterGlow` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks an `OrGl`, has an unknown data version, a wrong-typed `GlwT`, or fails to parse
- **THEN** `decode_outer_glow` returns `None` and does not panic

### Requirement: An outer glow composites behind the layer content

For an enabled `OuterGlow` on a visible non-group layer with `present` true, the CPU compositor SHALL composite the glow into the running canvas immediately before the layer's own content, with no offset, so the glow lies over the layers below and behind and around the layer content. It SHALL build a coverage matte
from the layer's content alpha — a pixel layer's alpha channel, a solid fill's
payload alpha, an opaque gradient or embedded smart-object coverage, or a pattern
fill's resolved pattern alpha — multiplied by the layer mask. It SHALL dilate the
matte by a spread radius derived from `spread` (a max filter), Gaussian-blur it
by a radius of `size` pixels using the crate's existing blur, and multiply the
result by the exterior mask `1 − matte` so the glow is zero where the content is
opaque. It SHALL multiply the result by `opacity/100` and composite it with the
effect's `blend_mode` and `color`, without applying the layer's own opacity or
fill a second time. `Technique::Softer` and `Technique::Precise` SHALL both
render as this blurred exterior matte. A zero-area, fully off-canvas, or empty
matte, or `opacity` 0 SHALL be a no-op. A layer with no effect, or with a
disabled or not-present effect, SHALL composite byte-identically to the same
document without the effect. The glow SHALL be clipped to the canvas, SHALL NOT
panic, and SHALL NOT add a dependency.

#### Scenario: The glow surrounds the content

- **WHEN** an opaque square layer over an opaque white backdrop has an outer glow with `size` 3, `spread` 0, opacity 100, and a non-white colour
- **THEN** backdrop pixels adjacent to the square are tinted toward the glow colour and pixels beyond the glow's reach are unchanged

#### Scenario: The glow is exterior and leaves the content unchanged

- **WHEN** the content pixel of the glowing layer is fully opaque
- **THEN** the composited content pixel keeps the content colour, because the glow is knocked out by the content and composited behind it

#### Scenario: Spread extends the glow

- **WHEN** the same layer is rendered with `spread` 0 and with a positive `spread`
- **THEN** the positive-spread glow covers no fewer pixels than the unspread glow, and `spread` 100 with a non-zero `size` produces a harder edge than `spread` 0

#### Scenario: Size softens the glow

- **WHEN** the same layer is rendered with `size` 0 and with `size` 6
- **THEN** the `size` 6 glow transitions across more backdrop pixels than the `size` 0 result (which, with `spread` 0, is a no-op), so a larger `size` widens the glow's edge

#### Scenario: Opacity and colour shape the glow

- **WHEN** a glow with `opacity` 50 and a red `color` is composited over a white backdrop
- **THEN** the glowing pixels are a partially transparent red tint rather than black, and lowering `opacity` toward 0 approaches the backdrop

#### Scenario: The layer mask shapes the glow

- **WHEN** a glowing layer's mask hides half the canvas
- **THEN** the glow is absent where the mask is zero and present where the mask is 255

#### Scenario: A disabled or absent glow is a no-op

- **WHEN** a layer carries no `lfx2` block, or an `OrGl` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

#### Scenario: The glow is bounded to the content bbox plus reach

- **WHEN** a small content layer with a large `size` is composited on a much larger canvas
- **THEN** pixels outside the content rect padded by the spread and blur reach are byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes an inner shadow

`pictura-render` SHALL derive a typed `InnerShadow { enabled, present,
blend_mode, color, opacity, angle_deg, distance, choke, size, use_global_angle,
knocks_out }` from a layer's `lfx2` (`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged
block. The block SHALL be read as a `DescriptorBlock2` (a `u32` version and a
`u32` data version, then a version-16 descriptor body) and the top-level `IrSh`
object SHALL be decoded. The decoder SHALL read `enab` as `enabled`, `present` as
`present`, `Md  ` (typeID `BlnM`, the capitalized effect blend vocabulary (`Nrml`/`Mltp`/`Scrn`), not the layer blend key) as
`blend_mode`, `Clr ` (an `RGBC` object whose `Rd `/`Grn `/`Bl  ` values are on the
`0..=255` scale) as `color`, `Opct` as `opacity` in percent, `uglg` as
`use_global_angle`, `lagl` as `angle_deg` in degrees, `Dstn` as `distance` in
pixels, `Ckmt` as `choke` in percent, `blur` as `size` in pixels, and
`layerConceals` as `knocks_out`. Numeric values SHALL be accepted as either a
unit float or a `doub`. A missing `Md  ` SHALL decode to `Multiply`, a missing
colour SHALL decode to black, an unknown blend-mode key SHALL decode to
`Multiply`, and absent numeric or boolean keys SHALL take the Photoshop defaults
(opacity 75, angle 120, distance 5, choke 0, size 5, use-global-angle true,
knocks-out true). A finite value outside its documented range SHALL be clamped
(`opacity` and `choke` `0..=100`, `distance` `0..=30000`, `size` `0..=250`). A
missing `lfx2` block, a missing `IrSh`, an unknown data version, a wrong-typed or
non-finite numeric value (including a finite `f64` that overflows to infinity as
an `f32`), a wrong `Md  ` typeID, a non-`RGBC` `Clr `, or a descriptor that fails
to parse SHALL be a no-op (`None`) and SHALL NOT panic. An `IrSh` whose `present`
or `enab` is false SHALL decode but SHALL render nothing.

#### Scenario: An inner shadow descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains an `IrSh` with `enab` true, `present` true, `Md  ` value `Mltp`, a non-black `Clr `, `Opct` 60, `uglg` false, `lagl` 45, `Dstn` 8, `Ckmt` 20, `blur` 12, and `layerConceals` false
- **THEN** `decode_inner_shadow` returns an `InnerShadow` whose `enabled` and `present` are true, `blend_mode` is `Multiply`, `color` is the stored colour, `opacity` is 60, `angle_deg` is 45, `distance` is 8, `choke` is 20, `size` is 12, `use_global_angle` is false, and `knocks_out` is false

#### Scenario: Missing inner shadow keys take Photoshop defaults

- **WHEN** an `IrSh` object carries only `enab` true and `present` true
- **THEN** the decoded `InnerShadow` has blend mode `Multiply`, black colour, opacity 75, angle 120, distance 5, choke 0, size 5, use-global-angle true, and knocks-out true

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** an `IrSh` carries `blur` 1e30, `Dstn` 1e30, and `Ckmt` 1e30
- **THEN** `decode_inner_shadow` returns an `InnerShadow` with `size` 250, `distance` 30000, and `choke` 100, and rendering it does not panic
- **AND WHEN** an `IrSh` carries `blur` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_inner_shadow` returns `None` and rendering does not panic

#### Scenario: A disabled inner shadow decodes but is inert

- **WHEN** an `IrSh` object has `present` true and `enab` false
- **THEN** `decode_inner_shadow` returns an `InnerShadow` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks an `IrSh`, has an unknown data version, a wrong-typed `Md  `, or fails to parse
- **THEN** `decode_inner_shadow` returns `None` and does not panic

### Requirement: An inner shadow composites inside the layer content

An enabled `InnerShadow` SHALL be composited inside the layer content. On a
visible non-group, non-destructive-adjustment layer with `present` true the CPU
compositor SHALL composite it into the running canvas **after** the layer's own
content, so the shadow darkens the content from within. It SHALL build a
coverage matte `M` from the layer's content
alpha — a pixel layer's alpha channel, a solid fill's payload alpha, an opaque
gradient or embedded smart-object coverage, or a pattern fill's resolved pattern
alpha — multiplied by the layer mask. It SHALL form the inverted matte `1 − M`,
offset it by `distance` pixels along the effect angle using
`dx = -distance·cos(angle)` and `dy = +distance·sin(angle)` in screen coordinates
(y down), erode it by a choke radius derived from `choke` (a min filter),
Gaussian-blur it by a radius of `size` pixels using the crate's existing blur,
and multiply the result by `M`, so the effect is confined to the content
interior. It SHALL multiply the result by `opacity/100` and composite it with the
effect's `blend_mode` and no extra layer opacity or fill. `knocks_out` SHALL be
decoded but SHALL NOT change the render. A zero-area, fully off-canvas, or empty
matte, or `opacity` 0 SHALL be a no-op. Every pixel outside the content coverage
`M` SHALL be byte-identical to the same document without the effect. A layer with
no effect, or with a disabled or not-present effect, SHALL composite
byte-identically to the same document without the effect. The shadow SHALL be
clipped to the canvas, SHALL NOT panic, and SHALL NOT add a dependency.

#### Scenario: The shadow is interior and leaves exterior pixels unchanged

- **WHEN** an opaque square layer over an opaque backdrop has an inner shadow with a non-zero `distance` and colour, and the composite is compared with the same document without the effect
- **THEN** pixels outside the square's coverage are byte-identical, and at least one interior pixel near an edge is darkened toward the shadow colour

#### Scenario: The shadow appears on the interior edge away from the offset

- **WHEN** an opaque square layer has an inner shadow with `distance` 4, `size` 0, `choke` 0, opacity 100, and `angle_deg` 0
- **THEN** an interior pixel near the right edge is darkened and an interior pixel near the left edge is unchanged

#### Scenario: Distance widens the interior band

- **WHEN** the same layer is rendered with `distance` 2 and with `distance` 6 at `angle_deg` 0
- **THEN** the darkened interior band for `distance` 6 extends farther from the edge than for `distance` 2

#### Scenario: Choke narrows the interior band

- **WHEN** the same layer is rendered with `choke` 0 and with a positive `choke` at a non-zero `size`
- **THEN** the positive-choke shadow covers no more interior pixels than the unchoked shadow

#### Scenario: Size softens the interior edge

- **WHEN** the same layer is rendered with `size` 0 and with `size` 6
- **THEN** the shadowed interior transition for `size` 6 spans more pixels than the hard `size` 0 band

#### Scenario: Opacity and colour shape the shadow

- **WHEN** an inner shadow with `opacity` 50 and a red `color` is composited over an opaque white content layer
- **THEN** the shadowed interior pixels are a partially transparent red tint rather than black, and lowering `opacity` toward 0 approaches the unshadowed content

#### Scenario: The layer mask shapes the matte

- **WHEN** an inner-shadowed layer's mask hides half the canvas
- **THEN** the shadow is absent where the mask is zero and present where the mask is 255

#### Scenario: The shadow is bounded to the content rect

- **WHEN** a small content layer with a large `size` is composited on a much larger canvas
- **THEN** every pixel outside the content rect is byte-identical to the same document without the effect

#### Scenario: A disabled or absent effect is a no-op

- **WHEN** a layer carries no `lfx2` block, or an `IrSh` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes an inner glow

`pictura-render` SHALL derive a typed `InnerGlow { enabled, present,
blend_mode, color, opacity, choke, size, source, technique }` from a layer's
`lfx2` (`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block. The block SHALL be read
as a `DescriptorBlock2` (a `u32` version and a `u32` data version, then a
version-16 descriptor body) and the top-level `IrGl` object SHALL be decoded. The
decoder SHALL read `enab` as `enabled`, `present` as `present`, `Md  ` (typeID
`BlnM`, the capitalized effect blend vocabulary (`Nrml`/`Mltp`/`Scrn`), not the layer blend key) as `blend_mode`, `Clr ` (an `RGBC`
object whose `Rd `/`Grn `/`Bl  ` values are on the `0..=255` scale) as `color`,
`Opct` as `opacity` in percent, `Ckmt` as `choke` in percent, `blur` as `size` in
pixels, `GlwT` (typeID `BETE`, values `SfBL` Softer or `PrBL` Precise) as
`technique`, and `glwS` (typeID `IGSr`, values `SrcE` Edge or `SrcC` Center) as
`source`. The decoder SHALL also accept the legacy/mis-authored `IGsr` typeID
for `glwS` leniently. Numeric values SHALL be accepted as either a unit float or
a `doub`. A
missing `Md  ` SHALL decode to `Screen`, a missing colour SHALL decode to white,
a missing `GlwT` or an unknown technique value SHALL decode to `Softer`, and a
missing `glwS` or an unknown source value SHALL decode to `Edge`. An unknown
blend-mode key SHALL decode to `Screen`. Absent numeric keys SHALL take the
Photoshop defaults (opacity 75, choke 0, size 5). A finite value outside its
documented range SHALL be clamped (`opacity` and `choke` `0..=100`, `size`
`0..=250`). A missing `lfx2` block, a missing `IrGl`, an unknown data version, a
wrong-typed or non-finite numeric value (including a finite `f64` that overflows
to infinity as an `f32`), a wrong `Md  `, `GlwT` or `glwS` typeID, a non-`RGBC`
`Clr `, or a descriptor that fails to parse SHALL be a no-op (`None`) and SHALL
NOT panic. An `IrGl` whose `present` or `enab` is false SHALL decode but SHALL
render nothing.

#### Scenario: An inner glow descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains an `IrGl` with `enab` true, `present` true, `Md  ` value `Scrn`, a non-white `Clr `, `Opct` 60, `Ckmt` 20, `blur` 12, `GlwT` `BETE`/`PrBL`, and `glwS` `IGSr`/`SrcC`
- **THEN** `decode_inner_glow` returns an `InnerGlow` whose `enabled` and `present` are true, `blend_mode` is `Screen`, `color` is the stored colour, `opacity` is 60, `choke` is 20, `size` is 12, `technique` is `Precise`, and `source` is `Center`

#### Scenario: Missing inner glow keys take Photoshop defaults

- **WHEN** an `IrGl` object carries only `enab` true and `present` true
- **THEN** the decoded `InnerGlow` has blend mode `Screen`, white colour, opacity 75, choke 0, size 5, `technique` `Softer`, and `source` `Edge`

#### Scenario: The source key is decoded from the IGSr enum

- **WHEN** an `IrGl` carries `glwS` with typeID `IGSr` and value `SrcE`, and another carries value `SrcC`
- **THEN** the first decodes to `source` `Edge` and the second to `source` `Center`, and rendering the two produces different composites
- **AND WHEN** an `IrGl` carries `glwS` with the legacy typeID `IGsr`
- **THEN** it still decodes, and a wrong typeID such as `XXXX` is `None`

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** an `IrGl` carries `blur` 1e30 and `Ckmt` 1e30
- **THEN** `decode_inner_glow` returns an `InnerGlow` with `size` 250 and `choke` 100, and rendering it does not panic
- **AND WHEN** an `IrGl` carries `blur` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_inner_glow` returns `None` and rendering does not panic

#### Scenario: A disabled inner glow decodes but is inert

- **WHEN** an `IrGl` object has `present` true and `enab` false
- **THEN** `decode_inner_glow` returns an `InnerGlow` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks an `IrGl`, has an unknown data version, a wrong-typed `Md  `, a wrong `GlwT` or `glwS` typeID, a non-`RGBC` `Clr `, or fails to parse
- **THEN** `decode_inner_glow` returns `None` and does not panic

### Requirement: An inner glow composites inside the layer content

An enabled `InnerGlow` SHALL be composited inside the layer content. On a visible
non-group, non-destructive-adjustment layer with `present` true the CPU
compositor SHALL composite it into the running canvas **after** the layer's own
content, so the glow tints the content from within. It SHALL build a coverage
matte `M` from the layer's content alpha — a pixel layer's alpha channel, a solid
fill's payload alpha, an opaque gradient or embedded smart-object coverage, or a
pattern fill's resolved pattern alpha — multiplied by the layer mask. It SHALL
erode `M` by a choke radius derived from `choke` (a min filter of radius
`round(choke / 100 · size)`), Gaussian-blur the result by a radius of `size`
pixels into `B` using the crate's existing blur, and derive the interior field
`field = 1 - B` for `Source = Edge` or `field = B` for `Source = Center`, then
composite `M · field · opacity/100`. It SHALL composite with the effect's
`blend_mode` and no extra layer opacity or fill.

For `Source = Edge` the field SHALL be strongest at the content edge and fade
inward; for `Source = Center` the field SHALL be strongest away from the edge and
lower at the edge. The `Center` field is the complement of the `Edge` field, an
approximation of Photoshop's distance-from-center falloff. `technique` `Precise`
SHALL render as `Softer`. A zero-area, fully off-canvas, or empty matte, or
`opacity` 0, or (`size` 0 with `choke` 0) under `Source = Edge` SHALL be a no-op.
Every pixel outside the content coverage `M` SHALL be byte-identical to the same
document without the effect. A layer with no effect, or with a disabled or
not-present effect, SHALL composite byte-identically to the same document without
the effect. The glow SHALL be clipped to the canvas, SHALL NOT panic, and SHALL
NOT add a dependency.

#### Scenario: The glow is interior and leaves exterior pixels unchanged

- **WHEN** an opaque square layer over an opaque backdrop has an inner glow with a non-zero `size` and colour, and the composite is compared with the same document without the effect
- **THEN** pixels outside the square's coverage are byte-identical, and at least one interior pixel near an edge is tinted toward the glow colour

#### Scenario: Edge and center sources produce different fields

- **WHEN** the same inner-glowing layer is rendered once with `source` `Edge` and once with `source` `Center`
- **THEN** the two composites differ, with the Edge run tinting pixels adjacent to the content edge and the Center run tinting the interior more than the edge

#### Scenario: Choke strengthens the glow toward the edge

- **WHEN** the same layer is rendered with `choke` 0 and with a positive `choke` at a non-zero `size`
- **THEN** the positive-choke glow is at least as strong in the interior band nearest the edge as the unchoked glow

#### Scenario: Size changes the glow falloff

- **WHEN** the same layer is rendered with `size` 0 and with `size` 6 under `Source = Edge`
- **THEN** `size` 0 produces a byte-identical composite to no effect and `size` 6 tints a band of interior pixels near the edge

#### Scenario: Opacity and colour shape the glow

- **WHEN** an inner glow with `opacity` 50 and a red `color` is composited over an opaque white content layer
- **THEN** the tinted interior pixels are a partially transparent red rather than white, and lowering `opacity` toward 0 approaches the unglowed content

#### Scenario: The layer mask shapes the matte

- **WHEN** an inner-glowing layer's mask hides half the canvas
- **THEN** the glow is absent where the mask is zero and present where the mask is 255

#### Scenario: The glow is bounded to the content rect

- **WHEN** a small content layer with a large `size` is composited on a much larger canvas
- **THEN** every pixel outside the content rect is byte-identical to the same document without the effect

#### Scenario: A disabled or absent effect is a no-op

- **WHEN** a layer carries no `lfx2` block, or an `IrGl` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes a stroke

`pictura-render` SHALL derive a typed `Stroke { enabled, present, blend_mode,
fill, opacity, size, position }` from a layer's `lfx2`
(`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block, where `fill` is a `StrokeFill`
that is `Solid([u8; 3])`, `Gradient { params, align_with_layer }`, or
`Pattern { params, angle_deg }`, reusing the existing `GradientFillParams` and
`PatternFillParams`. The block SHALL be read as a `DescriptorBlock2` (a `u32`
version and a `u32` data version, then a version-16 descriptor body) and the
top-level `FrFX` object SHALL be decoded. The decoder SHALL read `enab` as
`enabled`, `present` as `present`, `Md  ` (typeID `BlnM`, the capitalized effect
blend vocabulary (`Nrml`/`Mltp`/`Scrn`), not the layer blend key) as
`blend_mode`, `Opct` as `opacity` in percent, `Sz  ` as `size` in pixels, `Styl`
(typeID `FStl`, values `OutF` Outside, `InsF` Inside, `CtrF` Center) as
`position`, and `PntT` (typeID `FrFl`) as the fill type. Numeric values SHALL be
accepted as either a unit float or a `doub`. A missing `Md  ` SHALL decode to
`Normal`, an unknown blend-mode key SHALL decode to `Normal`, and a missing
`Styl` or an unknown position value SHALL decode to Outside. Absent numeric keys
SHALL take the Photoshop defaults (opacity 100, size 3). `size` SHALL be an
integer: a finite decoded value SHALL be rounded to the nearest integer and
clamped to `1..=250`, and `opacity` SHALL be clamped to `0..=100`.

`PntT` SHALL default to `SClr`. The value `SClr` SHALL decode to
`StrokeFill::Solid` from `Clr ` (an `RGBC` object whose `Rd `/`Grn `/`Bl  `
values are on the `0..=255` scale; absent → black; a non-`RGBC` `Clr ` SHALL
reject). The value `GrFl` SHALL decode to `StrokeFill::Gradient` from the `Grad`
gradient content, decoded by the shared `fill::gradient_params_from_desc` with
the shared `with_gradient_defaults` injecting an absent `Angl` → 0 and an absent
`Type` → `GrdT`/`Lnr `, and `Algn` (default true) as `align_with_layer`; an
absent or malformed `Grad` SHALL reject. The value `Ptrn` SHALL decode to
`StrokeFill::Pattern` from the `Ptrn` pattern content, decoded by the shared
`fill::pattern_params_from_desc` for `Ptrn`/`Scl `/`phase`, with
`link_with_layer` taken from `Lnkd` when present and otherwise from the value
the helper read (default true), and `Angl` (default 0) decoded into `angle_deg`;
an absent or malformed `Ptrn` SHALL reject. An unknown `PntT` value SHALL
reject. `Clr ` SHALL be ignored for the `GrFl` and `Ptrn` fills.

A missing `lfx2` block, a missing `FrFX`, an unknown data version, a wrong-typed
or non-finite numeric value (including a finite `f64` that overflows to infinity
as an `f32`), a wrong `Md  `, `Styl` or `PntT` typeID, or a descriptor that
fails to parse SHALL be a no-op (`None`) and SHALL NOT panic. An `FrFX` whose
`present` or `enab` is false SHALL decode but SHALL render nothing.

#### Scenario: A stroke descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains an `FrFX` with `enab` true, `present` true, `Md  ` value `Mltp`, a non-black `Clr `, `Opct` 60, `Sz  ` 12, `Styl` `FStl`/`InsF`, and `PntT` `FrFl`/`SClr`
- **THEN** `decode_stroke` returns a `Stroke` whose `enabled` and `present` are true, `blend_mode` is `Multiply`, `fill` is `Solid` with the stored colour, `opacity` is 60, `size` is 12, and `position` is `Inside`

#### Scenario: Missing stroke keys take Photoshop defaults

- **WHEN** an `FrFX` object carries only `enab` true and `present` true
- **THEN** the decoded `Stroke` has blend mode `Normal`, `fill` `Solid([0, 0, 0])`, opacity 100, size 3, and position `Outside`

#### Scenario: The position and fill-type keys are decoded from their enums

- **WHEN** an `FrFX` carries `Styl` typeID `FStl` with value `OutF`, another with `InsF`, and another with `CtrF`
- **THEN** the first decodes to `position` `Outside`, the second to `Inside`, and the third to `Center`
- **AND WHEN** an `FrFX` carries `Styl` with a wrong typeID such as `XXXX`, or `PntT` with a wrong typeID such as `XXXX`
- **THEN** `decode_stroke` returns `None`
- **AND WHEN** an `FrFX` carries `PntT` typeID `FrFl` with an unknown value such as `XXXX`
- **THEN** `decode_stroke` returns `None`

#### Scenario: A gradient fill type decodes to a gradient

- **WHEN** an `FrFX` carries `PntT` `FrFl`/`GrFl`, a `Grad` `Grdn` object with `GrdF` `CstS` and a custom two-stop `Clrs` list, `Angl` 45, `Type` `GrdT`/`Lnr `, `Rvrs` true, `Scl ` 150, and `Algn` false
- **THEN** `decode_stroke` returns a `Stroke` whose `fill` is `Gradient` with `params` carrying the two stops, `reverse` true, `kind` `Linear`, `angle_deg` 45, and `scale` 150, and `align_with_layer` false

#### Scenario: A pattern fill type decodes to a pattern

- **WHEN** an `FrFX` carries `PntT` `FrFl`/`Ptrn`, a `Ptrn` object with `Nm  ` and `Idnt` `pictura-pattern`, `Scl ` 50, `Lnkd` false, and `Angl` 30
- **THEN** `decode_stroke` returns a `Stroke` whose `fill` is `Pattern` with `params.pattern_id` `pictura-pattern`, `params.scale` 50, `params.link_with_layer` false, and `angle_deg` 30

#### Scenario: Malformed or absent gradient or pattern content is a no-op

- **WHEN** an `FrFX` carries `PntT` `FrFl`/`GrFl` but no `Grad`, a `Grad` that is not an object, a `GrdF` that is not `CstS`, or a stop list with fewer than two stops; or carries `PntT` `FrFl`/`Ptrn` but no `Ptrn`, a `Ptrn` that is not an object, or a missing `Idnt`
- **THEN** `decode_stroke` returns `None` and does not panic

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** an `FrFX` carries `Sz  ` 1e30 and `Opct` 1e30
- **THEN** `decode_stroke` returns a `Stroke` with `size` 250 and `opacity` 100, and rendering it does not panic
- **AND WHEN** an `FrFX` carries `Sz  ` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_stroke` returns `None` and rendering does not panic

#### Scenario: A disabled stroke decodes but is inert

- **WHEN** an `FrFX` object has `present` true and `enab` false
- **THEN** `decode_stroke` returns a `Stroke` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks an `FrFX`, has an unknown data version, a wrong-typed `Md  `, a non-`RGBC` `Clr ` for a solid fill, or fails to parse
- **THEN** `decode_stroke` returns `None` and does not panic

### Requirement: A stroke composites as a band at the content edge

An enabled `Stroke` SHALL be composited as a band at the layer's content edge,
above the layer. On a visible non-group, non-destructive-adjustment layer with
`present` true the CPU compositor SHALL composite it into the running canvas
**after** the layer's own content, so the band is drawn on top of the content.
It SHALL build a coverage matte `M` from the layer's content alpha — a pixel
layer's alpha channel, a solid fill's payload alpha, an opaque gradient or
embedded smart-object coverage, or a pattern fill's resolved pattern alpha —
multiplied by the layer mask. Let `n` be the integer `size`; the band SHALL be
formed from exact integer max/min filters with no blur:

- `position` **Outside**: `band(x, y) = dilate(M, n) − M`;
- `position` **Inside**: `band(x, y) = M − erode(M, n)`;
- `position` **Center**: with `out_r = ceil(n/2)` and `in_r = floor(n/2)`,
  `band(x, y) = dilate(M, out_r) − erode(M, in_r)`.

`dilate` is a separable max filter of the given integer radius and `erode` is a
separable min filter of the given radius; the band value SHALL be clamped to
`0..=1`. For each pixel the compositor SHALL source the fill from `fill`:

- `Solid(c)`: the flat colour `c/255` with alpha 1;
- `Gradient { params, align_with_layer }`: the colour of
  `fill::gradient_rgba` (always opaque) generated over the **layer rect** when
  `align_with_layer` is true and over the **canvas** otherwise, the out-of-rect
  aligned sample clamped to the nearest edge;
- `Pattern { params, angle_deg }`: the pattern tile colour and alpha from the
  shipped `Tile` sampler, anchored to the layer rect when
  `params.link_with_layer` is true and to the canvas origin otherwise, so the
  pattern tiles over the band and beyond the content rect; a pattern id absent
  from the document's library SHALL render the shipped grey placeholder rather
  than a no-op; `angle_deg` SHALL NOT be applied.

The compositor SHALL composite `band · source_alpha · opacity/100` with the
effect's own `blend_mode` and no extra layer opacity or fill. Every pixel outside
the content rect padded by `n` on each side SHALL be byte-identical to the same
document without the effect. The Outside band SHALL be zero where `M` is 1, so it
SHALL NOT alter the content interior; the Inside band SHALL be zero where `M` is
0, so it SHALL NOT alter the exterior beyond the content; the Center band SHALL
have pixels both outside and inside the content edge. A zero-area or fully
off-canvas rect, an empty matte, `opacity` 0, or a hand-built `size` of 0 SHALL
be a no-op (a decoded `Sz  ` of 0 clamps to the minimum size of 1, so it renders
a 1-pixel band rather than a no-op). A layer with no effect, or with a disabled
or not-present effect, SHALL composite byte-identically to the same document
without the effect. The stroke SHALL be clipped to the canvas, SHALL NOT panic,
and SHALL NOT add a dependency. The solid fill's composite SHALL be
byte-identical to the previous solid-only renderer.

#### Scenario: The outside band lies outside the content and leaves the interior unchanged

- **WHEN** an opaque square layer over an opaque backdrop has a stroke with `position` `Outside`, a non-zero `size`, opacity 100, and a non-content-colour `color`, and the composite is compared with the same document without the effect
- **THEN** at least one pixel just outside the square is tinted toward the stroke colour, every pixel inside the square's opaque coverage is byte-identical, and pixels beyond the padded reach are byte-identical

#### Scenario: The inside band is confined to the interior

- **WHEN** an opaque square layer over an opaque backdrop has a stroke with `position` `Inside`, a non-zero `size`, opacity 100, and a non-content-colour `color`
- **THEN** at least one pixel just inside the square's edge is tinted toward the stroke colour, and every pixel outside the square's coverage is byte-identical to the same document without the effect

#### Scenario: The centre band straddles the content edge

- **WHEN** the same layer is rendered with `position` `Center` and a non-zero `size`
- **THEN** at least one pixel outside the content edge and at least one pixel inside the content edge are tinted toward the stroke colour

#### Scenario: Size changes the band width

- **WHEN** the same layer is rendered with `size` 2 and with `size` 6 under `position` `Outside`
- **THEN** the `size` 6 band extends at least as far from the content edge as the `size` 2 band, and the `size` 6 band covers no fewer tinted pixels

#### Scenario: Opacity and colour shape the stroke

- **WHEN** a solid stroke with `opacity` 50 and a red `color` is composited over an opaque white content layer
- **THEN** the band pixels are a partially transparent red rather than white, and lowering `opacity` toward 0 approaches the unstroked content

#### Scenario: The blend mode shapes the stroke

- **WHEN** the same stroke is rendered once with `blend_mode` `Normal` and once with a different supported blend mode
- **THEN** the two composites differ

#### Scenario: A gradient stroke paints the band with the gradient

- **WHEN** an opaque square layer over an opaque backdrop has a gradient-fill stroke with a non-constant two-stop gradient and opacity 100
- **THEN** the band pixels outside the content vary along the gradient direction, and every pixel outside the padded reach is byte-identical to the same document without the effect

#### Scenario: A pattern stroke tiles the band beyond the content

- **WHEN** a layer has a pattern-fill stroke whose pattern is in the document library, and the same layer is rendered with a pattern id absent from the library
- **THEN** the band shows the pattern colour in the first case and the grey placeholder in the second case, and neither is a no-op

#### Scenario: The gradient align flag moves the gradient origin

- **WHEN** the same gradient stroke is rendered once with `align_with_layer` true and once false on a layer smaller than the canvas
- **THEN** the two composites differ over the band

#### Scenario: The layer mask shapes the matte

- **WHEN** a stroked layer's mask hides half the canvas
- **THEN** the band is absent where the mask is zero and present where the mask is 255

#### Scenario: The stroke is bounded to the content rect plus reach

- **WHEN** a small content layer with a large `size` is composited on a much larger canvas
- **THEN** every pixel outside the content rect padded by `size` is byte-identical to the same document without the effect

#### Scenario: A disabled or absent effect is a no-op

- **WHEN** a layer carries no `lfx2` block, or an `FrFX` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes a color overlay

`pictura-render` SHALL derive a typed `ColorOverlay { enabled, present,
blend_mode, color, opacity }` from a layer's `lfx2`
(`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block. The block SHALL be read as a
`DescriptorBlock2` (a `u32` version and a `u32` data version, then a version-16
descriptor body) and the top-level object whose class id is **`SoFi`** SHALL be
decoded. The decoder SHALL read `enab` as `enabled`, `present` as `present`,
`Md  ` (typeID `BlnM`) as `blend_mode`, `Clr ` (an `RGBC` object whose
`Rd `/`Grn `/`Bl  ` values are on the `0..=255` scale) as `color`, and `Opct` as
`opacity` in percent. Numeric values SHALL be accepted as either a unit float or
a `doub`. A missing `Md  ` or an unknown blend-mode key SHALL decode to
`Normal`; a missing `Clr ` SHALL decode to red `[255, 0, 0]` (the Photoshop
default, recorded as `(inferred)` — a real file always carries `Clr ` and this
crate cannot resolve a document foreground); a missing `Opct` SHALL decode to
100; `opacity` SHALL be clamped to `0..=100`. A missing `lfx2` block, a missing
`SoFi`, an unknown data version, a wrong-typed or non-finite numeric value
(including a finite `f64` that overflows to infinity as an `f32`), a wrong
`Md  ` typeID, a non-`RGBC` `Clr `, or a descriptor that fails to parse SHALL be
a no-op (`None`) and SHALL NOT panic. A `SoFi` whose `present` or `enab` is
false SHALL decode but SHALL render nothing.

#### Scenario: A color-overlay descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains a `SoFi` object with `enab` true, `present` true, `Md  ` `BlnM`/`Mltp`, a non-black `Clr `, and `Opct` 60
- **THEN** `decode_color_overlay` returns a `ColorOverlay` whose `enabled` and `present` are true, `blend_mode` is `Multiply`, `color` is the stored colour, and `opacity` is 60

#### Scenario: Missing color-overlay keys take Photoshop defaults

- **WHEN** a `SoFi` object carries only `enab` true and `present` true
- **THEN** the decoded `ColorOverlay` has blend mode `Normal`, red `[255, 0, 0]`, and opacity 100

#### Scenario: Out-of-range or non-finite color-overlay numerics are clamped or rejected

- **WHEN** a `SoFi` carries `Opct` 1e30
- **THEN** `decode_color_overlay` returns `opacity` 100 and rendering does not panic
- **AND WHEN** a `SoFi` carries `Opct` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_color_overlay` returns `None` and rendering does not panic

#### Scenario: A disabled color overlay decodes but is inert

- **WHEN** a `SoFi` object has `present` true and `enab` false
- **THEN** `decode_color_overlay` returns a `ColorOverlay` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent color-overlay effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks a `SoFi`, has an unknown data version, a `SoFi` with a wrong class id, a wrong-typed `Md  `, a non-`RGBC` `Clr `, or fails to parse
- **THEN** `decode_color_overlay` returns `None` and does not panic

### Requirement: A color overlay fills the content coverage above the layer

An enabled `ColorOverlay` SHALL be composited as a flat colour over the layer's
content coverage, above the layer. On a visible non-group,
non-destructive-adjustment layer with `present` true the CPU compositor SHALL
composite it **after** the layer's own content. It SHALL build a coverage matte
`M` from the layer's content alpha — a pixel layer's alpha channel, a solid
fill's payload alpha, an opaque gradient or embedded smart-object coverage, or a
pattern fill's resolved pattern alpha — multiplied by the layer mask, and SHALL
composite, for each pixel with `M > 0`, the colour `color` with alpha
`M · opacity/100` using the overlay's own `blend_mode` and no extra layer opacity
or fill. Every pixel outside the content rect SHALL be byte-identical to the same
document without the effect. A zero-area or fully off-canvas rect, an empty
matte, or `opacity` 0 SHALL be a no-op. A layer with no effect, or with a
disabled or not-present effect, SHALL composite byte-identically to the same
document without the effect. The overlay SHALL be clipped to the canvas, SHALL
NOT panic, and SHALL NOT add a dependency.

#### Scenario: The color overlay is confined to the content

- **WHEN** an opaque square layer over an opaque backdrop has an enabled and present color overlay with a non-content colour and opacity 100, and the composite is compared with the same document without the effect
- **THEN** every pixel inside the square's opaque coverage is the overlay colour and every pixel outside it is byte-identical

#### Scenario: Opacity and blend mode shape the color overlay

- **WHEN** the same layer is rendered with `opacity` 50 and with `opacity` 100, and once with `blend_mode` `Normal` and once with a different supported blend mode
- **THEN** the 50 % composite is a partial blend toward the content, the 100 % composite is the overlay colour, and the two blend-mode composites differ

#### Scenario: The layer mask shapes the color overlay

- **WHEN** a layer's mask hides half the canvas
- **THEN** the overlay is absent where the mask is zero and present where the mask is 255

#### Scenario: The color overlay is bounded to the content rect

- **WHEN** a small content layer is composited on a much larger canvas
- **THEN** every pixel outside the content rect is byte-identical to the same document without the effect

#### Scenario: A disabled or absent color overlay is a no-op

- **WHEN** a layer carries no `lfx2` block, or a `SoFi` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes a gradient overlay

`pictura-render` SHALL derive a typed `GradientOverlay { enabled, present,
blend_mode, opacity, stops, reverse, kind, angle_deg, scale, align_with_layer }`
from a layer's `lfx2` block, reusing `pictura-adjust`'s `GradientStop` and
`GradientKind`. The decoder SHALL find the top-level object whose class id is
**`GrFl`** and read `enab`, `present`, `Md  ` (typeID `BlnM`) as `blend_mode`
(default/unknown → `Normal`), `Opct` as `opacity` in percent (default 100,
clamped `0..=100`), `Grad` (a `Grdn` object carrying `GrdF` `CstS` and a `Clrs`
list of custom stops) as `stops`, `Angl` as `angle_deg` (default 0), `Type`
(typeID `GrdT`) as `kind` (default `Linear`), `Rvrs` as `reverse` (default
false), `Scl ` as `scale` in percent (default 100), and `Algn` as
`align_with_layer` (default true). `Ofst`, noise and `Dthr` SHALL be ignored.
Numeric values SHALL be accepted as a unit float or a `doub`; a finite
out-of-range `opacity` SHALL be clamped; a non-finite value (including a finite
`f64` that overflows `f32`) SHALL reject the effect. A missing `lfx2`, a missing
`GrFl`, an unknown data version, a wrong class id, a wrong `Md  `/`Type`/`GrdF`
typeID, a `Grad` that is not an object, a stop list with fewer than two or
non-increasing locations, or a parse error SHALL be `None`, and the decoder
SHALL NOT panic. A `GrFl` whose `present` or `enab` is false SHALL decode but
SHALL render nothing.

#### Scenario: A gradient-overlay descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains a `GrFl` with `enab` true, `present` true, `Md  ` `BlnM`/`Nrml`, `Opct` 80, a custom two-stop black-to-white `Grad`, `Angl` 45, `Type` `GrdT`/`Lnr `, `Rvrs` true, `Scl ` 150, and `Algn` false
- **THEN** the decoded `GradientOverlay` has `enabled`/`present` true, `blend_mode` `Normal`, `opacity` 80, two stops, `reverse` true, `kind` `Linear`, `angle_deg` 45, `scale` 150, and `align_with_layer` false

#### Scenario: Missing gradient-overlay keys take Photoshop defaults

- **WHEN** a `GrFl` carries a valid `Grad` and omits `Rvrs`, `Scl `, `Algn`, `Md  ` and `Opct`
- **THEN** the decoded `GradientOverlay` has `reverse` false, `scale` 100, `align_with_layer` true, blend mode `Normal`, and opacity 100

#### Scenario: A malformed gradient overlay is a no-op

- **WHEN** a `GrFl` has a wrong `Type` typeID, a `Grad` that is not an object, a `GrdF` that is not `CstS`, a stop list with fewer than two stops or non-increasing locations, or a non-finite `Angl`
- **THEN** `decode_gradient_overlay` returns `None` and does not panic

#### Scenario: A disabled gradient overlay decodes but is inert

- **WHEN** a `GrFl` has `present` true and `enab` false
- **THEN** `decode_gradient_overlay` returns a `GradientOverlay` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

### Requirement: A gradient overlay fills the content coverage above the layer

An enabled `GradientOverlay` SHALL be composited as the gradient source over the
layer's content coverage, above the layer. On a visible non-group,
non-destructive-adjustment layer with `present` true the CPU compositor SHALL
generate the gradient with the shipped gradient geometry (respecting `kind`,
`angle_deg`, `scale` and `reverse`) and composite it **after** the layer's own
content. The gradient SHALL be generated over the **layer rect** when
`align_with_layer` is true (pixel `(x, y)` samples local `(x - left, y - top)`)
and over the **canvas** otherwise (pixel `(x, y)` samples `(x, y)`). Using the
masked content matte `M`, the compositor SHALL composite, for each pixel with
`M > 0`, the sampled gradient colour with alpha `M · opacity/100` using the
overlay's own `blend_mode` and no extra layer opacity or fill. Every pixel
outside the content rect SHALL be byte-identical to the same document without the
effect. A zero-area or fully off-canvas rect, an empty matte, or `opacity` 0
SHALL be a no-op. A layer with no effect, or with a disabled or not-present
effect, SHALL composite byte-identically to the same document without the effect.
The overlay SHALL be clipped to the canvas, SHALL NOT panic, and SHALL NOT add a
dependency.

#### Scenario: The gradient overlay is confined to the content

- **WHEN** a small opaque square layer over a large opaque backdrop has an enabled and present gradient overlay with opacity 100
- **THEN** every pixel outside the square's content rect is byte-identical to the same document without the effect, and at least one pixel inside varies with the gradient

#### Scenario: The gradient geometry follows the overlay parameters

- **WHEN** the same gradient overlay is rendered with two different `angle_deg`, `scale`, `kind` or `reverse` values
- **THEN** the composites differ

#### Scenario: Align with layer moves the gradient origin

- **WHEN** the same gradient overlay is rendered once with `align_with_layer` true and once false on a layer smaller than the canvas
- **THEN** the two composites differ, showing that the gradient geometry is anchored to the layer rect or the canvas respectively

#### Scenario: Opacity and blend mode shape the gradient overlay

- **WHEN** the same layer is rendered with `opacity` 50 and 100, and once with `blend_mode` `Normal` and once with a different supported blend mode
- **THEN** lowering opacity approaches the content, and the two blend-mode composites differ

#### Scenario: A disabled or absent gradient overlay is a no-op

- **WHEN** a layer carries no `lfx2` block, or a `GrFl` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes a pattern overlay

`pictura-render` SHALL derive a typed `PatternOverlay { enabled, present,
blend_mode, opacity, pattern_id, scale, angle_deg, align_with_layer }` from a
layer's `lfx2` block. The decoder SHALL find the top-level object whose class id
is **`patternFill`** and read `enab` and `present`, `Md  ` (typeID `BlnM`) as
`blend_mode` (default/unknown → `Normal`), `Opct` as `opacity` in percent
(default 100, clamped `0..=100`), `Ptrn` (a `Ptrn` object carrying `Nm  ` and a
non-empty `Idnt`) as `pattern_id`, `Scl ` as `scale` in percent (default 100),
`Algn` as `align_with_layer` (default true), and `Angl` as `angle_deg` (default
0, decoded for symmetry but not applied). An optional `phase` `Pnt ` object's
`Hrzn`/`Vrtc` SHALL supply the tile origin, defaulting to `(0, 0)`. Numeric
values SHALL be accepted as a unit float or a `doub`; a non-finite value
(including a finite `f64` that overflows `f32`) SHALL reject the effect. A
missing `lfx2`, a missing `patternFill`, an unknown data version, a wrong class
id, a wrong `Md  ` typeID, a `Ptrn` that is not an object, a missing `Idnt`, or
a parse error SHALL be `None`, and the decoder SHALL NOT panic. A `patternFill`
whose `present` or `enab` is false SHALL decode but SHALL render nothing.

#### Scenario: A pattern-overlay descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains a `patternFill` with `enab` true, `present` true, `Md  ` `BlnM`/`Nrml`, `Opct` 80, a `Ptrn` object with `Nm  ` and `Idnt` `pictura-pattern`, `Scl ` 50, `Angl` 30, and `Algn` true
- **THEN** the decoded `PatternOverlay` has `enabled`/`present` true, `blend_mode` `Normal`, `opacity` 80, `pattern_id` `pictura-pattern`, `scale` 50, `angle_deg` 30, and `align_with_layer` true

#### Scenario: Missing pattern-overlay keys take Photoshop defaults

- **WHEN** a `patternFill` carries a valid `Ptrn` and omits `Scl `, `Algn`, `Md  ` and `Opct`
- **THEN** the decoded `PatternOverlay` has `scale` 100, `align_with_layer` true, blend mode `Normal`, and opacity 100

#### Scenario: A malformed pattern overlay is a no-op

- **WHEN** a `patternFill` has a `Ptrn` that is not an object, a missing `Idnt`, a wrong-typed `Md  `, or a non-finite `Scl `
- **THEN** `decode_pattern_overlay` returns `None` and does not panic

#### Scenario: A disabled pattern overlay decodes but is inert

- **WHEN** a `patternFill` has `present` true and `enab` false
- **THEN** `decode_pattern_overlay` returns a `PatternOverlay` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

### Requirement: A pattern overlay fills the content coverage above the layer

An enabled `PatternOverlay` SHALL be composited as the pattern source over the
layer's content coverage, above the layer. On a visible non-group,
non-destructive-adjustment layer with `present` true the CPU compositor SHALL
resolve the pattern from the document's decoded pattern library and tile it with
the shipped tiling (respecting `scale`), and composite it **after** the layer's
own content. The tile SHALL be anchored to the layer rect when
`align_with_layer` is true and to the document origin otherwise, offset by the
`phase` origin when present; a pattern id that is not in the library SHALL render
the shipped grey placeholder rather than a no-op. Using the masked content matte
`M`, the compositor SHALL composite, for each pixel with `M > 0`, the pattern
colour with alpha `M · pattern_alpha/255 · opacity/100` using the overlay's own
`blend_mode` and no extra layer opacity or fill. `angle_deg` SHALL NOT be
applied. Every pixel outside the content rect SHALL be byte-identical to the same
document without the effect. A zero-area or fully off-canvas rect, an empty
matte, or `opacity` 0 SHALL be a no-op. A layer with no effect, or with a
disabled or not-present effect, SHALL composite byte-identically to the same
document without the effect. The overlay SHALL be clipped to the canvas, SHALL
NOT panic, and SHALL NOT add a dependency.

#### Scenario: The pattern overlay is confined to the content

- **WHEN** a small opaque square layer over a large opaque backdrop has an enabled and present pattern overlay with opacity 100
- **THEN** every pixel outside the square's content rect is byte-identical to the same document without the effect, and at least one pixel inside shows the pattern colour

#### Scenario: A missing pattern falls back to the placeholder

- **WHEN** a pattern overlay references a pattern id that is not in the document's pattern library
- **THEN** the composite inside the content shows the grey placeholder rather than being a no-op, and does not panic

#### Scenario: Scale and align with layer shape the pattern overlay

- **WHEN** the same pattern overlay is rendered with `scale` 100 and 200, and once with `align_with_layer` false instead of true
- **THEN** the tiled composites differ

#### Scenario: Opacity and blend mode shape the pattern overlay

- **WHEN** the same layer is rendered with `opacity` 50 and 100, and once with `blend_mode` `Normal` and once with a different supported blend mode
- **THEN** lowering opacity approaches the content, and the two blend-mode composites differ

#### Scenario: A disabled or absent pattern overlay is a no-op

- **WHEN** a layer carries no `lfx2` block, or a `patternFill` with `enab` false
- **THEN** the composite is byte-identical to the same document without the effect

### Requirement: The object-based effects descriptor decodes a satin

`pictura-render` SHALL derive a typed `Satin { enabled, present, blend_mode,
color, opacity, angle_deg, distance, size, invert }` from a layer's `lfx2`
(`OBJECT_BASED_EFFECTS_LAYER_INFO`) tagged block. The block SHALL be read as a
`DescriptorBlock2` (a `u32` version and a `u32` data version, then a version-16
descriptor body) and the top-level object whose class id is **`ChFX`** SHALL be
decoded. The decoder SHALL read `enab` as `enabled`, `present` as `present`,
`Md  ` (typeID `BlnM`) as `blend_mode`, `Clr ` (an `RGBC` object whose
`Rd `/`Grn `/`Bl  ` values are on the `0..=255` scale) as `color`, `Opct` as
`opacity` in percent, `lagl` as `angle_deg` in degrees, `Dstn` as `distance` in
pixels, `blur` as `size` in pixels, and `Invr` as `invert`. Numeric values SHALL
be accepted as either a unit float or a `doub`. A missing `Md  ` or an unknown
blend-mode key SHALL decode to `Multiply`; a missing `Clr ` SHALL decode to black
`[0, 0, 0]` (the Photoshop default, recorded as `(inferred)` — a real file always
carries `Clr ` and this crate cannot resolve a document foreground); absent
numeric keys SHALL take the Photoshop UI defaults (opacity 50, angle 19,
distance 11, size 14) and a missing `Invr` SHALL decode to false. A finite value
outside its documented range SHALL be clamped (`opacity` `0..=100`, `distance`
`0..=30000`, `size` `0..=250`). The contour (`MpgS`), anti-alias (`AntA`),
global-light flag (`uglg`) and `showInDialog` keys SHALL be ignored; the
effective angle SHALL be the stored `lagl`. A missing `lfx2` block, a missing
`ChFX`, an unknown data version, a wrong class id, a wrong-typed or non-finite
numeric value (including a finite `f64` that overflows to infinity as an `f32`),
a wrong `Md  ` typeID, a non-`RGBC` `Clr `, or a descriptor that fails to parse
SHALL be a no-op (`None`) and SHALL NOT panic. A `ChFX` whose `present` or `enab`
is false SHALL decode but SHALL render nothing.

#### Scenario: A satin descriptor decodes to typed parameters

- **WHEN** an `lfx2` block contains a `ChFX` object with `enab` true, `present` true, `Md  ` `BlnM`/`Mltp`, a non-black `Clr `, `Opct` 60, `lagl` 45, `Dstn` 8, `blur` 12, and `Invr` true
- **THEN** `decode_satin` returns a `Satin` whose `enabled` and `present` are true, `blend_mode` is `Multiply`, `color` is the stored colour, and `opacity`, `angle_deg`, `distance`, `size` and `invert` are the stored values

#### Scenario: Missing satin keys take Photoshop defaults

- **WHEN** a `ChFX` object carries only `enab` true and `present` true
- **THEN** the decoded `Satin` has blend mode `Multiply`, black colour, opacity 50, angle 19, distance 11, size 14, and `invert` false

#### Scenario: Out-of-range numeric values are clamped or rejected

- **WHEN** a `ChFX` carries `blur` 1e30, `Dstn` 1e30, and `Opct` 1e30
- **THEN** `decode_satin` returns a `Satin` with `size` 250, `distance` 30000, and `opacity` 100, and rendering it does not panic
- **AND WHEN** a `ChFX` carries `blur` 1e300 (a finite `f64` that overflows `f32`)
- **THEN** `decode_satin` returns `None` and rendering does not panic

#### Scenario: A disabled satin decodes but is inert

- **WHEN** a `ChFX` object has `present` true and `enab` false
- **THEN** `decode_satin` returns a `Satin` with `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect

#### Scenario: Malformed or absent satin effects are a no-op

- **WHEN** a layer has no `lfx2` block, or its `lfx2` block lacks a `ChFX`, has an unknown data version, a `ChFX` with a wrong class id, a wrong-typed `Md  `, a non-`RGBC` `Clr `, or fails to parse
- **THEN** `decode_satin` returns `None` and does not panic

### Requirement: A satin composites inside the layer content

An enabled `Satin` SHALL be composited inside the layer content as a directional
interior band. On a visible non-group, non-destructive-adjustment layer with
`present` true the CPU compositor SHALL composite it **after** the layer's own
content. It SHALL build a coverage matte `M` from the layer's content alpha — a
pixel layer's alpha channel, a solid fill's payload alpha, an opaque gradient or
embedded smart-object coverage, or a pattern fill's resolved pattern alpha —
multiplied by the layer mask, and a Gaussian blur `B = blur(M, size)` of radius
`size` pixels using the crate's existing blur. With the clamped `distance`, the
finite `angle`, and screen coordinates (y down), it SHALL shift the field by
`dx = -round(distance·cos(angle))` and `dy = +round(distance·sin(angle))` and
form

```
band(x, y)  = |B(x - dx, y - dy) - B(x + dx, y + dy)|   clamped to 0..=1
field(x, y) = invert ? 1 - band : band
satin(x, y) = M(x, y) · field(x, y) · opacity/100
```

then composite `satin` tinted by `color` with the effect's own `blend_mode` and
no extra layer opacity or fill. The contour (`MpgS`), anti-alias (`AntA`) and the
global-light resource SHALL NOT change the render. `invert` false SHALL leave the
band; `invert` true SHALL use its complement. Every pixel outside the content
coverage `M` SHALL be byte-identical to the same document without the effect. A
zero-area or fully off-canvas rect, an empty matte, or `opacity` 0 SHALL be a
no-op; a zero `distance` with `invert` false SHALL be a no-op. A layer with no
effect, or with a disabled or not-present effect, SHALL composite byte-identically
to the same document without the effect. The satin SHALL be clipped to the canvas,
SHALL NOT panic, and SHALL NOT add a dependency.

#### Scenario: The satin is interior and leaves exterior pixels unchanged

- **WHEN** an opaque square layer over an opaque backdrop has an enabled and present satin with a non-zero `distance`, `size` and colour, and the composite is compared with the same document without the effect
- **THEN** every pixel outside the square's coverage is byte-identical, and at least one interior pixel differs from the un-satined content

#### Scenario: Angle and distance shape the satin band

- **WHEN** the same satin is rendered with two different `angle_deg` values, and separately with two different `distance` values at a fixed angle
- **THEN** the composites differ, showing that the band direction and width follow `angle_deg` and `distance`

#### Scenario: Size softens the satin band

- **WHEN** the same satin is rendered with `size` 0 and with a larger `size`
- **THEN** the larger-`size` transition spans at least as many interior pixels as the `size` 0 band

#### Scenario: Invert flips the satin field

- **WHEN** the same satin is rendered once with `invert` false and once with `invert` true
- **THEN** the two composites differ, with the inverted run using the complement of the band

#### Scenario: Opacity, colour and blend mode shape the satin

- **WHEN** a satin with `opacity` 50 and a red `color` is composited over an opaque white content layer, and once more with `opacity` 100 and a different supported `blend_mode`
- **THEN** lowering opacity approaches the unsatined content, and the two blend-mode composites differ

#### Scenario: The layer mask shapes the matte

- **WHEN** a satined layer's mask hides half the canvas
- **THEN** the satin is absent where the mask is zero and present where the mask is 255

#### Scenario: The satin is bounded to the content rect plus reach

- **WHEN** a small content layer with a large `size` and `distance` is composited on a much larger canvas
- **THEN** every pixel outside the content rect padded by the distance and blur reach is byte-identical to the same document without the effect

#### Scenario: A disabled or absent satin is a no-op

- **WHEN** a layer carries no `lfx2` block, or a `ChFX` with `enab` false, or a `ChFX` with `distance` 0 and `invert` false
- **THEN** the composite is byte-identical to the same document without the effect

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
`Ptrn`), `useShape` and `showInDialog` keys SHALL be ignored; the effective angle
and altitude SHALL be the stored `lagl`/`Lald`. A missing `lfx2` block, a missing
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

### Requirement: The legacy effects block decodes into the typed effect set

`pictura-render` SHALL derive the existing typed effect params
(`DropShadow`, `InnerShadow`, `OuterGlow`, `InnerGlow`, `BevelEmboss` and
`ColorOverlay`) from a layer's legacy `lrFX` (`EFFECTS_LAYER`) tagged block via
`decode_legacy_effects(&Layer) -> Option<LegacyEffects>`. The block SHALL be
parsed as the fixed `EffectsLayer` binary struct — a `u16` version (0) and a
`u16` count, then `count` records each beginning `8BIM` followed by a 4-byte
`ostype` (`cmnS`, `dsdw`, `isdw`, `oglw`, `iglw`, `bevl`, `sofi`) and a `u32`
body length — with all integers big-endian and no alignment padding. The decoder
SHALL read the shared `CommonStateInfo` (`cmnS`: `u32` version, `u8` visible) and
AND its `visible` flag into every record's `enabled` (`visible` defaults to true
when `cmnS` is absent). Every present record SHALL decode with `present` true,
because a record exists only when written.

The decoder SHALL map the fields `lrFX` carries and take the typed effect
defaults for the `lfx2`-only fields it does not:

- shadow records (`dsdw`/`isdw`): `blend_mode` from the stored 4-byte layer
  blend key through `BlendMode::from_psd_key` (not the capitalized `BlnM`
  vocabulary), `color` from the 16-bit `Color` channels (`value >> 8`),
  `opacity` from the `u8` scaled `0..=255 → 0..=100`, `angle` as `angle_deg`,
  `distance`, `blur` as `size`, `intensity` as `spread` (drop) or `choke`
  (inner), and `use_global_angle`; a shadow's `knocks_out` SHALL be false and
  `present` true;
- glow records (`oglw`/`iglw`): `blend_mode`, `color`, `opacity`, `blur` as
  `size` and `intensity` as `spread` (outer) or `choke` (inner), with `technique`
  `Softer`; an inner glow's `source` SHALL be `Center` when a version-2 `invert`
  byte is non-zero and `Edge` otherwise;
- bevel (`bevl`): `angle` as `angle_deg`, `depth`, `blur` as `size`,
  `soften` 0, `altitude_deg` 30, `use_global_angle`, the `bevel_style` byte as
  the `BevelStyle` (0 `Outer`, 1 `Inner`, 2 `Emboss`, 3 `Pillow`, 4 `Stroke`),
  the `direction` byte as the `BevelDirection`, `technique` `Smooth`, and both
  halves' blend mode, colour and opacity;
- solid fill (`sofi`): `blend_mode`, `color` and `opacity`; a `sofi` whose
  version is not 2 SHALL be skipped.

A finite value outside its typed range SHALL be clamped to the same bounds the
shipped `lfx2` decoders use (`opacity` `0..=100`, `distance` `0..=30000`, `size`
and `soften` `0..=250`, `choke`/`spread` `0..=100`, bevel `depth` `0..=1000` and
`altitude_deg` `0..=90`). A missing `lrFX` block, an unsupported `EffectsLayer`
version, a truncated body, a record whose signature is not `8BIM`, or a count
that overruns the payload SHALL return `None`; an individually unknown or
malformed record SHALL be skipped without failing the block. The decoder SHALL
NOT panic.

#### Scenario: A legacy drop shadow and outer glow decode to typed parameters

- **WHEN** an `lrFX` block contains `cmnS` visible 1, a `dsdw` with `blur` 6, `intensity` 10, `angle` 45, `distance` 8, a non-black `Color`, blend key `mul `, enabled 1, use-global-angle 0 and opacity 255, and an `oglw` with `blur` 10, `intensity` 20, a non-white `Color`, blend key `scrn`, enabled 1 and opacity 128
- **THEN** `decode_legacy_effects` returns a `LegacyEffects` whose `drop_shadow` has `enabled` true, `present` true, `blend_mode` `Multiply`, the stored colour, `opacity` 100, `angle_deg` 45, `distance` 8, `spread` 10 and `size` 6, and whose `outer_glow` has `enabled` true, `present` true, `blend_mode` `Screen`, the stored colour, `spread` 20, `size` 10, `technique` `Softer` and an opacity equal to `128 · 100 / 255`

#### Scenario: A legacy inner shadow and inner glow decode with their choke and source

- **WHEN** an `lrFX` block contains an `isdw` with `intensity` 15 and an `iglw` version 2 with a non-zero `invert` byte and `intensity` 25
- **THEN** the `inner_shadow` has `choke` 15 and the `inner_glow` has `choke` 25, `source` `Center` and `technique` `Softer`
- **AND WHEN** the `iglw` has a zero `invert` byte
- **THEN** its `source` is `Edge`

#### Scenario: A legacy bevel and solid fill decode to typed parameters

- **WHEN** an `lrFX` block contains a `bevl` with `angle` 120, `depth` 100, `blur` 5, `bevel_style` 1, `direction` 0, highlight and shadow blend keys, colours and opacities, and enabled 1, and a version-2 `sofi` with a blend key, colour, opacity 191 and enabled 1
- **THEN** the `bevel` has `style` `Inner`, `technique` `Smooth`, `direction` `Up`, `size` 5, `depth` 100, `altitude_deg` 30 and `soften` 0, and the `color_overlay` has the stored blend mode, colour and an opacity equal to `191 · 100 / 255`

#### Scenario: The common state gates every legacy effect

- **WHEN** an `lrFX` block has a `cmnS` whose `visible` is 0 and a `dsdw` whose own enabled byte is 1
- **THEN** the decoded `drop_shadow` has `enabled` false, and compositing the layer leaves the output byte-identical to the same document without the effect
- **AND WHEN** the block has no `cmnS` record
- **THEN** the record's own enabled byte alone decides `enabled`

#### Scenario: Fields lrFX does not carry take the effect defaults

- **WHEN** a legacy drop shadow, outer glow, inner glow and bevel are decoded
- **THEN** the drop shadow takes the `lfx2` default blend mode and colour for an absent key, the glows take `technique` `Softer`, the inner glow defaults `source` to `Edge`, and the bevel takes `technique` `Smooth`, `soften` 0 and `altitude_deg` 30

#### Scenario: A malformed block is a no-op and never panics

- **WHEN** a layer has no `lrFX` block, or its `lrFX` block has an unsupported version, a truncated record body, a record signature other than `8BIM`, or a count that overruns the payload
- **THEN** `decode_legacy_effects` returns `None` and does not panic
- **AND WHEN** a block contains an unknown `ostype` record, a `sofi` whose version is not 2, or a `Color` with an unknown space alongside a valid `dsdw`
- **THEN** the unknown or malformed record is skipped and the valid `dsdw` still decodes

### Requirement: The compositor resolves lfx2 over the legacy effects block

The decode path the compositor uses SHALL be a single resolver,
`decode_layer_effects(&Layer) -> LayerEffects`, which SHALL be authoritative over
both effect encodings. When a layer has an `lfx2` block, the resolver SHALL read
every effect from `lfx2` and SHALL ignore the `lrFX` block entirely; when the
layer has no `lfx2` block, the resolver SHALL derive the mapped set from `lrFX`
and SHALL leave the effects with no legacy record (`satin`, `stroke`, gradient
overlay, pattern overlay) absent. The resolver SHALL NOT combine the two encodings
and SHALL NOT apply the same effect twice. Both compositor passes
(`composite_layer_effects` and `composite_layer_effects_above`) SHALL consume the
resolved set, with the same group and destructive-adjustment skip as today.

#### Scenario: lfx2 wins when both blocks are present

- **WHEN** a layer carries an `lfx2` block whose `DrSh` is enabled and present and a legacy `lrFX` block whose `dsdw` describes a different shadow
- **THEN** the layer's resolved drop shadow is the `lfx2` one, and the composite is byte-identical to the same document with the `lrFX` block removed

#### Scenario: The legacy block is used when lfx2 is absent

- **WHEN** a layer carries only a legacy `lrFX` block with an enabled and present `dsdw`
- **THEN** the resolved drop shadow is the decoded legacy shadow and the layer composite differs from the same document without the effect

#### Scenario: An lfx2 block that omits an effect suppresses its legacy record

- **WHEN** a layer carries an `lfx2` block with an outer glow but no `DrSh`, and a legacy `lrFX` block with a `dsdw`
- **THEN** the resolved set has no drop shadow and only the outer glow renders

#### Scenario: The legacy block supplies only the effects it maps

- **WHEN** a layer carries only a legacy `lrFX` block with a `dsdw` and no other record
- **THEN** the resolved `satin`, `stroke`, gradient overlay and pattern overlay are absent and no effect other than the drop shadow renders

### Requirement: A legacy effect renders through the shipped layer-effect renderers

An enabled and present effect resolved from a legacy `lrFX` block SHALL render
through the same shipped `composite_*` functions an equivalent `lfx2` effect
uses, with no change to those functions: a drop shadow and outer glow composite
below the layer content, and an inner shadow, inner glow, bevel and color overlay
composite above it. Fields the legacy record carries SHALL drive the render, and
a legacy effect SHALL composite identically to an `lfx2` effect whose typed
params are equal for the render-affecting fields. A resolved effect that maps to
a renderer the shipped code declines (for example a bevel whose style is not
`Inner`, or a decoded glow whose size and offsets make it a no-op) SHALL leave
the composite byte-identical to the same document without the effect. A disabled
or not-present resolved effect, an effect absent from both blocks, or a legacy
extension with no record SHALL be a no-op. The composite SHALL be clipped to the
canvas, SHALL NOT panic, and SHALL NOT add a dependency.

#### Scenario: A legacy drop shadow renders like the equivalent lfx2 shadow

- **WHEN** two documents carry an identically masked content layer, one with an `lfx2` `DrSh` and one with a legacy `lrFX` `dsdw` whose decoded `DropShadow` has the same blend mode, colour, opacity, angle, distance, spread and size
- **THEN** the two composites are byte-identical

#### Scenario: A legacy outer glow renders around the content

- **WHEN** an opaque square layer over an opaque backdrop has a legacy `oglw` with a non-zero `size` and a non-white colour
- **THEN** backdrop pixels adjacent to the square are tinted toward the glow colour and pixels beyond the glow's reach are byte-identical to the same document without the effect

#### Scenario: A legacy interior effect renders above the content

- **WHEN** an opaque square layer has a legacy `isdw`, `iglw` or version-2 `sofi`
- **THEN** at least one pixel inside the square's coverage changes toward the effect colour when compared with the same document without the effect, and every pixel outside the coverage is byte-identical

#### Scenario: An unmapped or declined legacy effect is a no-op

- **WHEN** a layer carries a legacy `bevl` whose decoded style is `Outer`, or a `sofi` whose version is not 2
- **THEN** no effect renders and the composite is byte-identical to the same document without the legacy block

#### Scenario: A disabled legacy effect is a no-op

- **WHEN** a layer's only legacy effect record has an enabled byte of 0, or the block's common state is not visible
- **THEN** the composite is byte-identical to the same document without the legacy block

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
