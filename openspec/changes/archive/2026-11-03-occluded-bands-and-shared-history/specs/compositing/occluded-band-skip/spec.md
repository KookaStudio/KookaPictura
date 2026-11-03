## ADDED Requirements

### Requirement: A band skips what an opaque layer hides

In an 8-bit document with no knockout, a band of the CPU banded composite SHALL
start at the topmost Normal layer that is opaque over the whole band (a pixel
layer with no effects, clipping, smart object or native store, full opacity and
fill, no type curve, Blend If or mask, and alpha 255 at every sample of the band)
when the bottom shown layer is likewise opaque over the band and every shown
layer between them is a pixel layer with no effects, clipping or smart object.
When no layer is shown above that layer, the band SHALL be that layer's colour
bytes with alpha 255. The composite MUST stay byte-identical to compositing every
layer.

#### Scenario: A band starts at the opaque layer over it [obs_start]

- **WHEN** an opaque bottom carries a translucent Multiply, an opaque Normal
  layer over some bands with one clear pixel in another, and a translucent Screen
  with a layer clipped to it; and the same with a bottom that has clear pixels
- **THEN** the bands the Normal layer fully covers start at it, the band with the
  clear pixel and the uncovered bands start at the bottom, and nothing is skipped
  over the clear-bottom stack

#### Scenario: A skipped or copied band equals the whole stack [obs_exact]

- **WHEN** those stacks, and the same with every layer above the opaque one
  hidden, are composited in bands and band by band on one canvas each
- **THEN** both are byte-identical, the hidden-above stack copies the opaque
  layer's bands, and `to_u8(k / 255)` is `k` for every byte
