# Design: add-adaptive-wide-angle

Adobe's projection and solver are closed; everything below is an
approximation built on the published method the filter is known to follow
(Carroll, Agrawala, Agarwala, "Optimizing Content-Preserving Projections for
Wide-Angle Images", SIGGRAPH 2009).

## Lens model

- **Focal length in pixels.** `F = focal_mm × crop / 43.27 × diagonal_px`.
  Only the product of Focal Length and Crop Factor matters.
- **Source → sphere.** With `d = p − centre`, `r = |d|`:
  Fisheye (equidistant) `θ = r / F`; Perspective (rectilinear)
  `θ = atan(r / F)`; Full Spherical (equirectangular, 2:1) longitude
  `(x / w − ½)·2π`, latitude `(½ − y / h)·π`.
- **Base projection.** Stereographic, `R = centre + 2F·(vx, vy) / (1 + vz)`:
  conformal (the book's "shape-conformal"), unit magnification at the centre.
  A fisheye direction past 90° is clamped to the 90° circle (radius 2F).
  This reproduces a CS6 capture: Fisheye, 11.74 mm, crop 0.20, Scale 50 %
  leaves a disc of radius ≈ 0.054 × diagonal. Full Spherical's base is the
  equirectangular image itself, so no constraints means no change.
- **Scale and offset.** The output is `centre + scale·(v − centre) + offset`.

## Constraints

- **Line.** Two source points; the traced line is the great circle between
  their directions, sampled about every half cell and mapped back to the
  source. That is the curve a straight scene edge makes under the lens model,
  which is what CS6's tool draws.
- **Polygon.** Three or more source points, closed. Each edge is a line
  constraint; coarse cells whose centre is inside take the gnomonic
  projection about the polygon's mean direction as their reference shape.
- **Orientation.** Unfixed (cyan), Horizontal (yellow), Vertical (magenta),
  Fixed angle (green); a constraint that cannot be computed (coincident or
  antipodal endpoints, fewer than two samples inside the layer, a
  Full Spherical arc across the seam, a polygon under three vertices) is
  invalid (red) and takes no part in the solve.

## Solver

- **Unknowns.** Output positions of a coarse grid, 48 cells across the
  longer side.
- **Energies.** Similarity per triangle (two per cell, all three vertex
  rotations), against the base projection's triangle (gnomonic inside
  polygons, weight 4); a weak anchor to the base projection (fixes the
  similarity gauge, so no constraints = the base projection exactly); per
  line sample `n·(P(s_k) − P(s_0)) = 0`, where `P(s) = R(s) + Σ w_i (v_i − R_i)`
  keeps the exact base curve inside a cell. Horizontal, Vertical and Fixed
  give `n`; Unfixed lines take `n` from the current solution, re-solved five
  times.
- **Linear algebra.** Normal equations assembled row by row into a sparse
  symmetric matrix (insertion-ordered rows, so sums are deterministic) and
  solved by Jacobi-preconditioned conjugate gradients, warm-started. No
  constraints: no solve.

## Rendering

- A fine grid (coarse cells subdivided to ≤ 16 px) takes
  `R(fine vertex) + bilinear(coarse v − R)`, so the exact base projection
  survives between coarse vertices. Its triangles are rasterized in output
  space; each covered pixel centre maps by barycentric weights to a source
  position, sampled bilinearly (alpha-weighted, edges clamped). The first
  triangle in grid order owns a pixel. Uncovered pixels are transparent, or
  white on a buffer with no alpha plane *(ponytail: CS6's Background-layer
  fill is unverified)*. The same rasterizer draws the dialog view at screen
  resolution; bands of rows run in parallel, each in fixed triangle order.
- `FilterKind::moves_alpha` lists Adaptive Wide Angle, so the shared fold
  resamples the alpha plane with the colour.

## App

- **Kind.** `adaptive-wide-angle`: Correction (0 Fisheye, 1 Perspective,
  2 Full Spherical), Scale %, Focal Length mm, Crop Factor, Offset X, Offset Y
  (fractions of the layer), and a constraint-set id (0 = none). Constraint
  points are stored as fractions of the layer, so Last Filter on another
  layer stretches them. Kept sets live for the process (eight most recent),
  as Liquify's meshes do.
- **Session.** `filter_tools/adaptive_wide_angle.rs` holds the layer's planes,
  the parameters, the constraints and the solved warp. Every edit re-solves.
- **Defaults.** Fisheye, Scale 100 %. With EXIF `FocalLength` the focal length
  is prefilled and Crop Factor is `FocalLengthIn35mmFilm / FocalLength`
  (1.0 when missing); a focal length past 15.31 mm is shown at 15.31 with the
  crop factor raised to keep the product. Without EXIF the product is a
  normal lens (43.27 mm): 15.31 mm × 2.83. As Shot is enabled only with EXIF.
  Ranges: Scale 50–150 %, Focal Length 1.00–15.31 mm, Crop Factor
  0.20–10.00 (read off CS6 captures; unverified). Full Spherical is disabled
  unless the layer is 2:1; Focal Length and Crop Factor are disabled in it.
- **Gating.** The menu row also needs an 8-bit RGB or Grayscale document.
- **Dialog.** Tool column (Constraint C, Polygon Y, Move M, Hand H, Zoom Z),
  a help line showing the hovered control's tip, the canvas with a zoom row
  and the Camera Model / Lens Model lines, and the right column: OK, Cancel,
  Correction, Scale, Focal Length, Crop Factor, As Shot, Detail (100 % loupe
  following the pointer), and Preview (Ctrl+P), Show Constraints (Ctrl+W),
  Show Mesh (Ctrl+E). A drag with Constraint adds a line (Shift: Horizontal
  or Vertical by the drag's dominant axis); clicks with Polygon add corners,
  a click on the first corner closes it, E removes the last corner, Esc
  drops it. Clicking a constraint selects it (handles on its points; drag a
  handle to move it), Delete removes it, right click offers Unfixed,
  Horizontal, Vertical, Arbitrary (fixed at its current angle) and Delete.
  With Preview off the canvas shows the uncorrected layer and constraints
  are drawn and edited there.
- **Ceilings** (`ponytail:`): no lens-profile database or Auto, no Panorama,
  no Smart Filter re-edit, no rotation handles or `S`-drag, no
  Save/Load Constraints, no GPU path, bilinear minification in the view.
