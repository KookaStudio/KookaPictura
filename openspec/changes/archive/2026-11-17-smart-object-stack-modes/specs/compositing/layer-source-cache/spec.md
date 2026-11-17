# Spec Delta

## ADDED Requirements

### Requirement: A stack-mode change invalidates the cached source

Changing a smart object's active stack mode SHALL rebuild its embedded source
and the composited raster derived from it. The rebuilt pixels SHALL equal a
purge-then-composite. The embedded-source cache key SHALL include the active
mode, and the composited raster cache key SHALL include it as well.

#### Scenario: Switching the mode rebuilds the source [lsc_stack_invalidates]
- **WHEN** an object's stack mode changes from Median to Maximum and the document is composited again
- **THEN** the source is rebuilt from the payload and equals a purge-then-composite

#### Scenario: The composited raster is reused until the mode changes [lsc_stack_raster]
- **WHEN** an object with an active mode is composited twice, then its mode changes and it is composited again
- **THEN** the first two composites reuse the raster and the third rebuilds it
