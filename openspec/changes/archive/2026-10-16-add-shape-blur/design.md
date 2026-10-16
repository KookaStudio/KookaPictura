# Design: add-shape-blur

## Kernel

- **Shape.** `custom_shape_kernel(shape, 2r+1)` rasterizes the Custom Shape at
  its designed proportions, centred, its longer side spanning the square; the
  weights are its 0–255 antialiased coverage. CS6's kernel is closed; this is
  the arbitrary-kernel family the contract names. *(approximation)*
- **Orientation.** A true convolution: the kernel is rotated 180° before it is
  slid, so a lone bright pixel spreads into the shape as drawn (an arrow
  pointing right lights a right-pointing arrow). ImageMagick's
  `-morphology Convolve` rotates the same way; `Correlate` does not, and the
  oracle fails if the two are swapped.
- **Radius.** The kernel is `2r+1` px square, so Radius is its half-width.
  The dialog range 5–1000 and default 10 are inferred (the contract marks
  them unverified).

## Evaluation

Each kernel row splits into runs of equal weight (interior runs of 255, edge
pixels mostly one-pixel runs). With per-row prefix sums of the source plane,
a run costs one difference, so a pixel costs the kernel's run count (≈ its
height × edge crossings) instead of its area. Rows run in parallel; each
output row accumulates run by run across the whole row, the interior a plain
slice difference and only the ends paying for the clamp-to-edge repeat.
Integer sums keep the result exact (`(Σ w·v + Σw/2) / Σw`), so the rows are
order-independent and bit-stable. Measured on a 1024² RGB buffer: radius 46
star ≈ 100 ms (release).

The prefix sums are `u32` per plane (`h × (w+1)`), one plane at a time.

## Alpha and preview

- The colour pass never writes alpha. Shape Blur joins
  `filter_preserves_opacity`'s spreading list, so the alpha pass blurs the
  layer's transparency with the same kernel.
- `preview_apron` uses the radius (capped at 512 by the shared clamp), and the
  kind is local, so viewport previews stay exact up to that radius.
