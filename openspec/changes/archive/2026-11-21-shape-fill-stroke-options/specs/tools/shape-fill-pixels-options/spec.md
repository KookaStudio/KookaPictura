# tools/shape-fill-pixels-options Specification

## Purpose
The Fill Pixels mode's Mode, Opacity, and Anti-aliased controls.

## ADDED Requirements

### Requirement: Fill Pixels options

In Fill Pixels mode the shape options bar SHALL show Mode, Opacity, and
Anti-aliased. Mode SHALL select the blend used to paint the outline, Opacity
SHALL scale the painted coverage, and Anti-aliased off SHALL produce hard
0/255 coverage. The controls SHALL be hidden or disabled outside Pixels mode.

#### Scenario: Opacity and anti-alias affect the painted outline

- **WHEN** a rectangle is filled in Pixels mode at 50 % Opacity with Anti-aliased off on a white layer
- **THEN** covered pixels are a hard blend of the foreground at half strength and the edge has no fractional coverage

#### Scenario: Mode affects the painted outline

- **WHEN** a rectangle is filled in Pixels mode with the Multiply mode over a coloured layer
- **THEN** the covered pixels are multiplied rather than replaced
