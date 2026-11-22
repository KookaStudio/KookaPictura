# Design — crop resampling and Reveal All

## Resampling target

`CropOptions` gains a small value type:

```cpp
struct CropResampleTarget {
    int width = 0;
    int height = 0;
    double ppi = 72.0;
    bool perCm = false;
};
```

`width > 0 && height > 0` means the output-size mode is armed. The options bar
owns the lifecycle:

- Entering `W x H x Resolution` seeds the target from the current crop box and
  the document ppi (72 when the document has no resolution resource), then sets
  the crop ratio to `W / H`.
- Editing W or H sets the target and the ratio. It no longer resizes the box;
  the box is refitted to the new ratio by the existing `CropOptions::setRatio`
  notification, so the crop region may be larger or smaller than the output.
- Swap exchanges the target and the fields; Clear/Reset clears the target and
  the ratio.
- Switching away from the mode clears the target.

The commit reads the target from `CropOptions`. When it is armed, the tool calls
a resampled bridge instead of the plain crop bridge. Both straighten and plain
commits use it.

## Engine commit

The Rust bridge validates the target, then performs the whole commit on a
clone so a refusal leaves the document untouched:

1. `apply_crop` (existing) crops/grows and optionally deletes cropped pixels.
2. `resize_document(..., Resample::Bicubic)` runs only when the target differs
   from the cropped canvas.
3. `set_document_resolution` writes the requested ppi and unit when it differs.

The straighten variant rotates the clone first, then runs the same crop and
resample steps. Bicubic is the documented General-preference default; there is
no General interpolation preference to read, so the value is fixed and marked
as a ceiling in the code.

## Reveal All

`pictura_render::reveal_all` computes the union of:

- the current canvas `(0, 0, width, height)`, so Reveal All never shrinks;
- every non-group layer rect, recursively through groups;
- every layer mask rect;
- every retained native-store rect.

The union is the new canvas origin and size. `crop_document_grow` already
reframes a document with a possibly negative origin, shifts every layer and
mask, extends document channels, and rebases retained stores without
resampling. Pixels outside the old canvas were never discarded when `Delete
Cropped Pixels` was off, so they reappear unchanged. A destructive crop trimmed
them, so the union equals the canvas and the command is a no-op.

The app bridge records one `Reveal All` history state through the normal
`edit` path and clears the geometry selection because the canvas dimensions
change. The Image menu row is added to the Rust command catalogue so the C++
menu and the script catalogue share one source of truth.

## Why no stored crop region

Photoshop's exact non-destructive crop resource is not public. The retained
rasters already encode the reveal extent, survive PSD save because layer rects
may extend beyond the canvas, and match CS6's documented "Reveal All reveals
pixels that extend beyond the canvas" behaviour. A separate crop-region field
would duplicate that information and still need a private PSD key; it is left
out.
