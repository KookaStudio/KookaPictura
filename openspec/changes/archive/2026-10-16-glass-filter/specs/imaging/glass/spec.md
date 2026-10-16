## ADDED Requirements

### Requirement: Glass refracts through a built-in surface

The system SHALL provide `Filter::Glass { distortion: u32, smoothness: u32, texture: GlassTexture, scaling: u32, invert: bool }`, where `GlassTexture` is one of `Blocks`, `Canvas`, `Frosted` or `TinyLens`. Each texture SHALL be a height map sized by `scaling` percent and blurred more as `smoothness` rises. Every destination pixel SHALL sample the source with bilinear interpolation and clamp-to-edge, displaced along the map's slope by an amount proportional to `distortion`. `invert` SHALL reverse the displacement. A `distortion` of 0 SHALL leave the buffer unchanged. The result SHALL be deterministic, and alpha SHALL be untouched. A `distortion` above 20, a `smoothness` outside 1..=15, or a `scaling` outside 50..=200 SHALL be rejected with `FilterError::InvalidParams` before any mutation. The surfaces and the model are tuned by eye against CS6 renders and are a behavioral approximation of a closed algorithm.

#### Scenario: Zero distortion is a no-op

- **WHEN** `Glass` is applied with distortion 0 and any texture
- **THEN** the buffer is bit-identical to its prior state

#### Scenario: Every surface bends and grows with Distortion

- **WHEN** `Glass` is applied to a horizontal ramp rising 4 levels per pixel with each texture at smoothness 3 and scaling 100, at distortion 3 and at distortion 15
- **THEN** the mean horizontal shift away from the clamped edges exceeds 0.3 px at distortion 3 and is more than twice as large at distortion 15

#### Scenario: Invert flips the bend

- **WHEN** `Glass` is applied twice with the same settings and `invert` off, and once with `invert` on
- **THEN** the two `invert`-off results are bit-identical, the `invert`-on result differs, and alpha is unchanged

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `Glass` is called with distortion 21, smoothness 0 or 16, or scaling 49 or 201
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

### Requirement: Glass is a Distort Filter Gallery entry

The app SHALL map the `glass` kind to `Filter::Glass` from five parameters: Distortion (0–20, default 5), Smoothness (1–15, default 3), Texture (Blocks, Canvas, Frosted, Tiny Lens; default Frosted), Scaling (50–200 %, default 100), and Invert (default off). The Filter Gallery SHALL list Glass second in its Distort category, after Diffuse Glow, and `Filter ▸ Distort ▸ Glass` SHALL open its dialog.

#### Scenario: Defaults and mapping

- **WHEN** the `glass` kind is resolved with no parameters, and with `[19, 15, 3, 162, 1]`
- **THEN** it yields distortion 5, smoothness 3, Frosted, scaling 100, invert off; and distortion 19, smoothness 15, Tiny Lens, scaling 162, invert on

#### Scenario: The gallery lists Glass under Distort

- **WHEN** the Filter Gallery builds its categories
- **THEN** the Distort category holds three entries, and the second is the `glass` kind
