# Spec Delta

## ADDED Requirements

### Requirement: Embedded source preserves its merged transparency

When the renderer uses an embedded document's stored merged composite as the
smart-object source, it SHALL treat the first extra merged plane of that
composite as the transparency plane and carry it into the source alpha. The
source SHALL keep the stored merged composite's merged transparency plane: a
transparent margin SHALL render transparent, and a fully opaque first extra
plane SHALL render opaque. A composite with no extra merged plane SHALL render
opaque. A grayscale composite's first extra merged plane SHALL be honored the
same way as an RGB composite's. A depth-1 Bitmap composite's packed extra planes
SHALL NOT be treated as alpha.

#### Scenario: A transparent merged composite keeps its alpha [sor_merged_alpha]

- **WHEN** a proxy-less smart object's embedded payload is a document whose
  stored merged composite carries a transparency plane
- **THEN** the rendered source's alpha equals that plane, and the layer
  composites transparent where the plane is transparent

#### Scenario: A grayscale composite keeps its alpha [sor_merged_alpha_gray]

- **WHEN** the embedded payload is a grayscale document whose merged composite
  carries a transparency plane
- **THEN** the rendered source is gray with that plane as its alpha

#### Scenario: The first extra merged plane is the transparency [sor_merged_alpha_first_extra]

- **WHEN** the embedded payload's stored merged composite carries more than one
  extra plane, with the transparency first
- **THEN** the rendered source's alpha equals the first extra plane and ignores
  the later planes

#### Scenario: An opaque merged composite stays opaque [sor_merged_alpha_absent]

- **WHEN** the embedded payload's merged composite carries no extra merged plane
- **THEN** the rendered source is fully opaque across the layer rect
