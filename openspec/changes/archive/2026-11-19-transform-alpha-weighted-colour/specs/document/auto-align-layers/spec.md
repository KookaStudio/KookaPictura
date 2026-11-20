# Spec Delta

## ADDED Requirements

### Requirement: Auto-Align matches the rendered coverage

Auto-Align's matcher SHALL read each layer's luma premultiplied by its alpha
(the layer as it renders over black), so a partly transparent anti-aliased
edge SHALL ramp to zero rather than step from the full colour to zero. A pixel
SHALL count as covered for corner detection when its alpha is above zero.

#### Scenario: A half-covered pixel reads at half luma

- **WHEN** a layer row holds a grey pixel at alpha 255, the same grey at alpha 128, and a pixel at alpha 0
- **THEN** the work image reads the full luma, the luma scaled by 128/255, and zero, and only the alpha-0 pixel is uncovered

#### Scenario: A perspective-transformed layer still aligns

- **WHEN** a copy of the reference layer is Free Transformed onto a trapezoid (its slanted edges anti-aliased with their colour kept) and aligned with Perspective
- **THEN** the plan recovers a homography mapping the trapezoid back onto the reference
