## MODIFIED Requirements

### Requirement: AHD demosaic follows edges on Bayer patterns

`DemosaicMethod::Ahd` SHALL implement adaptive homogeneity-directed
demosaicing (Hirakawa & Parks, 2005) for 2×2 Bayer patterns: green is
interpolated horizontally and vertically, red and blue follow from colour
differences, and each pixel takes the direction more homogeneous in CIELab over
its neighbourhood. A 6×6 X-Trans pattern SHALL take the X-Trans method. Any
other pattern, or a Bayer image smaller than 8 × 8, SHALL produce the
bilinear result.

#### Scenario: Less false colour at an edge [demosaic_edge]

- **WHEN** grey images with a sharp vertical edge and a sharp horizontal edge are mosaiced and demosaiced with each method
- **THEN** AHD's largest channel difference from the grey value near the edge is under a quarter of bilinear's

#### Scenario: Fallback to bilinear [demosaic_fallback]

- **WHEN** a 3×3 red-green-blue plane and 1×1, 3×2 and 7×7 Bayer planes are demosaiced with AHD
- **THEN** each equals the bilinear result and none panics

## ADDED Requirements

### Requirement: X-Trans patterns demosaic along edges

`DemosaicMethod::Ahd` on a 6×6 pattern in which every 5 × 5 window holds red,
green and blue and every red or blue photosite has a green within three
photosites along its row or column SHALL use Pictura's own edge-directed
method: green at a red or blue photosite SHALL blend linear interpolations
between the nearest greens along the row and along the column, each weighted
by `1 / (1 + a)²` with `a` the mean absolute difference of same-colour
photosites one or two apart along that direction near the pixel; red and blue
SHALL be the pixel's green plus a mean of the colour differences (sample
minus green) of that colour's photosites in the 5 × 5 window, weighted down
by distance and by green difference, widened at the image border until the
colour appears. Each photosite SHALL keep its own sample. The method is not a
published one; parity with other converters is approximate.

#### Scenario: A ramp is reproduced [xtrans_gradient]

- **WHEN** grey ramps rising left to right and top to bottom are mosaiced through an X-Trans pattern and demosaiced with AHD
- **THEN** every channel is non-decreasing along the ramp and equals the ramp within 1 everywhere

#### Scenario: Less false colour at an X-Trans edge [xtrans_edge]

- **WHEN** grey images with a sharp vertical edge and a sharp horizontal edge are mosaiced through an X-Trans pattern and demosaiced with each method
- **THEN** AHD's largest channel difference from the grey value near the edge is under a quarter of bilinear's

#### Scenario: Patterns that are not X-Trans keep bilinear [xtrans_applies]

- **WHEN** a 6×6 pattern whose first four rows are only green (so some 5 × 5 window lacks red) is demosaiced with AHD
- **THEN** it equals the bilinear result
