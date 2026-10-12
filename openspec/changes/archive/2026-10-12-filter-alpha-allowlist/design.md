# Design: filter-alpha-allowlist

## Allowlist, then the fold (tasks 3.1/3.2 landed)

The real fix folds alpha into the working buffer so each kernel receives the
fourth plane and decides for itself. That is now the implemented path: the
call site in `apply_op_region` always folds an unlocked layer's alpha into a
four-plane buffer, moving kernels process every plane they are given (their
`.min(3)` caps dropped), value kernels keep the cap, the photorust path writes
the pixmap's alpha back only when `filters_alpha(filter)`, and the GPU sizes
its plane count from the same predicate. The grey rerun and the call-site
allowlist are gone; `filters_alpha` lives in `pictura-filters` next to the
kernels as the one classification.

The allowlist inversion (the original fix) fixed every reported kind first and
still describes which kernels filter alpha — the predicate IS that list, now
colocated with the kernels instead of applied as a second pass by the caller.
A kind added later leaves alpha untouched until someone adds it to
`filters_alpha`, so the failure mode stays safe.

A kind is on the list when the grey pass is exactly what CS6 shows for
transparency: the kernel treats every plane the same way regardless of value,
so the layer edge moves, smears, or erodes with the colour, and a flat plane
stays flat. Kinds that compare values against a threshold or cluster by
colour (Facet, Mezzotint, Diffuse's modes, Wind, Extrude, Tiles) stay off,
because the grey pass would partition alpha differently from colour.

## Offset's exposed area

`Filter::Offset` has `background: [u8; 3]` plus `transparent: bool`, and the
dialog carries both of CS6's undefined-area fills: `Background:` (the colour,
painted opaque) and `Set to Transparent` (alpha 0 in the exposed area). On a
normal layer the colour planes take `background` either way; only the folded
alpha plane's fill value differs (255 vs 0). Wrap Around moves alpha with the
colour as before.

## Tests

`pictura-render` `filter::tests`:

- `value_kernels_leave_alpha_bit_identical`: Solarize, Clouds, Difference
  Clouds, Fibers, Lens Flare, Trace Contour, Tiles, Find Edges, Emboss, High
  Pass, Add Noise, plus the three earlier exemptions, on an opaque and a
  half-transparent layer.
- `moving_kernels_keep_an_opaque_layer_opaque`: a sample of the allowlist,
  including Offset 2000/2000 without wrap and a black Background.
- `offset_moves_the_layer_edge_and_fills_what_it_exposes_opaque`.

`pictura-filters` pins the kernel-level contract: moving kernels run over the
fourth plane (blur/sharpen/morphology/distort scenario tests), value kernels
leave it bit-identical, and Offset's three fill modes (wrap, Background,
Transparent) are covered in `other::tests`.

Each of the original three fails against the previous predicate.
