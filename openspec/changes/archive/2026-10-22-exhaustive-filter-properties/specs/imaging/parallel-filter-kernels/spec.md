# Spec Delta

## MODIFIED Requirements

### Requirement: An opaque layer skips the alpha pass

When a moving kernel is applied to an unlocked layer whose alpha in the region
is fully opaque, the wrapper SHALL skip rerunning the kernel over the alpha. Every
such kernel maps a flat opaque plane to itself.

#### Scenario: Every moving kernel maps opaque to opaque

- **WHEN** each filter whose alpha fact is that its kernel moves runs over a flat 255 grey plane
- **THEN** the plane comes back flat 255
