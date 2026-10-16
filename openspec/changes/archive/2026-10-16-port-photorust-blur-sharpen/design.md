# Design: port-photorust-blur-sharpen

## Method

Per issue #222, each filter was routed temporarily through photorust's code
(radius mapped to σ by Kooka's 3σ convention) and run through the ImageMagick
differentials and `gpu_filter_parity`:

| Filter | photorust vs oracle | Decision |
|---|---|---|
| Gaussian Blur | max delta 1 (rounds to 8 bits between passes) | keep Kooka (exact) |
| Box Blur | max delta 1 (same) | keep Kooka (exact) |
| Median | max delta 16 (window clipped at the border) | keep Kooka (exact); take the histogram |
| Custom | max delta 0 | keep Kooka (see below) |
| Unsharp Mask | max delta 5, same as Kooka | keep Kooka (see below) |
| High Pass | no oracle | keep Kooka (see below) |
| Motion Blur | no oracle | keep Kooka (see below) |
| Blur, Blur More | no oracle before | photorust's model on Kooka's Gaussian |
| Sharpen, Sharpen More | no oracle before | photorust's model on Kooka's Unsharp Mask |
| Sharpen Edges | no oracle | photorust |
| Surface Blur | no oracle | photorust |

## Kept on Kooka's code

- **Custom:** exact on the opaque oracle image. On translucent pixels photorust
  convolves premultiplied 8-bit colour. The GPU kernel uploads colour only and
  cannot follow that within 1 LSB, and the spec fixes straight-colour f64
  accumulation. Its weights model is the same 5×5 grid, Scale, and Offset.
  photorust's CS6 default kernel has no consumer: the app does not map Custom.
- **Unsharp Mask:** photorust's version is no closer to ImageMagick (both max
  delta 5). Smart Sharpen's Gaussian path must stay byte-identical to
  Kooka's Unsharp Mask. photorust's 8-bit intermediate blur would also break
  the GPU combine's 1 LSB parity.
- **High Pass:** photorust's radius is σ; Kooka's is 3σ like Gaussian Blur.
  CS6 defines High Pass as the complement of Gaussian Blur at the same radius,
  so it stays on Gaussian Blur's convention.
- **Motion Blur:** photorust skips taps outside the image, where the spec
  requires clamp-to-edge. It also takes `distance + 1` taps at even
  distances. Kooka's matches the spec and its GPU plan.

## photorust's model, Kooka's arithmetic

Blur and Blur More are Gaussians at σ 0.7 and σ 2.0
(`BLUR_RADIUS = 2.1`, `BLUR_MORE_RADIUS = 6.0`). Sharpen and Sharpen More are
Unsharp Masks at 50 % and 100 %, σ 1 (`SHARPEN_RADIUS = 3.0`). This is
photorust's tuning. photorust's case: a 3×3 high-pass multiplies fine detail
about six times per pass, while CS6's Sharpen can be reapplied, each pass
adding a little.

They run on Kooka's Gaussian and Unsharp Mask, not photorust's, so:

- the GPU plans are the existing Gaussian and Unsharp plans; `plan.rs` reaches
  them through the same constants;
- all four gain ImageMagick differentials: Blur max delta 0
  (`-gaussian-blur 0x0.7`), Blur More 1, Sharpen 2 (`-unsharp 0x1+0.5`), and
  Sharpen More 3.

## Ported

- **Surface Blur:** photorust's per-channel mean of the neighbours within the
  threshold, on a sliding histogram. It is O(r) per pixel, where the bilateral
  it replaces was O(r²). Each channel decides independently which neighbours
  count. The window is clipped to the image, as in photorust and as
  Watercolor (which shares the function) expects. The spec's clamp-to-edge
  rule is modified for Surface Blur alone.
- **Sharpen Edges:** photorust's version. Port adaptation: the brightness is
  computed once per pixel instead of at each Sobel tap (same values).

## Median

Kooka's oracle-exact Median keeps its clamped window but replaces the
per-pixel sort with photorust's sliding histogram. A row's window holds each
clamped row and column as often as the sort's would, so the multiset (and
median) is identical. A unit test pins it to the sort.

## GPU

- **Blur, Blur More, Sharpen, Sharpen More:** the Gaussian (separable) and
  Unsharp (separable + combine) plans. The 3×3 `KERNEL` repeat loop and its
  kernel constants go; `KERNEL` serves Custom alone.
- **Surface Blur:** the shader sums the in-threshold neighbours per channel
  as integers and divides with truncation, exactly like the CPU (measured
  max delta 0). It scans the window, O(r²), where the CPU slides a histogram.
  At 1024² the GPU wins below radius 16 (r 4: 16 ms vs 45 ms) and loses above
  it (r 30: 146 ms vs 75 ms; r 100: 1.4 s vs 0.3 s). The plan therefore stops
  at radius 16 and wider radii fall back to the CPU.

## Performance

The shared Gaussian (`kernel::gaussian_blur_planes`) runs its row passes on
rayon. Each sum runs in kernel order, so the bytes are unchanged; the oracle
and GPU parity suites confirm it. This keeps Blur and Sharpen, now wider
than a 3×3 kernel, faster than before. See `proposal.md` for the profile.
