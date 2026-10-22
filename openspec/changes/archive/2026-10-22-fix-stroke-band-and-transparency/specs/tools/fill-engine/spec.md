# Spec Delta

## ADDED Requirements

### Requirement: The fill engine says what it does with the pixels' transparency

The fill engine SHALL take an alpha policy with the caller, and SHALL apply
it to every pixel it fills:

- `Blend` — the source is laid over the backdrop, and the backdrop's alpha
  survives, which is what a brush-mode blend does.
- `Keep` — the destination's alpha survives unchanged, and the pixels the
  destination holds empty are left alone rather than filled.
- `Replace` — the source colour's alpha becomes the pixel's alpha, and the
  empty pixels are filled.

A layer's transparency lock SHALL outrank the caller's policy: a
transparency-locked layer SHALL keep every pixel's alpha, and SHALL leave its
empty pixels alone, whatever policy the caller asked for.

The policy SHALL be applied at the depth the fill is blended at, so that a
16- or 32-bit document's pixels take the same treatment an 8-bit document's
do.

#### Scenario: Keeping the destination's alpha [fe_alpha_keep]

- **WHEN** a fill with the `Keep` policy runs over a layer whose pixels are
  half transparent
- **THEN** the pixels keep their alpha, and the layer's empty pixels are not
  filled

#### Scenario: The source's alpha becomes the pixel's alpha [fe_alpha_replace]

- **WHEN** a fill with the `Replace` policy runs over a layer with a source
  colour whose alpha is zero
- **THEN** the pixels it reaches hold zero alpha, and their colour is the
  blend of the source and the backdrop

#### Scenario: A transparency lock outranks the policy [fe_alpha_lock]

- **WHEN** a fill with the `Replace` policy runs over a layer whose
  transparency is locked
- **THEN** the layer's pixels keep their alpha and its empty pixels are not
  filled
