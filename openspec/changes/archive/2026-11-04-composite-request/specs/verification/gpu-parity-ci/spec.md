# Spec Delta

## MODIFIED Requirements

### Requirement: Fixture scenes run through the active backend

The ImageMagick fixture scenes, which cover the blend modes they map and the
partial-alpha scene, SHALL be composited through
`composite(CompositeRequest { doc, region: None, gpu_enabled: true })` in a
device-dependent test. Each result SHALL either report `Backend::Gpu` and lie
within ±1 LSB per channel of `composite_rgba(doc)`, or report `Backend::Cpu`
with bytes equal to `composite_rgba(doc)`. The reference SHALL be the CPU
oracle, never the ImageMagick output.

#### Scenario: A GPU-eligible scene matches the oracle

- **WHEN** a Multiply fixture scene is composited through the active backend on
  the software device
- **THEN** it reports `Backend::Gpu` and every channel is within 1 LSB of
  `composite_rgba`

#### Scenario: A declined scene equals the oracle

- **WHEN** a fixture scene changed to `Dissolve` is composited through the active
  backend
- **THEN** it reports `Backend::Cpu` and its bytes equal `composite_rgba`
