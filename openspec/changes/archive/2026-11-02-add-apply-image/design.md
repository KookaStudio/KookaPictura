# Design: add-apply-image

## Not an `Adjustment`

Adjustments see a position-free crop, but Apply Image samples a document-sized
source at each pixel, and it may change alpha. It gets its own `ActiveOp` arm and
kernel; the preview session and selection handling are shared.

## Source snapshots

When the source is the active document, a preview changes what `Merged` reads,
so re-reading on each preview would compound the blend. The dialog cancels the
preview, reads the source and mask once into opaque `ApplyImageSnapshot`s, and
re-reads only when a source or mask control changes. Two calls keep `&source`
and the pinned target from aliasing when they are the same view.

## Kernel

Work happens in the unit `f32` domain with `blend::blend` for layer modes:

```text
Add:      (A + B) / Scale + Offset
Subtract: (A - B) / Scale + Offset
```

A is the target and B the source. Results clamp to `[0, 1]` except on a 32-bit
target. `cov = source alpha × opacity × mask × selection`.

- Preserving (Preserve Transparency or a transparency lock): colour lerps by
  `cov`, alpha is unchanged, clear pixels are skipped.
- Otherwise: `αr = αb + cov − αb·cov`, and colour is W3C source-over with
  `Cs' = (1 − αb)·Cs + αb·B`.

A gray target (Grayscale layer, layer mask) reduces a colour source to
luminosity.

## Native depth

A layer whose store owns its colour planes is written in the store, and the
8-bit channels are re-narrowed. A partial store is refused, as in the native
fill. `PreviewBaseline::Section` saves 8-bit channels only, so native previews
take the whole-layer baseline, which shares planes until written.

Offset is `offset / 255` at every depth. Adobe doesn't state the scaling.

## Limits

`ponytail:` the write stays inside the layer rect, where Photoshop can grow the
layer. Dissolve is not listed because the kernel has no dither. Source colour
entries of a CMYK or Lab document read the RGB working data. The result is not
recorded for action replay.
