# blend-modes Specification

## Purpose
All 27 Photoshop blend functions, the separable W3C formulas, the Photoshop-only extended modes, and Dissolve, checked against an oracle.

## Requirements

### Requirement: All 27 Photoshop blend functions

The compositor SHALL implement the 27 Photoshop layer blend modes — Normal, Dissolve, Darken, Multiply, Color Burn, Linear Burn, Darker Color, Lighten, Screen, Color Dodge, Linear Dodge (Add), Lighter Color, Overlay, Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix, Difference, Exclusion, Subtract, Divide, Hue, Saturation, Color, and Luminosity — one-to-one with the PSD 4-byte blend keys.

#### Scenario: Every mode produces its defined result

- **WHEN** each of the 27 modes is evaluated for base `Cb = 0.25` and source `Cs = 0.75`
- **THEN** the blend function MUST return that mode's W3C or community value within `1/255`

#### Scenario: Non-separable modes use the W3C triplet helpers

- **WHEN** Hue, Saturation, Color, or Luminosity is evaluated
- **THEN** the result MUST match the W3C `SetLum`/`SetSat`/`ClipColor` helpers applied to the RGB triplet within `1e-4`

### Requirement: Separable formulas follow W3C Compositing and Blending Level 1

Separable modes SHALL be computed per channel in `[0, 1]` using the W3C Level 1 formulas, including the defined guards: Color Dodge MUST return 0 for a black backdrop and 1 for a full source, Color Burn MUST return 1 for a full backdrop and 0 for a black source, Overlay MUST be Hard Light with swapped operands, and Difference MUST be `|Cb − Cs|`.

#### Scenario: Dodge and burn guards avoid division by zero

- **WHEN** `Cb = 0` or `Cs = 1` for Color Dodge, or `Cb = 1` or `Cs = 0` for Color Burn
- **THEN** the formula MUST return the guarded value without producing `inf` or `NaN`

#### Scenario: Overlay is Hard Light with swapped operands

- **WHEN** `Overlay(Cb, Cs)` is evaluated
- **THEN** it MUST equal `HardLight(Cs, Cb)`

#### Scenario: Difference is absolute

- **WHEN** `Difference(Cb, Cs)` is evaluated
- **THEN** the result MUST be `|Cb − Cs|` per channel

### Requirement: Photoshop-only extended modes

The Photoshop-only separable modes SHALL use the community definitions: Linear Dodge `min(1, Cb + Cs)`, Linear Burn `max(0, Cb + Cs − 1)`, Vivid Light the Color Burn/Color Dodge split at `Cs = 0.5`, Linear Light `clamp(Cb + 2·Cs − 1)`, Pin Light the Darken/Lighten split at `Cs = 0.5`, Hard Mix `(Cb + Cs) ≥ 1 ? 1 : 0`, Subtract `max(0, Cb − Cs)`, Divide `Cs == 0 ? 1 : min(1, Cb / Cs)`, and Darker Color / Lighter Color the summed-channel comparison.

#### Scenario: Divide and Subtract clamp safely

- **WHEN** Divide is evaluated with a black source (`Cs = 0`)
- **THEN** the result MUST be 1 and MUST NOT be `inf` or `NaN`
- **WHEN** Subtract would produce a negative value
- **THEN** the result MUST be 0

#### Scenario: Hard Mix is binary per channel

- **WHEN** Hard Mix is evaluated
- **THEN** every output channel MUST be exactly 0 or 1

#### Scenario: Darker and Lighter Color compare channel sums

- **WHEN** the summed channels of the backdrop and source are compared
- **THEN** Darker Color MUST return whichever input color has the smaller sum and Lighter Color the larger, never a per-channel mix

### Requirement: Dissolve is a deterministic binary threshold

Dissolve SHALL select, per pixel, either the unchanged backdrop or the fully opaque source color using a deterministic per-pixel noise value compared against the effective source alpha (mask × opacity). The result MUST be binary per pixel and stable across re-renders of the same document.

#### Scenario: Dissolve is binary and deterministic

- **WHEN** a Dissolve layer at 50% opacity is composited twice
- **THEN** the two buffers MUST be identical and every pixel MUST be either the source color at full alpha or fully transparent

#### Scenario: Dissolve coverage tracks opacity

- **WHEN** the effective source alpha is `a`
- **THEN** the fraction of the noise field that passes MUST approximate `a`

### Requirement: ImageMagick differential oracle for supported modes

The test suite SHALL diff `composite_rgba` against committed ImageMagick `-compose` fixtures for the 19 modes ImageMagick implements with the same formula, applying a per-mode tolerance of 0, except Vivid Light, which SHALL use a tolerance of 1 for rounding.

#### Scenario: Solid and ramp scenes match ImageMagick

- **WHEN** a two-layer solid or ramp scene is composited for a supported mode
- **THEN** every output sample MUST be within the mode's tolerance of the ImageMagick reference

#### Scenario: Partial-alpha source is checked with Normal only

- **WHEN** the source layer has varying alpha
- **THEN** only the Normal (Porter-Duff source-over) result MUST be diffed against ImageMagick

#### Scenario: Fixture generator is reproducible

- **WHEN** `scripts/im_compose.py check` regenerates the reference images
- **THEN** the bytes MUST match the committed fixtures, or the check MUST be skipped when `magick` is not on `PATH`

### Requirement: Unsupported-by-ImageMagick modes covered by hand-computed W3C tests

The eight modes for which ImageMagick has no like-for-like operator — Dissolve, Darker Color, Lighter Color, Soft Light, Hue, Saturation, Color, and Luminosity — SHALL NOT be validated against ImageMagick and MUST instead be validated by unit tests against W3C formulas and hand-computed values.

#### Scenario: Unsupported modes are excluded from the oracle mapping

- **WHEN** the oracle mode table is inspected
- **THEN** those eight modes MUST NOT map to an ImageMagick operator and the remaining 19 MUST each map to one

#### Scenario: Hand-computed values pin the unsupported modes

- **WHEN** the unsupported modes are evaluated on fixed color pairs
- **THEN** the results MUST match hand-computed W3C values within tolerance

### Requirement: The engine layer-mode list is the only blend-mode table

The engine SHALL publish its layer blend-mode list as ordered (PSD key, CS6 display
name, has-neutral-colour) rows: the 27 layer modes, Normal through Luminosity,
with Pass Through excluded. Every layer blend-mode control and label in the shell SHALL build from
that list. That covers the New
Layer dialog, the Layer Style blend combo, the Layers panel blend combo and Mode
filter, and the Properties panel's mode label. A control's selected index SHALL
be the engine list index. The shell SHALL hold no blend key, name or
neutral-colour table of its own, and the New Layer dialog's neutral-colour option
SHALL follow the engine's has-neutral flag.

#### Scenario: Engine list order and names [blend_list_engine]

- **WHEN** the engine layer-mode list is enumerated
- **THEN** it has 27 entries in layer-mode order, with each key round-tripping
  through the PSD key decoder and each name equal to the mode's display name

#### Scenario: Layer combos build from the engine list [blend_list_combos]

- **WHEN** the Layers panel blend combo and Mode filter, the New Layer dialog's
  Mode combo and the Layer Style blend combo are populated
- **THEN** each lists the engine names in engine order. The Layers panel, filter
  and New Layer combos carry the engine keys as item data, and the Layer Style
  combo stays index-only, its index being the engine index.

#### Scenario: Choosing a mode sets that engine mode [blend_list_set]

- **WHEN** the user picks the entry at index *i* in the Layers panel blend combo
- **THEN** the active layer's blend key is the engine list's key at index *i*
