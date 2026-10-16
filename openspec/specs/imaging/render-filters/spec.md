# render-filters Specification

## Purpose
Seeded deterministic Clouds, Difference Clouds, Fibers, and Lens Flare with dispatch and self-test coverage.

## Requirements

### Requirement: Clouds replaces RGB with a seeded two-color noise field

The system SHALL provide `Filter::Clouds { color_a: [u8; 3], color_b: [u8; 3], starker: bool, seed: u64 }`. It SHALL replace every pixel's RGB on the layer with a deterministic fractal noise field interpolated between `color_a` and `color_b`, driven by `ChaCha8Rng` seeded with `seed`, leaving alpha untouched. The `starker` variant SHALL produce a higher-contrast field than the non-starker variant for the same seed. It MUST NOT read the pre-existing pixel colors.

#### Scenario: Clouds replaces pixels deterministically

- **WHEN** `Clouds` is applied twice to the same buffer with the same seed and colors
- **THEN** both results are bit-identical, every RGB value lies within the per-channel range spanned by `color_a` and `color_b`, and alpha equals its pre-call value

#### Scenario: Starker variant differs in contrast

- **WHEN** `Clouds` runs with `starker: false` and `starker: true` at the same seed
- **THEN** the two outputs differ and the starker output has the higher per-channel standard deviation

### Requirement: Difference Clouds blends the noise field via Difference

The system SHALL provide `Filter::DifferenceClouds { color_a: [u8; 3], color_b: [u8; 3], starker: bool, seed: u64 }`. It SHALL generate the Clouds noise field for the same parameters and set each pixel's RGB to the per-channel absolute difference between the pre-existing color and the field color, leaving alpha untouched. Applying it repeatedly SHALL change the result (cumulative marble patterning).

#### Scenario: Difference formula holds per pixel

- **WHEN** `DifferenceClouds` is applied to a known buffer
- **THEN** every resulting RGB channel equals `|existing − field|` computed from the Clouds field for the same seed, and alpha is unchanged

#### Scenario: Repeated application keeps changing the image

- **WHEN** `DifferenceClouds` runs twice in sequence on a non-uniform buffer
- **THEN** the second result differs from the first

### Requirement: Fibers renders seeded directional fibers

The system SHALL provide `Filter::Fibers { variance: f64, strength: f64, color_a: [u8; 3], color_b: [u8; 3], seed: u64 }`. It SHALL replace the layer's RGB with deterministic streaks elongated along the vertical axis, blended from `color_b` (background) to `color_a` (foreground), leaving alpha untouched. The streaks SHALL layer smooth broad clumps under hard-edged hairs one or two pixels wide. `variance` SHALL shorten the streaks and shift the balance towards hairs, and `strength` SHALL lengthen them. A `variance` of 0 SHALL fill every pixel with the even blend of the two colours. Values of `variance` outside 0..=64 or `strength` outside 1..=64 SHALL be rejected with `FilterError::InvalidParams` before any mutation. The model is photorust's, tuned by eye against CS6, and is a behavioral approximation of a closed algorithm.

#### Scenario: Fibers run down the picture

- **WHEN** `Fibers` is applied at variance 12 and strength 4
- **THEN** the summed tone change between horizontal neighbours exceeds four times the summed change between vertical neighbours, and the output reaches within 16 levels of both colours

#### Scenario: Fibers parameters shape the field

- **WHEN** `Fibers` is applied at variance 8 and at variance 48 with the same seed and strength, and at strength 1 and strength 48 with the same seed and variance
- **THEN** each output is bit-identical across repeated runs at its own settings, all RGB values lie within the colour range, the variance-48 run changes more than twice as much down each column as the variance-8 run, and the strength-48 run changes less down a column than the strength-1 run

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `Fibers` is called with `variance` below 0 or above 64, or `strength` below 1 or above 64, or either is NaN
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

### Requirement: Lens Flare renders a deterministic additive light pass

The system SHALL provide `Filter::LensFlare { brightness: f64, center: (f64, f64), lens: LensType }` with `LensType` values `Zoom`, `Prime35`, `Prime105`, and `MoviePrime`. It SHALL add light onto the existing RGB: a bright core at `center` (fractions of the width and height, clamped into 0..=1), a glow and halo around it, rays (or, for `MoviePrime`, a horizontal streak), and tinted ghosts strung along the line from the flare through the middle of the frame. Results SHALL be clamped to 0..=255 and alpha SHALL be left untouched. Every size SHALL be a fraction of the image's half-diagonal, so the same parameters on a scaled copy produce a scaled copy of the flare. `brightness` outside 10..=300 SHALL be rejected with `FilterError::InvalidParams` before any mutation. The filter SHALL be fully deterministic (no seed). The model is photorust's, tuned by eye against CS6, and is a behavioral approximation of a closed algorithm.

