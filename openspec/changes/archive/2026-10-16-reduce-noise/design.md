# Design: reduce-noise

## Model

The CS6 Help describes a spatial + channel denoise: luminance noise reduction
applied to all channels, an edge-detection threshold (Preserve Details) that
protects edges, a chroma pass (Reduce Color Noise), a post-denoise sharpen, and
JPEG-artifact removal. This is behavioral parity only; Adobe's kernel is closed.

- **Per-channel bilateral.** Each colour plane is smoothed by a 5×5 bilateral
  filter (fixed spatial σ = 1.6). Strength scales the range σ (0 at Strength 0,
  40 levels at 10); Preserve Details narrows it to a tenth at 100, so only
  near-identical samples average and edges survive. Strength 0 is a no-op.
- **Chroma.** The colour planes are blurred (σ up to 3 at Reduce Color Noise
  100) and recombined with the original luminance, so random colour speckle goes
  and luma detail stays.
- **Sharpen Details.** An unsharp pass on luminance (σ 1) whose delta is added
  back to every plane.
- **Remove JPEG Artifacts.** The two samples straddling each 8-pixel block seam
  are averaged when their step is below 40 levels, softening blockiness and
  halos without touching a real edge.
- Basic mode sends the same Strength/Preserve pair for all three channels;
  Advanced sends a pair per channel.

## Limits

The window is fixed and the constants are tuned by eye, not measured against
CS6. Alpha is never touched (a value kernel), unlike the rank filters of the
family. `FILT-030` marks the ranges and defaults unverified.
