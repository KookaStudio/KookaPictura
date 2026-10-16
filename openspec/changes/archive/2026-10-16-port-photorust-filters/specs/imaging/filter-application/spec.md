## ADDED Requirements

### Requirement: Ported photorust filter engine

The system SHALL run the Artistic, Brush Strokes, Sketch, and Texture filters, and the Add Noise, Dust & Scratches, Average, Emboss, Find Edges, Diffuse, Glowing Edges, Extrude, Tiles, Trace Contour, Wind, Mosaic, Crystallize, Pointillize, Facet, Fragment, Mezzotint, Color Halftone, Twirl, Pinch, Spherize, Ripple, Wave, Polar Coordinates, and ZigZag filters through the engine ported from photorust (GPL-3.0). `apply` SHALL validate the buffer and each parameter against its capability's ranges before converting to the engine's interleaved image, SHALL write back only the colour planes so alpha is never modified, and SHALL fold each variant's `seed` into the engine's noise so the same seed reproduces a result bit for bit and different seeds re-roll it. The engine SHALL be deterministic regardless of thread count. Solarize SHALL stay on its ImageMagick-exact implementation.

#### Scenario: Ported filters keep the shared contract

- **WHEN** each ported filter is applied to RGB and RGBA buffers, twice, with an out-of-range parameter, and to 1×1, 1×N, and N×1 buffers
- **THEN** it succeeds with alpha unchanged, repeats bit for bit, rejects the bad parameter without writing a sample, and never panics

#### Scenario: Seeds re-roll the stochastic filters

- **WHEN** a ported filter that takes a `seed` is applied with two different seeds
- **THEN** the two outputs differ