#### Scenario: Brightness is monotone at the core

- **WHEN** `LensFlare` runs on a dark frame at centre (0.5, 0.5), and at brightness 25, 100, and 300 at centre (0.2, 0.2)
- **THEN** the pixel under the centre is blown out to 255 while an off-axis corner stays dark, and an off-axis sample rises strictly with brightness

#### Scenario: Lens type and center change the geometry

- **WHEN** `LensFlare` runs at centre (0.25, 0.25), near the left edge on black, and with each of the four `LensType` values
- **THEN** the placed centre is brighter than an off-axis point by more than 100 levels, the right half along the ghost axis carries more than twice the light of a row well off it, the four lens outputs all differ, each output is bit-identical across repeated runs, and an off-frame centre (−3, 5) renders identically to (0, 1)

#### Scenario: A flare is the same picture at any size

- **WHEN** the same `LensFlare` runs on a 600×400 frame and a 150×100 frame
- **THEN** outside blown-out blocks, each small-frame pixel is within 20 levels of the mean of the 4×4 large-frame block under it

#### Scenario: Out-of-range brightness is rejected untouched

- **WHEN** `LensFlare` is called with brightness below 10, above 300, or NaN
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

### Requirement: Dispatch integration and app kinds

The `Filter` enum SHALL carry the four new variants and `pictura_filters::apply` SHALL dispatch them. The app SHALL expose kinds `clouds`, `difference-clouds`, `fibers`, and `lens-flare` through `filter_from_kind` with fixed defaults (black/white colors, `starker: false`, seed 1, variance 16, strength 4, brightness 100, unit center, Zoom lens), and the Qt shell filter combo SHALL list them. `pictura_render::apply_filter` SHALL apply them through the existing layer path unchanged.

#### Scenario: App applies a Render filter through the layer path

- **WHEN** `apply_filter("clouds")` is invoked in the app with no selection
- **THEN** the topmost pixel layer's RGB changes within its rect, the rest of the composite is unchanged, and a second application with the same kind is bit-identical to the first result

### Requirement: Headless self-test coverage

The app `--self-test` SHALL apply at least one Render filter end-to-end: assert the command returns `true`, the composite changes only inside the target layer's rect, alpha outside the rect is unchanged, and a repeated application is bit-identical (seeded replacement semantics).

#### Scenario: Self-test proves a Render filter in the app

- **WHEN** the app runs `--self-test` with the layered fixture
- **THEN** the Render-filter assertions pass and the self-test exits successfully

### Requirement: Lighting Effects filter

The system SHALL implement `Filter::Lighting` over a CS6 light rig: shared Colorize colour, Ambience, Exposure, Gloss, Metallic (each −100..=100), a bump Texture channel (None/Red/Green/Blue), and Height (0..=100), plus 1..=16 lights. Each light SHALL carry a type (Spot, Point, Infinite), a visible flag, colour, Intensity and Hotspot (−100..=100), a unit centre, an angle, a size and width (fractions of the half-diagonal), and an elevation (0..=90°). A Spot SHALL light an ellipse with semi-axes size and width along its angle, at full strength inside a hotspot ellipse lying toward the aimed end and fading to nothing at the outer edge. A Point SHALL fall off with distance to its radius. An Infinite light SHALL light the whole frame evenly from its angle and elevation. Every visible light SHALL add an ambient-relative diffuse term and, with Gloss above zero, a Blinn highlight, over the layer colour and the selected bump channel's gradient. A hidden light SHALL contribute nothing. Intensity 50 SHALL leave a lit flat surface at about its own brightness, and negative intensity SHALL remove light. The default SHALL be CS6's Default style: one white Spot at the unit centre with intensity 35 and hotspot 69. Sizes and positions SHALL be frame-relative, so a scaled copy lights the same. Alpha SHALL be preserved, output SHALL be deterministic, and an out-of-range parameter or a rig of no lights or more than 16 SHALL be rejected as `FilterError::InvalidParams` without mutating the buffer. The model is photorust's shading tuned by eye against CS6, a behavioral approximation of a closed algorithm.

#### Scenario: Lighting relights the colour planes and preserves alpha

