# Design: add-smart-blur

## Kernel

- **Selective average.** Each pixel is the Gaussian-weighted mean
  (σ = radius / 3, the `FILT-010` convention) of the taps in the disc of
  Radius whose R, G and B are all within Threshold levels of the centre. One
  joint test, rather than Surface Blur's per-channel one, keeps a pixel's
  channels from averaging different neighbours and fringing. The centre always
  passes, so the divisor is never zero. *(approximation)*
- **Threshold.** Read as levels on the 0–255 scale. Channel differences are
  whole levels, so the float threshold compares as its floor.
- **Quality.** Low / Medium / High step through the disc so each side carries
  at most 3, 6 or 12 taps (step `⌈⌈r⌉ / n⌉`). Up to that radius every quality
  samples every pixel, so Quality only matters for wide searches; at Radius 100
  High is ≤ 625 taps. The engine reuses `Quality` (Draft / Good / Best) for
  the three levels rather than adding a second three-step enum.
- **Edges.** A pixel is an edge when one of its 4-neighbours is more than
  Threshold away in some channel, so a step marks a two-pixel line, one pixel
  on each side. Edge Only writes 255 on edges and 0 elsewhere on all colour
  planes; Overlay Edge writes 255 on edges and the Normal result elsewhere.
  The edge width does not grow with Radius, matching CS6's output.

## Evaluation

The colour planes are copied once; rows run in parallel, each pixel
independent, so the result is order-independent and bit-stable (pinned by
`kernel_digests`). Measured on a 1024² RGB buffer: radius 50, High ≈ 160 ms
(release).

## Alpha and preview

- The colour pass never writes alpha. Normal mode joins
  `filter_preserves_opacity`'s spreading list; Edge Only and Overlay Edge
  write absolute values, so their alpha stays bit-identical.
- `preview_apron` is `⌈radius⌉ + 1` (the edge test reads one pixel further).
