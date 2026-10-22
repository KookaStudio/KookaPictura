# compositing/vector-mask-antialiasing Specification

## Purpose
Antialiased vector-mask coverage: an edge pixel of a vector mask or shape layer
shows the share of it the path covers, so slanted and curved edges are smooth.

## Requirements

### Requirement: Antialiased vector-mask coverage

The compositor SHALL sample a vector mask's coverage of a pixel as the share
of the pixel its closed subpaths cover, under the mask's even-odd or non-zero
fill rule, measured along 16 horizontal sample lines per pixel row with the
covered length exact along each line, scaled to 0–255 and rounded to nearest.
A pixel wholly inside SHALL be 255 and one wholly outside 0. The coverage SHALL
be cached per pixel row and bounded to the first query's clip plus one row each
side; a row outside the cache SHALL be computed from the edges, so the clip
bounds only the cache.

#### Scenario: Whole and partial pixels [vma_partial]

- **WHEN** a 2×2 square on whole pixels, a strip from x = 1.5, and a triangle under the diagonal y = x are sampled
- **THEN** whole pixels are 255 or 0, the half-covered strip pixel is 128, and each pixel the diagonal crosses is 128, inside the cached rows or outside them

#### Scenario: A crafted path is bounded by the clip [vma_bound]

- **WHEN** a mask whose points span `i32::MIN` to `i32::MAX` is sampled over a 256-row clip
- **THEN** its smooth coverage caches 258 rows, and an uncached row inside the path is still 255

#### Scenario: A rotated shape has smooth edges [vma_rotated]

- **WHEN** a red rectangle shape over white is rotated 30° with Free Transform
- **THEN** the composite has blended pixels along the slanted edges, not only pure red and pure white
