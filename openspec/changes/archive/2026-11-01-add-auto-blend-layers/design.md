# Design

All Auto-Blend analysis is an approximation: Adobe's focus metric, seam method
and tonal model are unpublished.

## Compute then commit

`plan_auto_blend` reads an `Arc`-backed document clone on a worker and returns an
`AutoBlendPlan`; `apply_auto_blend` installs it inside one `view.edit`. The
filter commit path moves `history` to the worker and targets one layer, so it is
not reused. Cancelling drops the plan, so no state is added. If
`content_revision` moved during the run, for example through Agent Control, the
plan is dropped and an error reported.

## Stack Images

`|Laplacian|` energy at source resolution, box-reduced to the work grid, with
energy near alpha edges discarded so layer edges do not score as sharp. The score
multiplies focus by a Mertens exposure weight, which handles focus and exposure
stacks with one rule. Ties go to the topmost layer.

## Panorama

A seeded watershed over disagreement, flooding the highest disagreement first
from pixels only one layer covers. Fronts cross disagreeing areas quickly and
meet in low-difference valleys. A label only enters pixels its layer covers.

## Masks

One mask per layer cannot express true multiband blending, so soft weights
combine Gaussian pyramids of one-hot labels by local band-energy share. Masks
composite with "over", so weights become stacked alphas bottom to top:
`mask_i = w_i / (alpha_i * sum_{j<=i} w_j)`.

## Seamless

Brown-Lowe per-channel gains on thumbnails, written as two-point Curves. Inserted
directly above the base and below any existing clipped run. Curves clamp to 0..1,
which loses nothing because the compositor already clamps float samples.

## Limits

Above `MAX_WORK_PIXELS` (16 MP) analysis uses an integer-reduced grid, so seams
and mask edges stop at that grid. Weights assume Normal mode at 100 % opacity.
`default_color = 0` also cuts layer effects outside the layer rect.