- **WHEN** the default rig lights an RGBA frame whose alpha is 90
- **THEN** the colour planes change and the alpha plane is bit-identical

#### Scenario: A spot lights its ellipse toward its aim

- **WHEN** the default rig lights an even grey frame, and a Spot aimed along +x is sampled ahead of and behind its centre
- **THEN** the frame's middle is lit, a far corner is black, and the aimed-at side is brighter than the side behind

#### Scenario: Each light type differs

- **WHEN** the same frame is lit by a Spot, a Point, and an Infinite light
- **THEN** the three outputs differ

#### Scenario: Ambience, intensity, and falloff behave as in CS6

- **WHEN** the same frame is lit by each light type, with ambience raised, and with intensity raised, lowered below zero, or exposure raised
- **THEN** an Infinite light is even across the frame, a Point falls off faster than a Spot's hotspot, ambience lifts unlit pixels, and intensity and exposure raise the light while negative intensity lowers it

#### Scenario: Lights add up and hidden lights do not

- **WHEN** a rig of two identical Spots is compared with one, and a rig whose second light is hidden is compared with the single light
- **THEN** the pair lights more brightly, and the hidden-light rig is bit-identical to the single light

#### Scenario: A texture shapes the light and the rig is size-invariant

- **WHEN** a glossy rig lights a striped bump channel, and the same two-light rig lights a 600×400 and a 150×100 frame
- **THEN** the bump spreads the lit values far more than a flat frame, and each small pixel is within 8 levels of its 4×4 large block

#### Scenario: Bad parameters are refused untouched

- **WHEN** the rig has no lights, 17 lights, an out-of-range or non-finite parameter, a zero size, or an elevation above 90
- **THEN** the operator returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state, while a rig of exactly 16 lights is accepted

### Requirement: Picture Frame renders a procedural decorative border

The system SHALL provide `Filter::PictureFrame { options: FrameOptions }`. `FrameOptions` SHALL carry a frame number (1–47), a vine colour, margin, size, arrangement, a flower index (0 = none, 1–22), a flower colour and size, a leaf index (0 = none, 1–23), a leaf colour and size, and the Advanced controls Number of Lines, Thickness, Angle, Fade, and Invert. The filter SHALL draw a decorative border over the layer: motif frames lay a vine, flowers, and leaves along a rounded-rectangle track, and the ruled and moulded frames draw lines and a bevelled band from a corner distance field. Everything SHALL be procedural geometry — no external or traced artwork. The frame SHALL be composited source-over, so it can add opacity where the layer was transparent, and the result SHALL be deterministic (the arrangement is a seed, not a random draw). Every size SHALL be a fraction of the layer's shorter side, so the same parameters on a scaled copy produce a scaled copy of the frame. Any parameter outside its documented range SHALL be rejected with `FilterError::InvalidParams` before any mutation.

#### Scenario: Every frame draws a border and spares the middle

- **WHEN** `PictureFrame` runs with each frame number 1–47 at CS6's defaults on a white buffer
- **THEN** each output changes more than 200 pixels from the input, and a central region of the picture is bit-identical to the input

#### Scenario: Colours, margins, and arrangements take effect

- **WHEN** the vine and flower colours are set, the margin is raised, and the arrangement is changed
- **THEN** the output contains pixels of the vine colour and of the flower colour, a larger margin moves the first ink further in from the edge, and two arrangements give different outputs while the same arrangement is bit-identical across runs

#### Scenario: Advanced controls change the frame

- **WHEN** Thickness is raised, Number of Lines is raised, Fade is applied, and Invert is set
- **THEN** a thicker run changes more pixels than a thin one, more lines pack more crossings along the top band, Fade thins the middle of a side while sparing its corner, and Invert turns a motif frame and a moulded frame inside out

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `PictureFrame` is called with a frame number outside 1–47, a flower index above 22, a leaf index above 23, or any other control outside its range
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

### Requirement: Post-CS6 render entries are surfaced by decision

The `Filter ▸ Render` menu SHALL list the CS6 entries (Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects) and SHALL additionally surface `Picture Frame`, a CC 2014.2 entry shipped as a deliberate post-CS6 addition to Kooka. `Flame` and `Tree` were added in Photoshop CC 2014.2 and remain out of scope; they MUST NOT be surfaced in the menu and SHALL remain unsurfaced unless the project's parity scope changes.

#### Scenario: Picture Frame is present and Flame and Tree are absent

- **WHEN** the `Filter ▸ Render` menu is built
- **THEN** it contains `Picture Frame` and does not contain `Flame` or `Tree`
