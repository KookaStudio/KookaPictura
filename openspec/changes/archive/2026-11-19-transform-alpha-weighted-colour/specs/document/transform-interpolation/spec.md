# Spec Delta

## ADDED Requirements

### Requirement: Transform colour resamples weighted by alpha

When a layer transform resamples a layer that has an alpha channel, each colour
channel (in the 8-bit channels and in the retained native store) SHALL be
sampled with every kernel tap weighted by that tap's alpha, so the colour of a
fully transparent pixel SHALL NOT contribute to any output pixel. Where every
tap with a non-zero kernel weight is fully opaque, the colour SHALL equal the
unweighted kernel's value. An output pixel whose weighted alpha sum is not
positive SHALL store colour `0`. The alpha channel and the layer mask SHALL be
resampled unweighted.

#### Scenario: Erased edges keep their colour after a scale

- **WHEN** a layer whose opaque yellow area borders erased pixels (alpha 0, colour black) is scaled 250% with `Bilinear` or `Bicubic`
- **THEN** every output pixel with non-zero alpha keeps the yellow colour, within one level

#### Scenario: The layer edge does not darken

- **WHEN** a fully opaque single-colour layer is rotated 30° with `Bilinear`
- **THEN** every anti-aliased edge pixel with non-zero alpha keeps the layer's colour

#### Scenario: Opaque content is unchanged

- **WHEN** a plane is sampled with all taps opaque, through the weighted sampler and the plain kernel, for every method
- **THEN** both return the same sample

#### Scenario: The native store is weighted too

- **WHEN** a 16-bit layer with erased pixels is scaled with `Bilinear`
- **THEN** every native colour sample with non-zero alpha keeps the opaque colour, and the 8-bit channel equals its narrowing

### Requirement: A transform resamples the current pixels, not a stale store

Before a layer transform or warp resamples a layer's retained native store,
every store plane whose narrowing to 8 bits differs from the 8-bit plane it
shadows (a colour channel, the alpha channel, or the layer mask) SHALL be
replaced by that 8-bit plane widened to the store's depth, so pixels edited
only in the 8-bit planes (a brush or eraser stroke) SHALL NOT be restored by
the transform. A store plane that still narrows to its 8-bit plane SHALL keep
its native samples. A store holding a converted source mode's planes SHALL be
resampled unchanged.

#### Scenario: An erased hole stays erased after a scale

- **WHEN** a 16-bit layer's 8-bit planes have a hole erased after its store was captured, and the layer is scaled 250% with `Bilinear`
- **THEN** the hole stays fully transparent, and every store plane narrows to its 8-bit channel

#### Scenario: An erased hole stays erased after a warp

- **WHEN** the same layer is warped onto a quad twice its size with `Bilinear`
- **THEN** the hole stays fully transparent, and every store plane narrows to its 8-bit channel
