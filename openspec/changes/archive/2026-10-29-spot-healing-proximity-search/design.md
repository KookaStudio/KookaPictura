# Design

## Context

See `proposal.md` for the motivation and the delta spec for the contract.
Adobe's Spot Healing algorithm is closed. The spec records only "search the
annulus around the brush footprint for the best-matching source patch, then run
the same heal solve", with the metric undocumented. Everything below is
behavioural parity, approximated.

## Decisions

### A margin past the brush edge

The fully covered pixels (coverage at or above 0.99) are the brush core.
The margin is `clamp(round(0.3 * thickness), 3, 8)` pixels, where `thickness`
is the deepest core pixel's distance to the core's edge, about the brush radius
for both a click and a drag. Pixels within the margin plus one of the core join the hole.
Their write-back weight is 1 across the margin, so a defect's rim inside it is
rebuilt completely, and falls to 0 over the next pixel. The fill already meets
its fixed boundary there, so the fade only softens the last step. (A fade over
the outer half of the margin was tried first: it left a few levels of a
dot-sized defect's rim.) A mask with no fully covered pixel (a very soft or partial
mask) gets no margin and keeps the previous coverage-weighted blend.

Alternative considered: grow the hole but write back only the brush footprint.
This was rejected because it leaves the defect's rim, which lies outside the
footprint, unrepaired, and that rim is the grey ring this change is fixing.

Distances use a two-pass chamfer transform (weights 1 and √2).

### Proximity Match source search

- **Context.** The known pixels within 4 px of the hole. **Interior.** The hole
  pixels. Each set is thinned to at most 1024 samples, deterministically
  (every k-th pixel).
- **Candidates.** Offsets within `2 * span + 10` of the hole, where `span` is
  the hole's larger bounding-box side. A candidate is valid only when the
  footprint it reads (the hole's bounding box grown by 5 px) lies inside the
  image and does not overlap the hole's bounding box. A coarse grid of at most
  48 × 48 offsets is scored first, then the ±stride neighbourhood of the best
  one at stride 1.
- **Score.** The mean squared RGBA difference over the context, plus 0.5 times
  the mean squared difference between the candidate's interior and the smooth
  (Laplace) fill. The interior term keeps another defect nearby (the next dot in
  the sky) from being chosen as a source. Ties go to the nearer offset, then to
  the first one in scan order, so the result is deterministic.
- **Solve.** The Healing Brush's Poisson solve (`clone_region`'s core,
  factored into `poisson_fill`) with the chosen offset, now over-relaxed
  (ω = 1.85) so a source lit a whole step differently converges within the
  existing 96 sweeps. This also speeds up the Healing Brush and Patch solves'
  convergence; their results move toward the exact solution. It carries the source's
  gradient and holds the destination as the boundary, so the lighting matches.
- **Fallback.** With no valid candidate (a hole too large for the image), the
  Laplace fill, as before.

The constants (0.3, 3..8, 4 px, 0.5, 48, ω) are inferred and marked `ponytail:`.

## Risks / Trade-offs

- A gradient-domain solve can only correct lighting smoothly. When the only
  clean source is lit differently on one side of a high-contrast edge (the
  next rows up a steep sky), a few levels of smooth shading remain next to the
  edge. The edge itself is carried through, not smeared.
- The minimum margin is 3 px, not 2: a click one pixel off a dot-sized defect
  leaves rim pixels about 2.4 px from the core on the diagonal.
- Pixels up to 8 px outside the brush footprint can change. That is the point of
  the fix, but it is a contract change. `healing_leaves_pixels_outside_the_brush_alone`
  is restated as "beyond the margin", and the returned rectangles grow by the
  margin.
- The search costs O(candidates × samples), which is bounded: at most about
  2.3k coarse offsets and 2k samples each. A large drag thins its samples.
